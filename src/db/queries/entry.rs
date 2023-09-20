use std::ops::Range;

use bson::doc;
use futures_util::StreamExt;
use mongodb::Database;
use tokio::spawn;
use tracing::{debug, error};
use uuid::Uuid;

use crate::db::model::{BsonEntry, Entry};
use crate::{auth::UserId, db::collection_entries};

pub async fn get_entries_by_timeslot_id(
	db: Database,
	u: UserId,
	id: uuid::Uuid,
) -> anyhow::Result<Vec<Entry>> {
	let query = doc! {
		"timeslot_id": id,
		"user_id": u.as_str(),
	};

	spawn(async move {
		let entries = collection_entries(&db).await;

		Ok(entries
			.find(query, None)
			.await?
			.filter_map(|v| async {
				match v {
					Ok(x) => Some(x),
					Err(e) => {
						error!(%e, "invalid data in database");
						None
					}
				}
			})
			.map(std::convert::Into::into)
			.collect()
			.await)
	})
	.await
	.unwrap()
}

pub async fn get_entries_with_index_in(
	db: Database,
	u: UserId,
	timeslot_id: Uuid,
	indexes: Vec<u32>,
) -> anyhow::Result<Vec<Entry>> {
	let query = doc! {
		"timeslot_id": timeslot_id,
		"user_id": u.as_str(),
		"index": {
			"$in": indexes,
		}
	};

	spawn(async move {
		let entries = collection_entries(&db).await;

		Ok(entries
			.find(query, None)
			.await?
			.filter_map(|v| async {
				match v {
					Ok(x) => Some(x),
					Err(e) => {
						error!(%e, "invalid data in database");
						None
					}
				}
			})
			.map(std::convert::Into::into)
			.collect()
			.await)
	})
	.await
	.unwrap()
}

#[derive(thiserror::Error, Debug)]
pub enum InsertEntryError {
	#[error("duplicate index")]
	Duplicate,
	#[error("internal server error")]
	Other(#[from] anyhow::Error),
}

pub async fn insert_entry(db: Database, entry: BsonEntry) -> Result<(), InsertEntryError> {
	let res = spawn(async move {
		let entries = collection_entries(&db).await;

		entries.insert_one(entry, None).await
	})
	.await
	.unwrap();

	if let Err(e) = res {
		use mongodb::error::{ErrorKind, WriteError, WriteFailure};

		match *e.kind {
			ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) => {
				debug!("Duplicated entry.");

				return Err(InsertEntryError::Duplicate);
			}
			_ => {
				let anyerr: anyhow::Error = e.into();
				return Err(anyerr)?;
			}
		}
	};

	Ok(())
}

pub async fn get_entry_by_index_range(
	db: Database,
	u: UserId,
	id: uuid::Uuid,
	index_range: Range<u32>,
) -> anyhow::Result<Vec<Entry>> {
	let query = doc! {
		"user_id": u.as_str(),
		"timeslot_id": id,
		"index": {
			"$gte": index_range.start,
			"$lte": index_range.end,
		}
	};

	spawn(async move {
		let entries = collection_entries(&db).await;

		Ok(entries
			.find(query, None)
			.await?
			.filter_map(|v| async {
				if let Ok(entry) = v {
					Some(entry.into())
				} else {
					None
				}
			})
			.collect()
			.await)
	})
	.await
	.unwrap()
}
