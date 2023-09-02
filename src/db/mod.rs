use std::time::Duration;

use mongodb::{options::ClientOptions, Client, Collection, Database};

use crate::configuration::Config;

pub mod model;

mod migrate;

use model::{BsonEntry, BsonTimeSlot};

pub async fn get_db(cfg: &Config) -> Database {
	let mut opts = ClientOptions::parse(&cfg.database.uri).await.unwrap();

	opts.app_name = Some("Lernlotsen".to_string());

	opts.connect_timeout = Some(Duration::from_secs(2));
	opts.max_idle_time = Some(Duration::from_secs(300));

	let client = Client::with_options(opts).unwrap();
	let database = client.database(&cfg.database.database);

	migrate::migrate(&database).await;

	database
}

pub async fn collection_timeslots(db: &Database) -> Collection<BsonTimeSlot> {
	db.collection("timeslots")
}

pub async fn collection_entries(db: &Database) -> Collection<BsonEntry> {
	db.collection("entries")
}
