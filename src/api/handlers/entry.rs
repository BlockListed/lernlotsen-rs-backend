use anyhow::Context;
use axum::http::StatusCode;
use mongodb::Database;
use tracing::{debug, error};
use uuid::Uuid;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::UserId;
use crate::api::logic::entries::{
	get_time_from_index_and_timeslot, missing_entries, next_entry_date_timeslot, verify_state,
};
use crate::api::util::WebResult;
use crate::db::queries::entry::{get_entries_by_timeslot_id, insert_entry, InsertEntryError};
use crate::db::queries::timeslot::get_timeslot_by_id;

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
	let timeslot: TimeSlot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await?
	{
		Some(x) => x.into(),
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				"timeslot not found",
			))
		}
	};

	let res: Vec<_> = get_entries_by_timeslot_id(db, u, timeslot.id).await?
		.drain(..)
		.filter_map(|v| {
			let entry: Entry = v.into();
			let Some(time) = get_time_from_index_and_timeslot(&timeslot, entry.index) else {
				error!(timeslot=%entry.timeslot_id, index=%entry.index, "date of entry in database overflows the chrono limits");
				return None;
			};

			Some((entry, time.to_rfc3339()))
		})
		.collect::<Vec<_>>();

	Ok(WebResult::Fine(StatusCode::OK, res))
}

pub async fn missing(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<WebResult<Vec<(u32, String)>, &'static str>> {
	let timeslot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await?
	{
		Some(v) => v,
		None => {
			return Ok(WebResult::NotFine(
				StatusCode::NOT_FOUND,
				"timeslot not found",
			));
		}
	};

	Ok(WebResult::Fine(StatusCode::OK, missing_entries(db, u, timeslot.into()).await?))
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
	let selected_timeslot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await?
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
			timeslot_id: selected_timeslot.id.into(),
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

	match insert_entry(db, entry).await {
		Ok(_) => (),
		Err(e) => match e {
			InsertEntryError::Duplicate => {
				return Ok(WebResult::Fine(StatusCode::CONFLICT, "duplicate index"));
			}
			InsertEntryError::Other(e) => {
				Err(e)?;
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
	let ts = match get_timeslot_by_id(db, u, q.id).await?
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
