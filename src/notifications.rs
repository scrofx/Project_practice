
use crate::auth::AuthUser;
use crate::db::Db;
use crate::models::{
    DBNotificationType,
    Notification,
};
use crate::schema::notifications;
use campuslost_shared::{ApiError, NotificationDto, NotificationType, NotificationsResponse};
use diesel::dsl::count_star;
use diesel::prelude::*;
use rocket::{get, http::Status, patch, response::status, serde::json::Json};
use crate::api_error;

#[get("/notifications")]
pub async fn get_notifications(
    auth: AuthUser,
    db: Db,
) -> Result<
    Json<NotificationsResponse>,
    status::Custom<Json<ApiError>>,
> {
    let user_id = auth.user.id;

    let result = db
        .run(move |conn| {
            // Retrieve this user's notifications, newest first.
            let notification_records = notifications::table
                .filter(notifications::user_id.eq(user_id))
                .order((
                    notifications::created_at.desc(),
                    notifications::id.desc(),
                ))
                .select(Notification::as_select())
                .load::<Notification>(conn)?;

            // Count only this user's unread notifications.
            let unread_counter = notification_records
                .iter()
                .filter(|notification| !notification.is_read)
                .count() as i64;

            Ok::<_, diesel::result::Error>((
                notification_records,
                unread_counter,
            ))
        })
        .await;

    match result {
        Ok((records, unread_counter)) => {
            let notification_dtos = records
                .into_iter()
                .map(|notification| NotificationDto {
                    id: notification.id,
                    user_id: notification.user_id,
                    notification_type: notification.type_.into(),
                    payload: notification.payload,
                    is_read: notification.is_read,
                    created_at: notification.created_at,
                })
                .collect();

            Ok(Json(NotificationsResponse {
                notifications: notification_dtos,
                unread_counter,
            }))
        }

        Err(error) => {
            eprintln!("Get notifications database error: {error}");

            Err(api_error(
                Status::InternalServerError,
                "database_error",
                "Could not retrieve notifications",
            ))
        }
    }
}

#[patch("/notifications/<notification_id>/read")]
pub async fn mark_notification_as_read(
    notification_id: i64,
    auth: AuthUser,
    db: Db,
) -> Result<Status, status::Custom<Json<ApiError>>> {
        let user_id = auth.user.id;

    let result = db
        .run(move |conn| {
            diesel::update(
                notifications::table
                    .filter(notifications::id.eq(notification_id))
                    .filter(notifications::user_id.eq(user_id)),
            )
                .set(notifications::is_read.eq(true))
                .execute(conn)
        })
        .await;

    match result {
        Ok(1) => Ok(Status::NoContent),
        Ok(0) => Err(api_error(
            Status::NotFound,
            "notification_not_found",
            "Notification not found",
        )),
        Ok(_) => unreachable!("Updating by primary key affected multiple rows"),
        Err(_) => Err(api_error(
            Status::InternalServerError,
            "database_error",
            "Could not update notification",
        )),
    }
}
