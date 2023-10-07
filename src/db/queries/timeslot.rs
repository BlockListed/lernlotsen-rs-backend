use bson::doc;
use futures_util::{FutureExt, StreamExt};
use mongodb::{Client, Database};
use tokio::spawn;
use tracing::error;
use uuid::Uuid;

use crate::{
	auth::UserId,
	db::{
		self, collection_entries, collection_timeslots,
		model::{BsonTimeSlot, TimeSlot},
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

pub async fn delete_timeslot_by_id(c: Client, u: UserId, id: Uuid) -> anyhow::Result<()> {
	let timeslot_query = bson::doc! {
		"user_id": u.as_str(),
		"id": id,
	};

	let entry_query = bson::doc! {
		"user_id": u.as_str(),
		"timeslot_id": id,
	};

	tokio::spawn(async move {
		let mut session = c.start_session(None).await?;

		let db = db::get_db(&c);

		let timeslots = collection_timeslots(&db).await;
		let entries = collection_entries(&db).await;

		session
			.with_transaction(
				(timeslots, entries, timeslot_query, entry_query),
				|session, (ts, en, ts_q, en_q)| {
					async move {
						en.delete_many_with_session(en_q.clone(), None, session)
							.await?;

						ts.delete_one_with_session(ts_q.clone(), None, session)
							.await?;

						mongodb::error::Result::Ok(())
					}
					.boxed()
				},
				None,
			)
			.await?;

		Ok(())
	})
	.await
	.unwrap()
}
