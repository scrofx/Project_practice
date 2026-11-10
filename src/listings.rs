use campuslost_shared::{CreateListingRequest, ListingDto, ListingFilters, ListingStatus, ListingType, ModerationAction, ModerationActionRequest};
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::PgTextExpressionMethods;
use rocket::{form::FromForm, get, http::Status, post, response::status, serde::json::Json};
use serde_json::json;
use std::str::FromStr;
use uuid::Uuid;

use crate::models::{
    DBModerationAction, DBNotificationType, NewListingStatusHistory, NewModerationLog,
    NewNotification,
};
use crate::schema::{listing_status_history, moderation_logs, notifications};
use crate::{
    auth::{AuthUser, ModeratorGuard},
    db::Db,
    models::{DBListingStatus, DBListingType, Listing, NewListing},
    schema::listings,
};

#[derive(Debug, serde::Serialize)]
pub struct ListingApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug)]
enum ModerationTransactionError {
    NotPending,
    Database(diesel::result::Error),
}

impl From<diesel::result::Error> for ModerationTransactionError {
    fn from(error: diesel::result::Error) -> Self {
        ModerationTransactionError::Database(error)
    }
}

#[derive(Debug, FromForm)]
pub struct ListingQuery {
    pub q: Option<String>,
    #[field(name = "type")]
    pub listing_type: Option<String>,
    pub category_id: Option<String>,
    pub building_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub include_archived: Option<bool>,
}

impl ListingQuery {
    pub fn into_filters(self) -> Result<ListingFilters, String> {
        let listing_type = self
            .listing_type
            .map(|value| match value.as_str() {
                "lost" => Ok(ListingType::Lost),
                "found" => Ok(ListingType::Found),
                _ => Err("Invalid listing type".to_string()),
            })
            .transpose()?;

        let status = self
            .status
            .map(|value| match value.as_str() {
                "pending_review" => Ok(ListingStatus::PendingReview),
                "active" => Ok(ListingStatus::Active),
                "matched" => Ok(ListingStatus::Matched),
                "returned" => Ok(ListingStatus::Returned),
                "removed" => Ok(ListingStatus::Removed),
                "archived" => Ok(ListingStatus::Archived),
                _ => Err("Invalid listing status".to_string()),
            })
            .transpose()?;

        let category_id = self
            .category_id
            .map(|value| Uuid::parse_str(&value).map_err(|_| "Invalid category_id".to_string()))
            .transpose()?;

        let building_id = self
            .building_id
            .map(|value| Uuid::parse_str(&value).map_err(|_| "Invalid building_id".to_string()))
            .transpose()?;

        let date_from = self
            .date_from
            .map(|value| {
                NaiveDate::parse_from_str(&value, "%d-%m-%Y")
                    .map_err(|_| "Invalid date_from. Expected YYYY-MM-DD".to_string())
            })
            .transpose()?;

        let date_to = self
            .date_to
            .map(|value| {
                NaiveDate::parse_from_str(&value, "%d-%m-%Y")
                    .map_err(|_| "Invalid date_to. Expected YYYY-MM-DD".to_string())
            })
            .transpose()?;

        Ok(ListingFilters {
            q: self.q,
            listing_type,
            category_id,
            building_id,
            date_from,
            date_to,
            status,
            include_archived: self.include_archived,
        })
    }
}

fn api_error(
    status_code: Status,
    error: &str,
    message: &str,
) -> status::Custom<Json<ListingApiError>> {
    status::Custom(
        status_code,
        Json(ListingApiError {
            error: error.to_string(),
            message: message.to_string(),
        }),
    )
}

// Convert the database model into the public API DTO.
fn listing_to_dto(listing: Listing) -> ListingDto {
    ListingDto {
        id: listing.id,
        user_id: listing.user_id,
        listing_type: match listing.type_ {
            DBListingType::Lost => ListingType::Lost,
            DBListingType::Found => ListingType::Found,
        },
        category_id: listing.category_id,
        building_id: listing.building_id,
        title: listing.title,
        description: listing.description,
        photo_path: listing.photo_path,
        location_detail: listing.location_detail,
        event_date: listing.event_date,
        status: match listing.status {
            DBListingStatus::PendingReview => ListingStatus::PendingReview,
            DBListingStatus::Active => ListingStatus::Active,
            DBListingStatus::Matched => ListingStatus::Matched,
            DBListingStatus::Returned => ListingStatus::Returned,
            DBListingStatus::Removed => ListingStatus::Removed,
            DBListingStatus::Archived => ListingStatus::Archived,
        },
        moderated_by: listing.moderated_by,
        moderated_at: listing.moderated_at,
        created_at: listing.created_at,
        updated_at: listing.updated_at,
    }
}

#[post("/listings", format = "json", data = "<payload>")]
pub async fn create_listing(
    db: Db,
    auth: AuthUser,
    payload: Json<CreateListingRequest>,
) -> Result<Json<ListingDto>, status::Custom<Json<ListingApiError>>> {
    let request = payload.into_inner();

    // Basic input validation.
    let title = request.title.trim().to_string();
    let description = request.description.trim().to_string();

    if title.is_empty() {
        return Err(api_error(
            Status::BadRequest,
            "invalid_title",
            "Title cannot be empty",
        ));
    }

    if title.len() > 150 {
        return Err(api_error(
            Status::BadRequest,
            "invalid_title",
            "Title cannot exceed 150 characters",
        ));
    }

    if description.is_empty() {
        return Err(api_error(
            Status::BadRequest,
            "invalid_description",
            "Description cannot be empty",
        ));
    }

    let new_listing = NewListing {
        user_id: auth.user.id,
        type_: match request.listing_type {
            ListingType::Lost => DBListingType::Lost,
            ListingType::Found => DBListingType::Found,
        },
        category_id: request.category_id,
        building_id: request.building_id,
        title,
        description,

        // TEMPORARY: replaced by the actual uploaded image path
        // when we implement multipart upload.
        photo_path: String::new(),

        location_detail: request.location_detail,
        event_date: request.event_date,
        status: Some(DBListingStatus::PendingReview),
        moderated_by: None,
        moderated_at: None,
    };

    let result = db
        .run(move |conn| {
            diesel::insert_into(listings::table)
                .values(&new_listing)
                .returning(Listing::as_returning())
                .get_result::<Listing>(conn)
        })
        .await;

    match result {
        Ok(listing) => Ok(Json(listing_to_dto(listing))),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            _,
        )) => Err(api_error(
            Status::BadRequest,
            "invalid_reference",
            "The specified category or building does not exist",
        )),
        Err(_) => Err(api_error(
            Status::InternalServerError,
            "database_error",
            "Could not create listing",
        )),
    }
}

#[get("/listings?<query..>")]
pub async fn get_listings(
    query: ListingQuery,
    db: Db,
) -> Result<Json<Vec<ListingDto>>, status::Custom<Json<ListingApiError>>> {
    // Parse and validate query parameters.
    let filters = query
        .into_filters()
        .map_err(|message| api_error(Status::BadRequest, "invalid_filter", &message))?;

    let result = db
        .run(move |conn| {
            let mut db_query = listings::table
                .into_boxed::<diesel::pg::Pg>()
                .filter(listings::status.ne(DBListingStatus::PendingReview))
                .filter(listings::status.ne(DBListingStatus::Removed));

            if let Some(keyword) = filters.q {
                let pattern = format!("%{}%", keyword);

                db_query = db_query.filter(
                    listings::title
                        .ilike(pattern.clone())
                        .or(listings::description.ilike(pattern)),
                );
            }

            if let Some(listing_type) = filters.listing_type {
                let db_type = match listing_type {
                    ListingType::Lost => DBListingType::Lost,
                    ListingType::Found => DBListingType::Found,
                };

                db_query = db_query.filter(listings::type_.eq(db_type));
            }

            if let Some(category_id) = filters.category_id {
                db_query = db_query.filter(listings::category_id.eq(category_id));
            }

            if let Some(building_id) = filters.building_id {
                db_query = db_query.filter(listings::building_id.eq(building_id));
            }

            if let Some(date_from) = filters.date_from {
                db_query = db_query.filter(listings::event_date.ge(date_from));
            }

            if let Some(date_to) = filters.date_to {
                db_query = db_query.filter(listings::event_date.le(date_to));
            }

            if filters.include_archived != Some(true) {
                db_query = db_query.filter(listings::status.ne(DBListingStatus::Archived));
            }

            if let Some(status) = filters.status {
                let db_status = match status {
                    ListingStatus::PendingReview => DBListingStatus::PendingReview,
                    ListingStatus::Active => DBListingStatus::Active,
                    ListingStatus::Matched => DBListingStatus::Matched,
                    ListingStatus::Returned => DBListingStatus::Returned,
                    ListingStatus::Removed => DBListingStatus::Removed,
                    ListingStatus::Archived => DBListingStatus::Archived,
                };

                db_query = db_query
                    .filter(listings::status.eq(db_status))
                    .filter(listings::status.ne(DBListingStatus::Removed))
                    .filter(listings::status.ne(DBListingStatus::PendingReview));
            }

            db_query.select(Listing::as_select()).load::<Listing>(conn)
        })
        .await;

    match result {
        Ok(items) => {
            let items = items.into_iter().map(listing_to_dto).collect();

            Ok(Json(items))
        }
        Err(_) => Err(api_error(
            Status::InternalServerError,
            "database_error",
            "Could not retrieve listings",
        )),
    }
}

#[get("/moderation/listings")]
pub async fn get_pending_listings(
    _moderator: ModeratorGuard,
    db: Db,
) -> Result<Json<Vec<ListingDto>>, status::Custom<Json<ListingApiError>>> {
    let result = db
        .run(|conn| {
            listings::table
                .filter(listings::status.eq(DBListingStatus::PendingReview))
                .order(listings::created_at.asc())
                .select(Listing::as_select())
                .load::<Listing>(conn)
        })
        .await;

    match result {
        Ok(items) => {
            let items = items.into_iter().map(listing_to_dto).collect();

            Ok(Json(items))
        }
        Err(_) => Err(api_error(
            Status::InternalServerError,
            "database_error",
            "Could not retrieve pending listings",
        )),
    }
}

#[post("/moderation/listings", data = "<request>")]
pub async fn moderate_listing(
    moderator: ModeratorGuard,
    db: Db,
    request: Json<ModerationActionRequest>,
) -> Result<Json<ListingDto>, status::Custom<Json<ListingApiError>>> {
    let request = request.into_inner();

    let listing_id = request.listing_id;
    let moderator_id = moderator.0.user.id;

    let reason = request
        .reason
        .map(|reason| reason.trim().to_string())
        .filter(|reason| !reason.is_empty());

    if request.action == ModerationAction::Remove && reason.is_none() {
        return Err(api_error(
            Status::BadRequest,
            "rejection_reason_required",
            "A reason must be provided when removing a listing",
        ));
    }


    let (new_status, db_action, notification_type) =
        match request.action {
            ModerationAction::Approve => (
                DBListingStatus::Active,
                DBModerationAction::Approve,
                DBNotificationType::ListingApproved,
            ),
            ModerationAction::Remove => (
                DBListingStatus::Removed,
                DBModerationAction::Remove,
                DBNotificationType::ListingRemoved,
            ),
        };

    let result = db
        .run(move |conn| {
            conn.transaction::<Listing, ModerationTransactionError, _>(|conn| {
                let now = Utc::now();

                let moderated_listing = diesel::update(
                    listings::table
                        .filter(listings::id.eq(listing_id))
                        .filter(
                            listings::status.eq(DBListingStatus::PendingReview),
                        ),
                )
                    .set((
                        listings::status.eq(new_status),
                        listings::moderated_by.eq(Some(moderator_id)),
                        listings::moderated_at.eq(Some(now)),
                        listings::updated_at.eq(now),
                    ))
                    .returning(Listing::as_returning())
                    .get_result::<Listing>(conn)
                    .map_err(|error| match error {
                        diesel::result::Error::NotFound => {
                            ModerationTransactionError::NotPending
                        }
                        other => ModerationTransactionError::Database(other),
                    })?;


                let history = NewListingStatusHistory {
                    listing_id,
                    old_status: Some(DBListingStatus::PendingReview),
                    new_status,
                    changed_by: Some(moderator_id),
                    note: Some(
                        reason.clone().unwrap_or("No reason provided, pray this is an approve".parse().unwrap())
                    ),
                };

                diesel::insert_into(listing_status_history::table)
                    .values(&history)
                    .execute(conn)?;

                let moderation_log = NewModerationLog {
                    listing_id,
                    moderator_id,
                    action: db_action,
                    reason: reason.clone(),
                };

                diesel::insert_into(moderation_logs::table)
                    .values(&moderation_log)
                    .execute(conn)?;

                let notification = NewNotification {
                    user_id: moderated_listing.user_id,
                    type_: notification_type,
                    payload: Some(json!({
                        "listing_id": moderated_listing.id,
                        "title": moderated_listing.title.clone(),
                        "reason": reason,
                    })),
                    is_read: Some(false),
                };

                diesel::insert_into(notifications::table)
                    .values(&notification)
                    .execute(conn)?;

                Ok(moderated_listing)
            })
        })
        .await;

    match result {
        Ok(listing) => Ok(Json(listing_to_dto(listing))),

        Err(ModerationTransactionError::NotPending) => Err(api_error(
            Status::Conflict,
            "listing_not_pending",
            "The listing does not exist or is no longer awaiting review",
        )),

        Err(ModerationTransactionError::Database(error)) => {
            eprintln!("Could not moderate listing: {:?}", error);

            Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not moderate listing",
            ))
        }
    }
}
