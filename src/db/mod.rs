use std::{time::Duration, str::FromStr};

use sqlx::{PgPool, postgres::{PgConnectOptions, PgPoolOptions}};

use crate::configuration::Config;

pub mod model;
pub mod queries;


pub async fn get_pool(cfg: &Config) -> PgPool {
	let pg_options = PgConnectOptions::from_str(&cfg.database.uri)
		.expect("Invalid database URI")
		.application_name("lelo_backend");

	let pool_options = PgPoolOptions::new()
		.max_connections(5)
		.min_connections(1)
		.max_lifetime(Duration::from_secs(3600))
		.acquire_timeout(Duration::from_secs(30));

	let pool = pool_options.connect_with(pg_options).await.expect("Couldn not connect to db!");

	sqlx::migrate!()
		.run(&pool)
		.await
		.expect("Couldn't complete migrations");

	pool
}
