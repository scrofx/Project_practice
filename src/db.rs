use diesel::{
    dsl::sql_query,
    prelude::*,
    sql_types::Integer,
};
use rocket::{
    fairing::{self, Fairing, Info, Kind},
    Build, Rocket,
};
use rocket_sync_db_pools::{database, diesel};

/// Rocket-managed synchronous Diesel/PostgreSQL connection pool.
#[database("CampusLost")]
pub struct Db(diesel::PgConnection);

#[derive(QueryableByName)]
struct HealthcheckRow {
    #[diesel(sql_type = Integer)]
    _result: i32,
}

/// Runs a real database query during Rocket ignition. If PostgreSQL cannot
/// be reached (or the query fails), Rocket ignition is aborted.
pub struct DatabaseStartupHealthcheck;

#[rocket::async_trait]
impl Fairing for DatabaseStartupHealthcheck {
    fn info(&self) -> Info {
        Info {
            name: "PostgreSQL startup healthcheck",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let Some(db) = Db::get_one(&rocket).await else {
            eprintln!(
                "FATAL: PostgreSQL pool 'campus_lost' is unavailable; \
                 Rocket startup aborted."
            );
            return Err(rocket);
        };

        let result = db
            .run(|conn| {
                sql_query("SELECT 1 AS _result")
                    .get_result::<HealthcheckRow>(conn)
            })
            .await;

        match result {
            Ok(_) => {
                println!("PostgreSQL startup healthcheck passed.");
                Ok(rocket)
            }
            Err(error) => {
                eprintln!(
                    "FATAL: PostgreSQL startup healthcheck failed: {error}. \
                     Rocket startup aborted."
                );
                Err(rocket)
            }
        }
    }
}