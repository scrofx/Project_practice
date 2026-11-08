#![feature(trivial_bounds)]
mod db;
mod schema;
pub mod models;
pub mod auth;
mod enums;


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
            auth::me
        ])
}
