// Shared API contracts for CampusLost.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
// Enums: serialized values match the PostgreSQL enum labels.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Student,
    Moderator,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingType {
    Lost,
    Found,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    PendingReview,
    Active,
    Matched,
    Returned,
    Removed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    Suggested,
    Confirmed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnStatus {
    Pending,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationAction {
    Approve,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactMethod {
    Email,
    Phone,
    Telegram,
    Vk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    ListingCreated,
    ListingApproved,
    ListingRemoved,
    MatchFound,
    MatchConfirmed,
    ReturnConfirmedByOther,
    ReturnCompleted,
    ReviewReceived,
}

// Authentication DTOs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: Option<ContactMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub user: UserDto,
}

// users

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: ContactMethod,
    pub avatar_path: Option<String>,
    pub bio: Option<String>,
    pub is_active: bool,
    pub notifications_enabled: bool,
    pub rating_sum: i32,
    pub rating_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: Option<ContactMethod>,
    pub bio: Option<String>,
    pub notifications_enabled: Option<bool>,
}

// categories and campus_buildings

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampusBuildingDto {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// listings

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDto {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub listing_type: ListingType,
    pub category_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: String,
    pub photo_path: String,
    pub location_detail: Option<String>,
    pub event_date: NaiveDate,
    pub status: ListingStatus,
    pub moderated_by: Option<Uuid>,
    pub moderated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Metadata for POST /listings. Upload the image separately as multipart data;
/// the server generates and stores photo_path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListingRequest {
    #[serde(rename = "type")]
    pub listing_type: ListingType,
    pub category_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: String,
    pub location_detail: Option<String>,
    pub event_date: NaiveDate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateListingRequest {
    pub category_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub location_detail: Option<String>,
    pub event_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListingFilters {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub listing_type: Option<ListingType>,
    pub category_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub status: Option<ListingStatus>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeListingStatusRequest {
    pub status: ListingStatus,
    pub note: Option<String>,
}

// listing_status_history

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingStatusHistoryDto {
    pub id: i64,
    pub listing_id: Uuid,
    pub old_status: Option<ListingStatus>,
    pub new_status: ListingStatus,
    pub changed_by: Option<Uuid>,
    pub note: Option<String>,
    pub changed_at: DateTime<Utc>,
}

// matches

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDto {
    pub id: Uuid,
    pub lost_listing_id: Uuid,
    pub found_listing_id: Uuid,
    /// NUMERIC(5,2) from PostgreSQL, represented as a JSON number in the API.
    pub score: f64,
    pub status: MatchStatus,
    pub confirmed_by: Option<Uuid>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchAction {
    Confirm,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchActionRequest {
    pub action: MatchAction,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MatchContactDto {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub telegram_handle: Option<String>,
    pub vk_handle: Option<String>,
    pub preferred_contact: ContactMethod,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MatchContactsResponse {
    pub match_id: uuid::Uuid,
    pub contact: MatchContactDto,
}

#[derive(Debug, serde::Serialize)]
pub struct NotificationsResponse {
    pub notifications: Vec<NotificationDto>,
    pub unread_counter: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MatchActionResponse {
    pub match_data: MatchDto,
    pub return_id: Option<uuid::Uuid>,
}

// returns

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnDto {
    pub id: Uuid,
    pub match_id: Uuid,
    pub loser_confirmed: bool,
    pub loser_confirmed_at: Option<DateTime<Utc>>,
    pub finder_confirmed: bool,
    pub finder_confirmed_at: Option<DateTime<Utc>>,
    pub final_status: ReturnStatus,
    pub returned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnConfirmation {
    Loser,
    Finder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnConfirmationRequest {
    pub return_id: Uuid,
    pub confirmation: ReturnConfirmation,
}

// reviews

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDto {
    pub id: Uuid,
    pub return_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewee_id: Uuid,
    pub rating: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewRequest {
    pub rating: i16,
    pub comment: Option<String>,
}

// notifications

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDto {
    pub id: i64,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub payload: Value,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

// moderation_logs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationLogDto {
    pub id: i64,
    pub listing_id: Uuid,
    pub moderator_id: Uuid,
    pub action: ModerationAction,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationActionRequest {
    pub listing_id: Uuid,
    pub action: ModerationAction,
    pub reason: Option<String>,
}

// Generic API response helpers

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
}
