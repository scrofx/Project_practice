#![feature(trivial_bounds)]
mod db;
mod schema;
pub mod models;
pub mod auth;
mod enums;
mod listings;
mod catalog;
mod returns;
mod matching;
mod notifications;

use rocket::{get, launch, routes};
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket_sync_db_pools::database;
use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use campuslost_shared::ApiError;

pub fn api_error(
    http_status: Status,
    error: &str,
    message: &str,
) -> status::Custom<Json<ApiError>> {
    status::Custom(
        http_status,
        Json(ApiError { error: error.parse().unwrap(), message: message.parse().unwrap() }),
    )
}

#[get("/health")]
fn health() -> &'static str {
    "ok"
}

#[launch]
async fn rocket() -> _ {
    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set before starting the backend");

    let figment = rocket::Config::figment()
        .merge(("databases.campus_lost.url", database_url))
        .merge(("databases.campus_lost.pool_size", 10));

    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::some_exact(&[
            "http://127.0.0.1:8080",
        ]),
        allowed_methods: vec![
            Method::Get,
            Method::Post,
            Method::Patch,
        ]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::some(&[
            "Authorization",
            "Content-Type",
        ]),
        ..Default::default()
    }
        .to_cors()
        .expect("Failed to configure CORS");

    rocket::custom(figment)
        .attach(cors)
        // Register the pool before the startup healthcheck fairing.
        .attach(db::Db::fairing())
        .attach(db::DatabaseStartupHealthcheck)
        .mount("/", routes![
            health,
            auth::login,
            auth::register,
            auth::me,
            listings::create_listing,
            listings::get_listings,
            listings::get_pending_listings,
            listings::moderate_listing,
            listings::get_my_listings,
            matching::get_listing_matches,
            matching::action_on_match,
            matching::get_match_contacts,
            notifications::get_notifications,
            notifications::mark_notification_as_read,
            returns::confirm_return,
            catalog::get_buildings,
            catalog::get_categories
        ])
}
