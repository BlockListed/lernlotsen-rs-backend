use bson::doc;
use futures_util::StreamExt;
use mongodb::Database;
use tokio::spawn;
use tracing::error;
use uuid::Uuid;

use crate::{
	auth::UserId,
	db::{
		collection_timeslots,
		model::{BsonTimeSlot, TimeSlot}, collection_entries,
	},
};

pub async fn get_timeslots(db: Database, u: UserId) -> anyhow::Result<Vec<TimeSlot>> {
	let query = doc! {
		"user_id": u.as_str(),
	};

	spawn(async move {
		let timeslots = collection_timeslots(&db).await;

		Ok(timeslots
			.find(query, None)
			.await?
			.filter_map(|i| async {
				if let Ok(x) = i {
					Some(x)
				} else {
					error!("invalid data in database");
					None
				}
			})
			.map(std::convert::Into::into)
			.collect()
			.await)
	})
	.await
	.unwrap()
}

pub async fn get_timeslot_by_id(
	db: Database,
	u: UserId,
	id: Uuid,
) -> anyhow::Result<Option<TimeSlot>> {
	let query = doc! {
		"user_id": u.as_str(),
		"id": id,
	};

	let res: anyhow::Result<Option<TimeSlot>> = spawn(async move {
		let timeslots = collection_timeslots(&db).await;

		Ok(timeslots
			.find_one(query, None)
			.await?
			.map(std::convert::Into::into))
	})
	.await
	.unwrap();

	res
}

pub async fn insert_timeslot(db: Database, ts: BsonTimeSlot) -> anyhow::Result<()> {
	tokio::spawn(async move {
		let timeslots = collection_timeslots(&db).await;

		timeslots.insert_one(ts, None).await?;

		Ok(())
	})
	.await
	.unwrap()
}

pub async fn delete_timeslot_by_id(db: Database, u: UserId, id: Uuid) -> anyhow::Result<()> {
	let timeslot_query = bson::doc! {
		"user_id": u.as_str(),
		"id": id,
	};

	let entry_query = bson::doc! {
		"user_id": u.as_str(),
		"timeslot_id": id,
	};

	tokio::spawn(async move {
		let timeslots = collection_timeslots(&db).await;
		let entries = collection_entries(&db).await;

		let (timeslot_res, entry_res) = tokio::join!(
			timeslots.delete_one(timeslot_query, None),
			entries.delete_many(entry_query, None),
		);

		timeslot_res?;
		entry_res?;

		Ok(())
	})
	.await
	.unwrap()
}