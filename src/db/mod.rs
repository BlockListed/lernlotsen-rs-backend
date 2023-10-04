use std::time::Duration;

use mongodb::{options::ClientOptions, Client, Collection, Database};

use crate::configuration::Config;

pub mod model;
pub mod queries;

mod migrate;

use model::{BsonEntry, BsonTimeSlot};

use self::model::ConfigurationEntry;

pub async fn get_db(cfg: &Config) -> Database {
	let mut opts = ClientOptions::parse(&cfg.database.uri).await.unwrap();

	opts.app_name = Some("Lernlotsen".to_string());

	opts.connect_timeout = Some(Duration::from_secs(5));
	opts.server_selection_timeout = Some(Duration::from_secs(5));
	opts.max_idle_time = Some(Duration::from_secs(300));

	let client = Client::with_options(opts).unwrap();
	let database = client
		.default_database()
		.expect("missing default database in uri");

	migrate::migrate(&database).await;

	database
}

// I'm too lazy to change this shit.
#[allow(clippy::unused_async)]
pub async fn collection_timeslots(db: &Database) -> Collection<BsonTimeSlot> {
	db.collection("timeslots")
}

#[allow(clippy::unused_async)]
pub async fn collection_entries(db: &Database) -> Collection<BsonEntry> {
	db.collection("entries")
}

#[allow(clippy::unused_async)]
pub async fn collection_config(db: &Database) -> Collection<ConfigurationEntry> {
	db.collection("config")
}
