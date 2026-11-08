use diesel::prelude::*;
use rocket::{
    http::Status,
    post,
    response::status,
    serde::json::Json,
};
use campuslost_shared::{
    CreateListingRequest,
    ListingDto,
    ListingStatus,
    ListingType,
};

use crate::{
    auth::AuthUser,
    db::Db,
    models::{DBListingStatus, DBListingType, Listing, NewListing},
    schema::listings,
};

#[derive(Debug, serde::Serialize)]
pub struct ListingApiError {
    pub error: String,
    pub message: String,
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
        Err(diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation, _)) => Err(api_error(
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