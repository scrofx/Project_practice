// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "contact_method"))]
    pub struct ContactMethod;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "listing_status"))]
    pub struct ListingStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "listing_type"))]
    pub struct ListingType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "match_status"))]
    pub struct MatchStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "moderation_action"))]
    pub struct ModerationAction;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "notification_type"))]
    pub struct NotificationType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "return_status"))]
    pub struct ReturnStatus;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
    pub struct Tsvector;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_role"))]
    pub struct UserRole;
}

diesel::table! {
    campus_buildings (id) {
        id -> Uuid,
        name -> Text,
        code -> Text,
        address -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    categories (id) {
        id -> Uuid,
        name -> Text,
        slug -> Text,
        icon -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ListingStatus;

    listing_status_history (id) {
        id -> Int8,
        listing_id -> Uuid,
        old_status -> Nullable<ListingStatus>,
        new_status -> ListingStatus,
        changed_by -> Nullable<Uuid>,
        note -> Nullable<Text>,
        changed_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ListingType;
    use super::sql_types::ListingStatus;
    use super::sql_types::Tsvector;

    listings (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[sql_name = "type"]
        type_ -> ListingType,
        category_id -> Uuid,
        building_id -> Uuid,
        title -> Text,
        description -> Text,
        photo_path -> Text,
        location_detail -> Nullable<Text>,
        event_date -> Date,
        status -> ListingStatus,
        moderated_by -> Nullable<Uuid>,
        moderated_at -> Nullable<Timestamptz>,
        search_vector -> Nullable<Tsvector>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MatchStatus;

    matches (id) {
        id -> Uuid,
        lost_listing_id -> Uuid,
        found_listing_id -> Uuid,
        score -> Numeric,
        status -> MatchStatus,
        confirmed_by -> Nullable<Uuid>,
        confirmed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ModerationAction;

    moderation_logs (id) {
        id -> Int8,
        listing_id -> Uuid,
        moderator_id -> Uuid,
        action -> ModerationAction,
        reason -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::NotificationType;

    notifications (id) {
        id -> Int8,
        user_id -> Uuid,
        #[sql_name = "type"]
        type_ -> NotificationType,
        payload -> Jsonb,
        is_read -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ReturnStatus;

    returns (id) {
        id -> Uuid,
        match_id -> Uuid,
        loser_confirmed -> Bool,
        loser_confirmed_at -> Nullable<Timestamptz>,
        finder_confirmed -> Bool,
        finder_confirmed_at -> Nullable<Timestamptz>,
        final_status -> ReturnStatus,
        returned_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    reviews (id) {
        id -> Uuid,
        return_id -> Uuid,
        reviewer_id -> Uuid,
        reviewee_id -> Uuid,
        rating -> Int2,
        comment -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserRole;
    use super::sql_types::ContactMethod;

    users (id) {
        id -> Uuid,
        email -> Citext,
        password_hash -> Text,
        full_name -> Text,
        role -> UserRole,
        phone -> Nullable<Text>,
        telegram_handle -> Nullable<Text>,
        vk_handle -> Nullable<Text>,
        preferred_contact -> ContactMethod,
        avatar_path -> Nullable<Text>,
        bio -> Nullable<Text>,
        is_active -> Bool,
        notifications_enabled -> Bool,
        rating_sum -> Int4,
        rating_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(listing_status_history -> listings (listing_id));
diesel::joinable!(listing_status_history -> users (changed_by));
diesel::joinable!(listings -> campus_buildings (building_id));
diesel::joinable!(listings -> categories (category_id));
diesel::joinable!(matches -> users (confirmed_by));
diesel::joinable!(moderation_logs -> listings (listing_id));
diesel::joinable!(moderation_logs -> users (moderator_id));
diesel::joinable!(notifications -> users (user_id));
diesel::joinable!(returns -> matches (match_id));
diesel::joinable!(reviews -> returns (return_id));

diesel::allow_tables_to_appear_in_same_query!(
    campus_buildings,
    categories,
    listing_status_history,
    listings,
    matches,
    moderation_logs,
    notifications,
    returns,
    reviews,
    users,
);
