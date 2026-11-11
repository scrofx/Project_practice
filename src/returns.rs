use chrono::Utc;
use diesel::prelude::*;
use rocket::{
    http::Status,
    post,
    response::status,
    serde::json::Json,
};
use campuslost_shared::{
    ReturnConfirmation,
    ReturnConfirmationRequest,
    ReturnDto,
    ReturnStatus as ApiReturnStatus,
    ApiError,
};
use serde::Serialize;
use serde_json::json;

use crate::{api_error, auth::AuthUser, db::Db, models::{
    Listing, DBListingStatus, Match, DBMatchStatus, NewListingStatusHistory,
    NewNotification, DBNotificationType, Return as DbReturn,
    DBReturnStatus,
}, schema::{
    listing_status_history, listings, matches, notifications, returns,
}};


fn return_to_dto(record: DbReturn) -> ReturnDto {
    ReturnDto {
        id: record.id,
        match_id: record.match_id,
        loser_confirmed: record.loser_confirmed,
        loser_confirmed_at: record.loser_confirmed_at,
        finder_confirmed: record.finder_confirmed,
        finder_confirmed_at: record.finder_confirmed_at,
        final_status: match record.final_status {
            DBReturnStatus::Pending => ApiReturnStatus::Pending,
            DBReturnStatus::Completed => ApiReturnStatus::Completed,
            DBReturnStatus::Cancelled => ApiReturnStatus::Cancelled,
        },
        returned_at: record.returned_at,
        created_at: record.created_at,
    }
}

enum ReturnFailure {
    NotFound,
    Forbidden,
    Conflict,
    Database(diesel::result::Error),
}

impl From<diesel::result::Error> for ReturnFailure {
    fn from(error: diesel::result::Error) -> Self {
        Self::Database(error)
    }
}

#[post("/returns/confirm", format = "json", data = "<payload>")]
pub async fn confirm_return(
    db: Db,
    auth: AuthUser,
    payload: Json<ReturnConfirmationRequest>,
) -> Result<Json<ReturnDto>, status::Custom<Json<ApiError>>> {
    let request = payload.into_inner();
    let user_id = auth.user.id;

    let result = db
        .run(move |conn| {
            conn.transaction::<DbReturn, ReturnFailure, _>(|conn| {
                // Lock the return row. This serializes confirmations for this return.
                let return_record = returns::table
                    .find(request.return_id)
                    .for_update()
                    .select(DbReturn::as_select())
                    .first::<DbReturn>(conn)
                    .optional()?
                    .ok_or(ReturnFailure::NotFound)?;

                if return_record.final_status != DBReturnStatus::Pending {
                    return Err(ReturnFailure::Conflict);
                }

                // The match must already have been confirmed.
                let match_record = matches::table
                    .find(return_record.match_id)
                    .for_update()
                    .select(Match::as_select())
                    .first::<Match>(conn)?;

                if match_record.status != DBMatchStatus::Confirmed {
                    return Err(ReturnFailure::Conflict);
                }

                let lost_listing = listings::table
                    .find(match_record.lost_listing_id)
                    .select(Listing::as_select())
                    .first::<Listing>(conn)?;

                let found_listing = listings::table
                    .find(match_record.found_listing_id)
                    .select(Listing::as_select())
                    .first::<Listing>(conn)?;

                let (is_loser, other_user_id, already_confirmed) =
                    match request.confirmation {
                        ReturnConfirmation::Loser => {
                            if lost_listing.user_id != user_id {
                                return Err(ReturnFailure::Forbidden);
                            }

                            (
                                true,
                                found_listing.user_id,
                                return_record.loser_confirmed,
                            )
                        }
                        ReturnConfirmation::Finder => {
                            if found_listing.user_id != user_id {
                                return Err(ReturnFailure::Forbidden);
                            }

                            (
                                false,
                                lost_listing.user_id,
                                return_record.finder_confirmed,
                            )
                        }
                    };

                if already_confirmed {
                    return Err(ReturnFailure::Conflict);
                }

                let now = Utc::now();

                // Record this participant's confirmation.
                let updated_return = if is_loser {
                    diesel::update(returns::table.find(return_record.id))
                        .set((
                            returns::loser_confirmed.eq(true),
                            returns::loser_confirmed_at.eq(Some(now)),
                        ))
                        .execute(conn)?;

                    DbReturn {
                        loser_confirmed: true,
                        loser_confirmed_at: Some(now),
                        ..return_record
                    }
                } else {
                    diesel::update(returns::table.find(return_record.id))
                        .set((
                            returns::finder_confirmed.eq(true),
                            returns::finder_confirmed_at.eq(Some(now)),
                        ))
                        .execute(conn)?;

                    DbReturn {
                        finder_confirmed: true,
                        finder_confirmed_at: Some(now),
                        ..return_record
                    }
                };

                // If only one participant has confirmed, notify the other participant.
                if !(updated_return.loser_confirmed && updated_return.finder_confirmed) {
                    diesel::insert_into(notifications::table)
                        .values(&NewNotification {
                            user_id: other_user_id,
                            type_: DBNotificationType::ReturnConfirmedByOther,
                            payload: Some(json!({
                                "return_id": updated_return.id,
                                "match_id": updated_return.match_id,
                            })),
                            is_read: Some(false),
                        })
                        .execute(conn)?;

                    return Ok(updated_return);
                }

                diesel::update(returns::table.find(updated_return.id))
                    .set((
                        returns::final_status.eq(DBReturnStatus::Completed),
                        returns::returned_at.eq(Some(now)),
                    ))
                    .execute(conn)?;

                // Both listings must still be in the matched state.
                // Move each through returned, then archived, recording both transitions.
                for listing_id in [
                    match_record.lost_listing_id,
                    match_record.found_listing_id,
                ] {
                    let changed = diesel::update(
                        listings::table
                            .filter(listings::id.eq(listing_id))
                            .filter(listings::status.eq(DBListingStatus::Matched)),
                    )
                        .set((
                            listings::status.eq(DBListingStatus::Returned),
                            listings::updated_at.eq(now),
                        ))
                        .execute(conn)?;

                    if changed != 1 {
                        return Err(ReturnFailure::Conflict);
                    }

                    diesel::insert_into(listing_status_history::table)
                        .values(&NewListingStatusHistory {
                            listing_id,
                            old_status: Some(DBListingStatus::Matched),
                            new_status: DBListingStatus::Returned,
                            changed_by: Some(user_id),
                            note: Some("Return confirmed by both participants".to_string()),
                        })
                        .execute(conn)?;

                    let changed = diesel::update(
                        listings::table
                            .filter(listings::id.eq(listing_id))
                            .filter(listings::status.eq(DBListingStatus::Returned)),
                    )
                        .set((
                            listings::status.eq(DBListingStatus::Archived),
                            listings::updated_at.eq(now),
                        ))
                        .execute(conn)?;

                    if changed != 1 {
                        return Err(ReturnFailure::Conflict);
                    }

                    diesel::insert_into(listing_status_history::table)
                        .values(&NewListingStatusHistory {
                            listing_id,
                            old_status: Some(DBListingStatus::Returned),
                            new_status: DBListingStatus::Archived,
                            changed_by: Some(user_id),
                            note: Some("Listing archived after completed return".to_string()),
                        })
                        .execute(conn)?;
                }

                // Notify both participants that the return is complete.
                for participant_id in [lost_listing.user_id, found_listing.user_id] {
                    diesel::insert_into(notifications::table)
                        .values(&NewNotification {
                            user_id: participant_id,
                            type_: DBNotificationType::ReturnCompleted,
                            payload: Some(json!({
                                "return_id": updated_return.id,
                                "match_id": updated_return.match_id,
                            })),
                            is_read: Some(false),
                        })
                        .execute(conn)?;
                }

                Ok(DbReturn {
                    loser_confirmed: true,
                    loser_confirmed_at: updated_return.loser_confirmed_at,
                    finder_confirmed: true,
                    finder_confirmed_at: updated_return.finder_confirmed_at,
                    final_status: DBReturnStatus::Completed,
                    returned_at: Some(now),
                    ..updated_return
                })
            })
        })
        .await;

    match result {
        Ok(record) => Ok(Json(return_to_dto(record))),
        Err(ReturnFailure::NotFound) => Err(api_error(
            Status::NotFound,
            "return_not_found",
            "The specified return does not exist",
        )),
        Err(ReturnFailure::Forbidden) => Err(api_error(
            Status::Forbidden,
            "not_a_participant",
            "You do not own the listing for the requested confirmation side",
        )),
        Err(ReturnFailure::Conflict) => Err(api_error(
            Status::Conflict,
            "return_conflict",
            "The return cannot be confirmed in its current state, or this side has already confirmed",
        )),
        Err(ReturnFailure::Database(error)) => {
            eprintln!("Return confirmation database error: {error}");
            Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not process the return confirmation",
            ))
        }
    }
}
