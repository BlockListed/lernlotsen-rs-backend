#![allow(dead_code)]
use mongodb::{Database, IndexModel};

use super::{collection_timeslots, collection_entries};

pub async fn migrate(db: &Database) {
	up_timeslot_index_2023_09_02_12_39_22_00_00(db).await;
	up_entries_index_2023_09_02_12_43_36_00_00(db).await;
}

// Get timestamps for migrations using `date -u -Iseconds | sed -E "s/(:|-|\+|T)/_/g"`

async fn up_timeslot_index_2023_09_02_12_39_22_00_00(db: &Database) {
	let timeslots = collection_timeslots(db).await;

	let timeslots_id_index = IndexModel::builder()
		.keys(bson::doc! {
			"user_id": 1,
			"id": 1,
		})
		.options(
			mongodb::options::IndexOptions::builder()
				.name("timeslots_id_2023_09_02_12_39_22_00_00".to_string())
				.unique(Some(true))
				.build(),
		)
		.build();

	timeslots
		.create_index(timeslots_id_index, None)
		.await
		.unwrap();
}

async fn down_timeslot_index_2023_09_02_12_39_22_00_00(db: &Database) {
	let timeslots = collection_timeslots(db).await;

	timeslots.drop_index("timeslots_id_2023_09_02_12_39_22_00_00", None).await.unwrap();
}

async fn up_entries_index_2023_09_02_12_43_36_00_00(db: &Database) {
	let entries = collection_entries(db).await;

	let entries_index_index = IndexModel::builder()
		.keys(bson::doc! {
			"timeslot_id": 1,
			"index": 1,
		})
		.options(
			mongodb::options::IndexOptions::builder()
				.name("entries_id_index_2023_09_02_12_43_36_00_00".to_string())
				.unique(Some(true))
				.build(),
		)
		.build();

	entries
		.create_index(entries_index_index, None)
		.await
		.unwrap();
}

async fn down_entries_index_2023_09_02_12_43_36_00_00(db: &Database) {
	let entries = collection_entries(db).await;

	entries.drop_index("entries_id_index_2023_09_02_12_43_36_00_00", None).await.unwrap();
}