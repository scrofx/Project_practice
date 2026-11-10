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

use rocket::{get, launch, routes};
use rocket_sync_db_pools::database;


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
            returns::confirm_return,
            catalog::get_buildings,
            catalog::get_categories
        ])
}
