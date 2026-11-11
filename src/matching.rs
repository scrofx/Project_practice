use rocket::{
    get,
    http::Status,
    response::status,
    serde::json::Json,
};
use campuslost_shared::{MatchDto, MatchStatus};
use crate::auth::AuthUser;
use crate::db::Db;
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::{prelude::*, PgConnection};
use serde_json::json;
use std::collections::HashSet;
use diesel::dsl::sql;
use diesel::sql_types::{Bool, Text};
use crate::{
    models::{
        DBListingStatus, DBListingType, DBMatchStatus, DBNotificationType,
        Listing, Match, NewMatch, NewNotification,
    },
    schema::{listings, matches, notifications},
};

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

    if first_title.contains(&second_title)
        || second_title.contains(&first_title)
    {
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
                        to_tsvector('simple', "
            )
                .bind::<Text, _>(&search_text)
                .sql(")), ' | ')
            )"),
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
    scored_candidates.sort_by(
        |(first, first_score), (second, second_score)| {
            second_score
                .cmp(first_score)
                .then_with(|| first.id.cmp(&second.id))
        },
    );

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


#[derive(Debug, serde::Serialize)]
pub struct MatchApiError {
    pub error: String,
    pub message: String,
}

fn match_api_error(
    status_code: Status,
    error: &str,
    message: &str,
) -> status::Custom<Json<MatchApiError>> {
    status::Custom(
        status_code,
        Json(MatchApiError {
            error: error.to_string(),
            message: message.to_string(),
        }),
    )
}

#[get("/listings/<listing_id>/matches")]
pub async fn get_listing_matches(
    listing_id: &str,
    auth: AuthUser,
    db: Db,
) -> Result<Json<Vec<MatchDto>>, status::Custom<Json<MatchApiError>>> {
    let listing_id = match uuid::Uuid::parse_str(listing_id) {
        Ok(id) => id,
        Err(_) => {
            return Err(match_api_error(
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
            return Err(match_api_error(
                Status::NotFound,
                "listing_not_found",
                "Listing was not found",
            ));
        }
        Err(_) => {
            return Err(match_api_error(
                Status::InternalServerError,
                "database_error",
                "Could not retrieve listing",
            ));
        }
    };

    if listing.user_id != auth.user.id {
        return Err(match_api_error(
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
        Err(_) => Err(match_api_error(
            Status::InternalServerError,
            "database_error",
            "Could not retrieve match suggestions",
        )),
    }
}

