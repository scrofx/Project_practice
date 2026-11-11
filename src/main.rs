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

    rocket::custom(figment)
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
            matching::get_listing_matches,
            matching::action_on_match,
            matching::get_match_contacts,
            notifications::get_notifications,
            returns::confirm_return,
            catalog::get_buildings,
            catalog::get_categories
        ])
}
