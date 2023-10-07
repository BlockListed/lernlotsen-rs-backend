use std::ops::Range;

use sqlx::types::Json;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use crate::auth::UserId;
use crate::db::model::{self, Entry, EntryState, WebEntry};

pub async fn get_entries_by_timeslot_id(
	db: PgPool,
	u: UserId,
	id: uuid::Uuid,
) -> anyhow::Result<Vec<WebEntry>> {
	let entries_db = sqlx::query_as!(Entry, r#"SELECT user_id, index, timeslot_id, state AS "state: Json<EntryState>" FROM entries WHERE timeslot_id = $1 AND user_id = $2"#, id, u.as_str())
		.fetch_all(&db)
		.await?;

	let entries: Vec<WebEntry> = entries_db
		.into_iter()
		.filter_map(model::convert_entry)
		.collect();

	Ok(entries)
}

pub async fn get_entries_with_index_in(
	db: PgPool,
	u: UserId,
	timeslot_id: Uuid,
	indexes: Vec<i32>,
) -> anyhow::Result<Vec<WebEntry>> {
	let entries_db = sqlx::query_as!(Entry, r#"SELECT user_id, index, timeslot_id, state AS "state: Json<EntryState>" FROM entries WHERE timeslot_id = $1 AND user_id = $2 AND index = ANY($3)"#, timeslot_id, u.as_str(), &indexes[..])
		.fetch_all(&db)
		.await?;

	let entries: Vec<WebEntry> = entries_db
		.into_iter()
		.filter_map(model::convert_entry)
		.collect();

	Ok(entries)
}

#[derive(thiserror::Error, Debug)]
pub enum InsertEntryError {
	#[error("duplicate index")]
	Duplicate,
	#[error("internal server error")]
	Other(#[from] anyhow::Error),
}

pub async fn insert_entry(db: PgPool, entry: Entry) -> Result<(), InsertEntryError> {
	let index: i32 = entry.index;

	sqlx::query!(
		"INSERT INTO entries (id, user_id, index, timeslot_id, state) VALUES ($1, $2, $3, $4, $5)",
		uuid::Uuid::new_v4(),
		entry.user_id,
		index,
		entry.timeslot_id,
		entry.state as _
	)
	.execute(&db)
	.await
	.map_err(Into::<anyhow::Error>::into)?;

	Ok(())
}

pub async fn get_entry_by_index_range(
	db: PgPool,
	u: UserId,
	id: uuid::Uuid,
	index_range: Range<i32>,
) -> anyhow::Result<Vec<WebEntry>> {
	let entries_db = sqlx::query_as!(Entry, r#"SELECT user_id, timeslot_id, index, state AS "state: Json<EntryState>" FROM entries WHERE user_id = $1 AND timeslot_id = $2 AND index >= $3 AND index <= $4"#, u.as_str(), id, index_range.start, index_range.end)
		.fetch_all(&db)
		.await?;

	let entries: Vec<WebEntry> = entries_db
		.into_iter()
		.filter_map(model::convert_entry)
		.collect();

	Ok(entries)
}
