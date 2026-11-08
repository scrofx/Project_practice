use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use diesel_full_text_search::TsVector;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::UserRole"]
#[DbValueStyle = "snake_case"]
pub enum DBUserRole {
    Student,
    Moderator,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::ListingType"]
#[DbValueStyle = "snake_case"]
pub enum DBListingType {
    Lost,
    Found,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::ListingStatus"]
#[DbValueStyle = "snake_case"]
pub enum DBListingStatus {
    PendingReview,
    Active,
    Matched,
    Returned,
    Removed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::MatchStatus"]
#[DbValueStyle = "snake_case"]
pub enum DBMatchStatus {
    Suggested,
    Confirmed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::ReturnStatus"]
#[DbValueStyle = "snake_case"]
pub enum DBReturnStatus {
    Pending,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::ModerationAction"]
#[DbValueStyle = "snake_case"]
pub enum DBModerationAction {
    Approve,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::ContactMethod"]
#[DbValueStyle = "snake_case"]
pub enum DBContactMethod {
    Email,
    Phone,
    Telegram,
    Vk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::NotificationType"]
#[DbValueStyle = "snake_case"]
pub enum DBNotificationType {
    ListingCreated,
    ListingApproved,
    ListingRemoved,
    MatchFound,
    MatchConfirmed,
    ReturnConfirmedByOther,
    ReturnCompleted,
    ReviewReceived,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub role: DBUserRole,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: DBContactMethod,
    pub avatar_path: Option<String>,
    pub bio: Option<String>,
    pub is_active: bool,
    pub notifications_enabled: bool,
    pub rating_sum: i32,
    pub rating_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub role: Option<DBUserRole>,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: Option<DBContactMethod>,
    pub avatar_path: Option<String>,
    pub bio: Option<String>,
    pub is_active: Option<bool>,
    pub notifications_enabled: Option<bool>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::users)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
    pub role: Option<DBUserRole>,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: Option<DBContactMethod>,
    pub avatar_path: Option<String>,
    pub bio: Option<String>,
    pub is_active: Option<bool>,
    pub notifications_enabled: Option<bool>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::categories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::categories)]
pub struct NewCategory {
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::categories)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<Option<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::campus_buildings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CampusBuilding {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::campus_buildings)]
pub struct NewCampusBuilding {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::campus_buildings)]
pub struct UpdateCampusBuilding {
    pub name: Option<String>,
    pub code: Option<String>,
    pub address: Option<Option<String>>,
    pub is_active: Option<bool>,
}

#[derive(Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::listings)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Category, foreign_key = category_id))]
#[diesel(belongs_to(CampusBuilding, foreign_key = building_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Listing {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub type_: DBListingType,
    pub category_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: String,
    pub photo_path: String,
    pub location_detail: Option<String>,
    pub event_date: NaiveDate,
    pub status: DBListingStatus,
    pub moderated_by: Option<Uuid>,
    pub moderated_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::listings)]
pub struct NewListing {
    pub user_id: Uuid,
    pub type_: DBListingType,
    pub category_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: String,
    pub photo_path: String,
    pub location_detail: Option<String>,
    pub event_date: NaiveDate,
    pub status: Option<DBListingStatus>,
    pub moderated_by: Option<Uuid>,
    pub moderated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::listings)]
pub struct UpdateListing {
    pub type_: Option<DBListingType>,
    pub category_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub photo_path: Option<String>,
    pub location_detail: Option<Option<String>>,
    pub event_date: Option<NaiveDate>,
    pub status: Option<DBListingStatus>,
    pub moderated_by: Option<Option<Uuid>>,
    pub moderated_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::listing_status_history)]
#[diesel(belongs_to(Listing, foreign_key = listing_id))]
#[diesel(belongs_to(User, foreign_key = changed_by))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ListingStatusHistory {
    pub id: i64,
    pub listing_id: Uuid,
    pub old_status: Option<DBListingStatus>,
    pub new_status: DBListingStatus,
    pub changed_by: Option<Uuid>,
    pub note: Option<String>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::listing_status_history)]
pub struct NewListingStatusHistory {
    pub listing_id: Uuid,
    pub old_status: Option<DBListingStatus>,
    pub new_status: DBListingStatus,
    pub changed_by: Option<Uuid>,
    pub note: Option<String>,
}


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::matches)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Match {
    pub id: Uuid,
    pub lost_listing_id: Uuid,
    pub found_listing_id: Uuid,
    pub score: BigDecimal,
    pub status: DBMatchStatus,
    pub confirmed_by: Option<Uuid>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::matches)]
pub struct NewMatch {
    pub lost_listing_id: Uuid,
    pub found_listing_id: Uuid,
    pub score: BigDecimal,
    pub status: Option<DBMatchStatus>,
    pub confirmed_by: Option<Uuid>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::matches)]
pub struct UpdateMatch {
    pub score: Option<BigDecimal>,
    pub status: Option<DBMatchStatus>,
    pub confirmed_by: Option<Option<Uuid>>,
    pub confirmed_at: Option<Option<DateTime<Utc>>>,
}


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::returns)]
#[diesel(belongs_to(Match, foreign_key = match_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Return {
    pub id: Uuid,
    pub match_id: Uuid,
    pub loser_confirmed: bool,
    pub loser_confirmed_at: Option<DateTime<Utc>>,
    pub finder_confirmed: bool,
    pub finder_confirmed_at: Option<DateTime<Utc>>,
    pub final_status: DBReturnStatus,
    pub returned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::returns)]
pub struct NewReturn {
    pub match_id: Uuid,
    pub loser_confirmed: Option<bool>,
    pub loser_confirmed_at: Option<DateTime<Utc>>,
    pub finder_confirmed: Option<bool>,
    pub finder_confirmed_at: Option<DateTime<Utc>>,
    pub final_status: Option<DBReturnStatus>,
    pub returned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::returns)]
pub struct UpdateReturn {
    pub loser_confirmed: Option<bool>,
    pub loser_confirmed_at: Option<Option<DateTime<Utc>>>,
    pub finder_confirmed: Option<bool>,
    pub finder_confirmed_at: Option<Option<DateTime<Utc>>>,
    pub final_status: Option<DBReturnStatus>,
    pub returned_at: Option<Option<DateTime<Utc>>>,
}


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::reviews)]
#[diesel(belongs_to(Return, foreign_key = return_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Review {
    pub id: Uuid,
    pub return_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::reviews)]
pub struct NewReview {
    pub return_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i16,
    pub comment: Option<String>,
}


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::notifications)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Notification {
    pub id: i64,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub type_: DBNotificationType,
    pub payload: Value,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::notifications)]
pub struct NewNotification {
    pub user_id: Uuid,
    pub type_: DBNotificationType,
    pub payload: Option<Value>,
    pub is_read: Option<bool>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = crate::schema::notifications)]
pub struct UpdateNotification {
    pub payload: Option<Value>,
    pub is_read: Option<bool>,
}


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::moderation_logs)]
#[diesel(belongs_to(Listing, foreign_key = listing_id))]
#[diesel(belongs_to(User, foreign_key = moderator_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ModerationLog {
    pub id: i64,
    pub listing_id: Uuid,
    pub moderator_id: Uuid,
    pub action: DBModerationAction,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::moderation_logs)]
pub struct NewModerationLog {
    pub listing_id: Uuid,
    pub moderator_id: Uuid,
    pub action: DBModerationAction,
    pub reason: Option<String>,
}