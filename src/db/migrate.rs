#![allow(dead_code)]
// Yes, this is insane.
// Yes, this probably breaks, if multiple services try to run migrations at the same time.
// Yes, I hate this.

use mongodb::{
	options::{IndexOptions, UpdateOptions},
	Database, IndexModel,
};

use tracing::info;

use super::{collection_config, collection_entries, collection_timeslots};

async fn get_generation(db: &Database) -> u32 {
	let config = collection_config(db).await;

	let generation: u32 = config
		.find_one(
			bson::doc! {
				"key": "migration_generation",
			},
			None,
		)
		.await
		.unwrap()
		.map(|v| v.value.parse().unwrap())
		.unwrap_or(0);

	generation
}

async fn set_generation(db: &Database, generation: u32) {
	let config = collection_config(db).await;

	config
		.update_one(
			bson::doc! {
				"key": "migration_generation",
			},
			bson::doc! {
				"$set": {
					"value": generation.to_string(),
				},
			},
			UpdateOptions::builder().upsert(true).build(),
		)
		.await
		.unwrap();
}

pub async fn migrate(db: &Database) {
	let mut generation = get_generation(db).await;

	const FINAL_GENERATION: u32 = 4;

	while generation <= FINAL_GENERATION {
		info!(generation, "starting migration generation");
		match generation {
			0 => up_timeslot_index_2023_09_02_12_39_22_00_00(db).await,
			1 => up_entries_index_2023_09_02_12_43_36_00_00(db).await,
			2 => up_add_timezone_field_2023_09_02_12_49_42_00_00(db).await,
			3 => down_entries_index_2023_09_02_12_43_36_00_00(db).await,
			4 => up_entries_userid_index_2023_09_03_07_19_54_00_00(db).await,
			_ => unreachable!(),
		};

		let generation_changed = generation != get_generation(db).await;

		if generation_changed {
			panic!("Someone else is also running migrations, letting them finish, hopefully we get restarted.");
		}

		generation += 1;

		set_generation(db, generation).await;
	}

	info!(generation, "completed migrations");
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

	timeslots
		.drop_index("timeslots_id_2023_09_02_12_39_22_00_00", None)
		.await
		.unwrap();
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

	entries
		.drop_index("entries_id_index_2023_09_02_12_43_36_00_00", None)
		.await
		.unwrap();
}

async fn up_add_timezone_field_2023_09_02_12_49_42_00_00(db: &Database) {
	let timeslots = collection_timeslots(db).await;

	timeslots
		.update_many(
			bson::doc! {
				"timezone": { "$exists": false },
			},
			bson::doc! {
				"$set": {
					"timezone": "Europe/Berlin",
				}
			},
			None,
		)
		.await
		.unwrap();
}

// Probably useless but I dont care
async fn down_add_timezone_field_2023_09_02_12_49_42_00_00(db: &Database) {
	let timeslots = collection_timeslots(db).await;

	timeslots
		.update_many(
			bson::doc! {
				"timezone": { "$exists": true },
			},
			bson::doc! {
				"$unset": {
					"timezone": "",
				}
			},
			None,
		)
		.await
		.unwrap();
}

async fn up_entries_userid_index_2023_09_03_07_19_54_00_00(db: &Database) {
	let entries = collection_entries(db).await;

	let entries_userid_index_index = IndexModel::builder()
		.keys(bson::doc! {
			"timeslot_id": 1,
			"user_id": 1,
			"index": 1,
		})
		.options(
			IndexOptions::builder()
				.name("entries_id_userid_index_2023_09_03_07_19_54_00_00".to_string())
				.unique(true)
				.build(),
		)
		.build();

	entries
		.create_index(entries_userid_index_index, None)
		.await
		.unwrap();
}

async fn down_entries_userid_index_2023_09_03_07_19_54_00_00(db: &Database) {
	let entries = collection_entries(db).await;

	entries
		.drop_index("entries_id_userid_index_2023_09_03_07_19_54_00_00", None)
		.await
		.unwrap();
}
