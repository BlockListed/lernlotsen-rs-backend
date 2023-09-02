use anyhow::Context;
use axum::http::StatusCode;
use futures_util::StreamExt;
use mongodb::Database;
use tracing::{debug, error};
use uuid::Uuid;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::auth::UserId;
use crate::api::logic::entries::{
	get_time_from_index_and_timeslot, missing_entries, next_entry_date_timeslot, verify_state,
};
use crate::api::util::WebResult;
use crate::db::{collection_entries, collection_timeslots};

use crate::db::model::{BsonEntry, Entry, EntryState, TimeSlot};

#[derive(Deserialize)]
pub struct TimeSlotQuery {
	pub id: Uuid,
}

pub async fn query(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<WebResult<Vec<(Entry, String)>, &'static str>> {
	let timeslots = collection_timeslots(&db).await;

	let timeslot: TimeSlot = match timeslots
		.find_one(
			bson::doc! {
				"user_id": &u.0,
				"id": q.id,
			},
			None,
		)
		.await?
	{
		Some(x) => x.into(),
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				"timeslot not found",
			))
		}
	};

	let entries = collection_entries(&db).await;

	let res: Vec<_> = entries.find(bson::doc! {
			"timeslot_id": timeslot.id,
		}, None).await?
			.filter_map(|v| async {
				match v {
					Ok(x) => Some(x),
					Err(e) => {
						error!(%e, "invalid data in database");
						None
					}
				}
			})
			.filter_map(|v| async {
				let entry: Entry = v.into();
				let Some(time) = get_time_from_index_and_timeslot(&timeslot, entry.index as u64) else {
					error!(timeslot=%entry.timeslot_id, index=%entry.index, "date of entry in database overflows the chrono limits");
					return None;
				};

				Some((entry, time.to_rfc3339()))
			})
			.collect::<Vec<_>>()
			.await;

	Ok(WebResult::Fine(StatusCode::OK, res))
}

pub async fn missing(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<WebResult<Vec<(usize, String)>, &'static str>> {
	let timeslots = collection_timeslots(&db).await;

	let timeslot = match timeslots
		.find_one(
			Some(bson::doc! {
				"user_id": u.0,
				"id": q.id,
			}),
			None,
		)
		.await?
	{
		Some(v) => v,
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				"timeslot not found",
			));
		}
	};

	Ok(missing_entries(timeslot, &db).await)
}

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	state: EntryState,
	index: u32,
}

pub async fn create(
	u: UserId,
	db: Database,
	r: CreateEntry,
	q: TimeSlotQuery,
) -> anyhow::Result<WebResult<&'static str, Value>> {
	let timeslots = collection_timeslots(&db).await;

	let selected_timeslot = match timeslots
		.find_one(
			bson::doc! {
				"user_id": &u.0,
				"id": q.id,
			},
			None,
		)
		.await?
	{
		Some(x) => x,
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				json!("timeslot not found"),
			));
		}
	};

	anyhow::ensure!(
		selected_timeslot.user_id == u.0,
		"timeslot user_id is not equal to clients user_id"
	);

	let entry = match verify_state(&r.state, &selected_timeslot.students) {
		Ok(_) => BsonEntry {
			user_id: u.0,
			index: r.index,
			timeslot_id: selected_timeslot.id,
			state: r.state,
		},
		Err(s) => {
			debug!("request contained invalid students");
			return Ok(WebResult::NotFine(
				StatusCode::UNPROCESSABLE_ENTITY,
				json!({
					"invalid_students": s,
				}),
			));
		}
	};

	let entries = collection_entries(&db).await;

	if let Err(e) = entries.insert_one(entry, None).await {
		use mongodb::error::{ErrorKind, WriteError, WriteFailure};

		match *e.kind {
			ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) => {
				debug!("Duplicated entry.");

				return Ok(WebResult::NotFine(
					StatusCode::CONFLICT,
					json!("duplicate index"),
				));
			}
			_ => {
				return Err(e)?;
			}
		}
	};

	Ok(WebResult::Fine(StatusCode::CREATED, "created entry"))
}

pub async fn next(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<WebResult<(u32, String), &'static str>> {
	let timeslots = collection_timeslots(&db).await;

	let ts = match timeslots
		.find_one(
			bson::doc! {
				"user_id": &u.0,
				"id": q.id,
			},
			None,
		)
		.await?
	{
		Some(d) => d.into(),
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				"timeslot not found",
			))
		}
	};

	Ok(WebResult::Fine(
		StatusCode::OK,
		next_entry_date_timeslot(&ts)
			.map(|(i, d)| (i, d.to_rfc3339()))
			.context("timezone issue")?,
	))
}
