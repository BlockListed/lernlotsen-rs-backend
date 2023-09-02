use std::time::Duration;

use mongodb::{options::ClientOptions, Client, Collection, Database, IndexModel};

use crate::configuration::Config;

pub mod model;

use model::{BsonEntry, BsonTimeSlot};

pub async fn get_db(cfg: &Config) -> Database {
	let mut opts = ClientOptions::parse(&cfg.database.uri).await.unwrap();

	opts.app_name = Some("Lernlotsen".to_string());

	opts.connect_timeout = Some(Duration::from_secs(2));
	opts.max_idle_time = Some(Duration::from_secs(300));

	let client = Client::with_options(opts).unwrap();
	let database = client.database(&cfg.database.database);

	migrate(&database).await;

	database
}

async fn migrate(db: &Database) {
	let timeslots = collection_timeslots(db).await;

	let timeslots_id_index = IndexModel::builder()
		.keys(bson::doc! {
			"user_Id": 1,
			"id": 1,
		})
		.options(
			mongodb::options::IndexOptions::builder()
				.unique(Some(true))
				.build(),
		)
		.build();

	timeslots
		.create_index(timeslots_id_index, None)
		.await
		.unwrap();

	let entries = collection_entries(db).await;

	let entries_index_index = IndexModel::builder()
		.keys(bson::doc! {
			"timeslot_id": 1,
			"index": 1,
		})
		.options(
			mongodb::options::IndexOptions::builder()
				.unique(Some(true))
				.build(),
		)
		.build();

	entries
		.create_index(entries_index_index, None)
		.await
		.unwrap();
}

pub async fn collection_timeslots(db: &Database) -> Collection<BsonTimeSlot> {
	db.collection("timeslots")
}

pub async fn collection_entries(db: &Database) -> Collection<BsonEntry> {
	db.collection("entries")
}
