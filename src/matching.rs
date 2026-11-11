use crate::auth::AuthUser;
use crate::db::Db;
use crate::{api_error, models::{
    DBListingStatus, DBListingType, DBMatchStatus, DBNotificationType, DBReturnStatus, Listing,
    Match, NewListingStatusHistory, NewMatch, NewNotification, NewReturn,
}, schema::{listing_status_history, listings, matches, notifications, returns}};
use bigdecimal::BigDecimal;
use campuslost_shared::{ApiError, MatchAction, MatchActionRequest, MatchActionResponse, MatchContactDto, MatchContactsResponse, MatchDto, MatchStatus};
use chrono::Utc;
use diesel::dsl::sql;
use diesel::sql_types::{Bool, Text};
use diesel::{prelude::*, PgConnection};
use rocket::{get, http::Status, post, response::status, serde::json::Json};
use serde_json::json;
use std::collections::HashSet;
use crate::models::User;
use crate::schema::users;

const MINIMUM_SCORE: i32 = 40;
const MAX_SUGGESTIONS: usize = 5;

fn calculate_score(first: &Listing, second: &Listing) -> i32 {
    // Category equality is mandatory and contributes 40 points.
    let mut score = 40;

    if first.building_id == second.building_id {
        score += 30;
    }

    let days_apart = first
        .event_date
        .signed_duration_since(second.event_date)
        .num_days()
        .abs();

    if days_apart <= 3 {
        score += 20;
    }

    let first_title = first.title.to_lowercase();
    let second_title = second.title.to_lowercase();

    if first_title.contains(&second_title) || second_title.contains(&first_title) {
        score += 10;
    }

    score
}

/// Generates suggestions for a listing that has just become active.
pub(crate) fn generate_matches_for_listing(
    conn: &mut PgConnection,
    approved_listing: &Listing,
) -> Result<usize, diesel::result::Error> {
    if approved_listing.status != DBListingStatus::Active {
        return Ok(0);
    }

    let opposite_type = match approved_listing.type_ {
        DBListingType::Lost => DBListingType::Found,
        DBListingType::Found => DBListingType::Lost,
    };

    let search_text = format!(
        "{} {} {}",
        approved_listing.title,
        approved_listing.description,
        approved_listing.location_detail.as_deref().unwrap_or("")
    );

    let candidates = listings::table
        .filter(listings::status.eq(DBListingStatus::Active))
        .filter(listings::type_.eq(opposite_type))
        .filter(listings::category_id.eq(approved_listing.category_id))
        .filter(
            sql::<Bool>(
                "listings.search_vector @@ to_tsquery(
                'simple',
                array_to_string(
                    tsvector_to_array(
                        to_tsvector('simple', ",
            )
            .bind::<Text, _>(&search_text)
            .sql(
                ")), ' | ')
            )",
            ),
        )
        .select(Listing::as_select())
        .load::<Listing>(conn)?;

    // Find every match pair that already involves this listing.
    let existing_pairs: HashSet<(uuid::Uuid, uuid::Uuid)> = matches::table
        .filter(
            matches::lost_listing_id
                .eq(approved_listing.id)
                .or(matches::found_listing_id.eq(approved_listing.id)),
        )
        .select((matches::lost_listing_id, matches::found_listing_id))
        .load::<(uuid::Uuid, uuid::Uuid)>(conn)?
        .into_iter()
        .collect();

    let mut scored_candidates: Vec<(Listing, i32)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let (lost_id, found_id) = match approved_listing.type_ {
                DBListingType::Lost => (approved_listing.id, candidate.id),
                DBListingType::Found => (candidate.id, approved_listing.id),
            };

            if existing_pairs.contains(&(lost_id, found_id)) {
                return None;
            }

            let score = calculate_score(approved_listing, &candidate);

            if score < MINIMUM_SCORE {
                return None;
            }

            Some((candidate, score))
        })
        .collect();

    // Highest score first
    scored_candidates.sort_by(|(first, first_score), (second, second_score)| {
        second_score
            .cmp(first_score)
            .then_with(|| first.id.cmp(&second.id))
    });

    scored_candidates.truncate(MAX_SUGGESTIONS);

    let mut created_count = 0;

    for (candidate, score) in scored_candidates {
        let (lost_id, found_id) = match approved_listing.type_ {
            DBListingType::Lost => (approved_listing.id, candidate.id),
            DBListingType::Found => (candidate.id, approved_listing.id),
        };

        let new_match = NewMatch {
            lost_listing_id: lost_id,
            found_listing_id: found_id,
            score: BigDecimal::from(score),
            status: Some(DBMatchStatus::Suggested),
            confirmed_by: None,
            confirmed_at: None,
        };

        let created_match = diesel::insert_into(matches::table)
            .values(&new_match)
            .returning(Match::as_returning())
            .get_result::<Match>(conn)?;

        // Notify the owner of the newly approved listing.
        let first_notification = NewNotification {
            user_id: approved_listing.user_id,
            type_: DBNotificationType::MatchFound,
            payload: Some(json!({
                "match_id": created_match.id,
                "listing_id": approved_listing.id,
                "counterpart_listing_id": candidate.id,
                "score": score,
            })),
            is_read: Some(false),
        };

        diesel::insert_into(notifications::table)
            .values(&first_notification)
            .execute(conn)?;

        // Notify the owner of the existing counterpart listing too.
        if candidate.user_id != approved_listing.user_id {
            let second_notification = NewNotification {
                user_id: candidate.user_id,
                type_: DBNotificationType::MatchFound,
                payload: Some(json!({
                    "match_id": created_match.id,
                    "listing_id": candidate.id,
                    "counterpart_listing_id": approved_listing.id,
                    "score": score,
                })),
                is_read: Some(false),
            };

            diesel::insert_into(notifications::table)
                .values(&second_notification)
                .execute(conn)?;
        }

        created_count += 1;
    }

    Ok(created_count)
}


#[get("/listings/<listing_id>/matches")]
pub async fn get_listing_matches(
    listing_id: &str,
    auth: AuthUser,
    db: Db,
) -> Result<Json<Vec<MatchDto>>, status::Custom<Json<ApiError>>> {
    let listing_id = match uuid::Uuid::parse_str(listing_id) {
        Ok(id) => id,
        Err(_) => {
            return Err(api_error(
                Status::BadRequest,
                "invalid_listing_id",
                "Listing ID must be a valid UUID",
            ));
        }
    };

    let listing_result = db
        .run(move |conn| {
            listings::table
                .filter(listings::id.eq(listing_id))
                .select(Listing::as_select())
                .first::<Listing>(conn)
        })
        .await;

    let listing = match listing_result {
        Ok(listing) => listing,
        Err(diesel::result::Error::NotFound) => {
            return Err(api_error(
                Status::NotFound,
                "listing_not_found",
                "Listing was not found",
            ));
        }
        Err(_) => {
            return Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not retrieve listing",
            ));
        }
    };

    if listing.user_id != auth.user.id {
        return Err(api_error(
            Status::Forbidden,
            "forbidden",
            "You can only view matches for your own listings",
        ));
    }

    // Retrieve suggestions involving this listing.
    let result = db
        .run(move |conn| {
            matches::table
                .filter(
                    matches::lost_listing_id
                        .eq(listing_id)
                        .or(matches::found_listing_id.eq(listing_id)),
                )
                .filter(matches::status.eq(DBMatchStatus::Suggested))
                .order(matches::score.desc())
                .select(Match::as_select())
                .load::<Match>(conn)
        })
        .await;

    match result {
        Ok(found_matches) => {
            let response = found_matches
                .into_iter()
                .map(|item| MatchDto {
                    id: item.id,
                    lost_listing_id: item.lost_listing_id,
                    found_listing_id: item.found_listing_id,
                    score: item.score.to_string().parse::<f64>().unwrap_or(0.0),
                    status: MatchStatus::Suggested,
                    confirmed_by: item.confirmed_by,
                    confirmed_at: item.confirmed_at,
                    created_at: item.created_at,
                })
                .collect();

            Ok(Json(response))
        }
        Err(_) => Err(api_error(
            Status::InternalServerError,
            "database_error",
            "Could not retrieve match suggestions",
        )),
    }
}


enum MatchActionFailure {
    NotFound,
    Forbidden,
    Conflict,
    Database(diesel::result::Error),
}

impl From<diesel::result::Error> for MatchActionFailure {
    fn from(error: diesel::result::Error) -> Self {
        Self::Database(error)
    }
}

fn match_to_dto(record: Match) -> MatchDto {
    let status = match record.status {
        DBMatchStatus::Suggested => MatchStatus::Suggested,
        DBMatchStatus::Confirmed => MatchStatus::Confirmed,
        DBMatchStatus::Rejected => MatchStatus::Rejected,
    };

    MatchDto {
        id: record.id,
        lost_listing_id: record.lost_listing_id,
        found_listing_id: record.found_listing_id,
        score: record.score.to_string().parse::<f64>().unwrap_or(0.0),
        status,
        confirmed_by: record.confirmed_by,
        confirmed_at: record.confirmed_at,
        created_at: record.created_at,
    }
}

#[post("/matches/<match_id>/action", format = "json", data = "<payload>")]
pub async fn action_on_match(
    match_id: &str,
    auth: AuthUser,
    db: Db,
    payload: Json<MatchActionRequest>,
) -> Result<Json<MatchActionResponse>, status::Custom<Json<ApiError>>> {
    let match_id = match uuid::Uuid::parse_str(match_id) {
        Ok(id) => id,
        Err(_) => {
            return Err(api_error(
                Status::BadRequest,
                "invalid_match_id",
                "Match ID must be a valid UUID",
            ));
        }
    };

    let user_id = auth.user.id;
    let action = payload.into_inner().action;

    let result = db
        .run(move |conn| {
            conn.transaction::<(Match, Option<uuid::Uuid>), MatchActionFailure, _>(|conn| {

                let match_record = matches::table
                    .find(match_id)
                    .for_update()
                    .select(Match::as_select())
                    .first::<Match>(conn)
                    .optional()?
                    .ok_or(MatchActionFailure::NotFound)?;

                if match_record.status != DBMatchStatus::Suggested {
                    return Err(MatchActionFailure::Conflict);
                }

                // Lock both listings and verify their current state.
                let lost_listing = listings::table
                    .find(match_record.lost_listing_id)
                    .for_update()
                    .select(Listing::as_select())
                    .first::<Listing>(conn)?;

                let found_listing = listings::table
                    .find(match_record.found_listing_id)
                    .for_update()
                    .select(Listing::as_select())
                    .first::<Listing>(conn)?;

                // Only the owner of the LOST listing can act on this match.
                if lost_listing.user_id != user_id {
                    return Err(MatchActionFailure::Forbidden);
                }

                // Both listings must still be active.
                if lost_listing.status != DBListingStatus::Active
                    || found_listing.status != DBListingStatus::Active
                {
                    return Err(MatchActionFailure::Conflict);
                }

                match action {
                    MatchAction::Reject => {
                        // Reject only the suggestion. Listings remain active.
                        let updated_match = diesel::update(
                            matches::table
                                .filter(matches::id.eq(match_id))
                                .filter(matches::status.eq(DBMatchStatus::Suggested)),
                        )
                        .set(matches::status.eq(DBMatchStatus::Rejected))
                        .returning(Match::as_returning())
                        .get_result::<Match>(conn)?;

                        Ok((updated_match, None))
                    }

                    MatchAction::Confirm => {
                        let now = Utc::now();

                        // Confirm the match.
                        let updated_match = diesel::update(
                            matches::table
                                .filter(matches::id.eq(match_id))
                                .filter(matches::status.eq(DBMatchStatus::Suggested)),
                        )
                        .set((
                            matches::status.eq(DBMatchStatus::Confirmed),
                            matches::confirmed_by.eq(Some(user_id)),
                            matches::confirmed_at.eq(Some(now)),
                        ))
                        .returning(Match::as_returning())
                        .get_result::<Match>(conn)?;

                        // Move both listings from active to matched.
                        for listing_id in
                            [match_record.lost_listing_id, match_record.found_listing_id]
                        {
                            let changed = diesel::update(
                                listings::table
                                    .filter(listings::id.eq(listing_id))
                                    .filter(listings::status.eq(DBListingStatus::Active)),
                            )
                            .set((
                                listings::status.eq(DBListingStatus::Matched),
                                listings::updated_at.eq(now),
                            ))
                            .execute(conn)?;

                            if changed != 1 {
                                return Err(MatchActionFailure::Conflict);
                            }

                            diesel::insert_into(listing_status_history::table)
                                .values(&NewListingStatusHistory {
                                    listing_id,
                                    old_status: Some(DBListingStatus::Active),
                                    new_status: DBListingStatus::Matched,
                                    changed_by: Some(user_id),
                                    note: Some(
                                        "Listing matched after match confirmation".to_string(),
                                    ),
                                })
                                .execute(conn)?;
                        }

                        // Create the pending return record.
                        let return_id = diesel::insert_into(returns::table)
                            .values(&NewReturn {
                                match_id,
                                loser_confirmed: Some(false),
                                loser_confirmed_at: None,
                                finder_confirmed: Some(false),
                                finder_confirmed_at: None,
                                final_status: Some(DBReturnStatus::Pending),
                                returned_at: None,
                            })
                            .returning(returns::id)
                            .get_result::<uuid::Uuid>(conn)?;

                        // Notify both participants. Contact information is
                        // deliberately not included in notification payloads.
                        for participant_id in [lost_listing.user_id, found_listing.user_id] {
                            diesel::insert_into(notifications::table)
                                .values(&NewNotification {
                                    user_id: participant_id,
                                    type_: DBNotificationType::MatchConfirmed,
                                    payload: Some(json!({
                                        "match_id": match_id,
                                        "return_id": return_id,
                                    })),
                                    is_read: Some(false),
                                })
                                .execute(conn)?;
                        }

                        Ok((updated_match, Some(return_id)))
                    }
                }
            })
        })
        .await;

    match result {
        Ok((record, return_id)) => Ok(Json(MatchActionResponse {
            match_data: match_to_dto(record),
            return_id,
        })),

        Err(MatchActionFailure::NotFound) => Err(api_error(
            Status::NotFound,
            "match_not_found",
            "The specified match does not exist",
        )),

        Err(MatchActionFailure::Forbidden) => Err(api_error(
            Status::Forbidden,
            "not_lost_listing_owner",
            "Only the owner of the lost listing can confirm or reject this match",
        )),

        Err(MatchActionFailure::Conflict) => Err(api_error(
            Status::Conflict,
            "match_conflict",
            "The match or one of its listings is no longer in a state that allows this action",
        )),

        Err(MatchActionFailure::Database(error)) => {
            eprintln!("Match action database error: {error}");

            Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not process the match action",
            ))
        }
    }
}

#[get("/matches/<match_id>/contacts")]
pub async fn get_match_contacts(
    match_id: &str,
    auth: AuthUser,
    db: Db,
) -> Result<
    Json<MatchContactsResponse>,
    status::Custom<Json<ApiError>>,
> {
    let match_id = match uuid::Uuid::parse_str(match_id) {
        Ok(id) => id,
        Err(_) => {
            return Err(api_error(
                Status::BadRequest,
                "invalid_match_id",
                "Match ID must be a valid UUID",
            ));
        }
    };

    let requester_id = auth.user.id;

    let result = db
        .run(move |conn| {
            let match_record = matches::table
                .find(match_id)
                .select(Match::as_select())
                .first::<Match>(conn)
                .optional()?
                .ok_or(MatchActionFailure::NotFound)?;

            // Contact details are available only after confirmation.
            if match_record.status != DBMatchStatus::Confirmed {
                return Err(MatchActionFailure::Conflict);
            }

            let lost_listing = listings::table
                .find(match_record.lost_listing_id)
                .select(Listing::as_select())
                .first::<Listing>(conn)?;

            let found_listing = listings::table
                .find(match_record.found_listing_id)
                .select(Listing::as_select())
                .first::<Listing>(conn)?;

            // Determine which participant is requesting access.
            let other_user_id = if lost_listing.user_id == requester_id {
                found_listing.user_id
            } else if found_listing.user_id == requester_id {
                lost_listing.user_id
            } else {
                return Err(MatchActionFailure::Forbidden);
            };

            // Prevent returning the requester's own contact details
            // if both listings happen to belong to the same user.
            if other_user_id == requester_id {
                return Err(MatchActionFailure::Forbidden);
            }

            let other_user = users::table
                .find(other_user_id)
                .select(User::as_select())
                .first::<User>(conn)?;

            Ok(MatchContactsResponse {
                match_id,
                contact: MatchContactDto {
                    full_name: other_user.full_name,
                    email: other_user.email,
                    phone: other_user.phone,
                    telegram_handle: other_user.telegram_handle,
                    vk_handle: other_user.vk_handle,
                    preferred_contact: other_user.preferred_contact.into(),
                },
            })
        })
        .await;

    match result {
        Ok(response) => Ok(Json(response)),

        Err(MatchActionFailure::NotFound) => Err(api_error(
            Status::NotFound,
            "match_not_found",
            "The specified match does not exist",
        )),

        Err(MatchActionFailure::Forbidden) => Err(api_error(
            Status::Forbidden,
            "forbidden",
            "You are not a participant in this match",
        )),

        Err(MatchActionFailure::Conflict) => Err(api_error(
            Status::Conflict,
            "match_not_confirmed",
            "Contact information is available only after match confirmation",
        )),

        Err(MatchActionFailure::Database(error)) => {
            eprintln!("Get match contacts database error: {error}");

            Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not retrieve contact information",
            ))
        }
    }
}

