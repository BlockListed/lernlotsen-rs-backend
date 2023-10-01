use anyhow::Context;
use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use mongodb::Database;
use tracing::{debug, error};
use uuid::Uuid;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::logic::entry::{
	get_time_from_index_and_timeslot, missing_entries, next_entry_date_timeslot, verify_state,
};
use crate::api::util::prelude::*;
use crate::auth::UserId;
use crate::db::queries::entry::{get_entries_by_timeslot_id, insert_entry, InsertEntryError};
use crate::db::queries::timeslot::get_timeslot_by_id;

use crate::db::model::{BsonEntry, Entry, EntryState, Student, TimeSlot};

#[derive(Deserialize)]
pub struct TimeSlotQuery {
	pub id: Uuid,
}

pub enum TimeslotQueryError {
	TimeslotNotFound,
}

#[allow(clippy::from_over_into)]
impl Into<WebError<&'static str>> for TimeslotQueryError {
	fn into(self) -> WebError<&'static str> {
		use TimeslotQueryError::*;
		match self {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

pub async fn query(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<Result<Vec<(Entry, String)>, TimeslotQueryError>> {
	let timeslot: TimeSlot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await? {
		Some(x) => x,
		None => return Ok(Err(TimeslotQueryError::TimeslotNotFound)),
	};

	let mut res: Vec<_> = get_entries_by_timeslot_id(db, u, timeslot.id).await?
		.drain(..)
		.filter_map(|v| {
			let entry: Entry = v;
			let Some(time) = get_time_from_index_and_timeslot(&timeslot, entry.index) else {
				error!(timeslot=%entry.timeslot_id, index=%entry.index, "date of entry in database overflows the chrono limits");
				return None;
			};

			Some((entry, time.to_rfc3339()))
		})
		.collect::<Vec<_>>();

	res.sort_unstable_by(|a, b| b.0.index.cmp(&a.0.index));

	Ok(Ok(res))
}

pub enum MissingEntriesError {
	TimeslotNotFound,
}

#[allow(clippy::from_over_into)]
impl Into<WebError<&'static str>> for MissingEntriesError {
	fn into(self) -> WebError<&'static str> {
		use MissingEntriesError::*;
		match self {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

pub async fn missing(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<Result<Vec<(u32, String)>, MissingEntriesError>> {
	let timeslot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await? {
		Some(v) => v,
		None => {
			return Ok(Err(MissingEntriesError::TimeslotNotFound));
		}
	};

	Ok(Ok(missing_entries(db, u, timeslot).await?))
}

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	state: EntryState,
	index: u32,
}

pub enum CreateEntryError {
	TimeslotNotFound,
	InvalidStudents(Vec<Student>),
	DuplicateIndex,
}

#[allow(clippy::from_over_into)]
impl Into<WebError<Value>> for CreateEntryError {
	fn into(self) -> WebError<Value> {
		use CreateEntryError::*;
		match self {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found".into()).into(),
			InvalidStudents(s) => (
				StatusCode::UNPROCESSABLE_ENTITY,
				json!({"invalid_students": s}),
			)
				.into(),
			DuplicateIndex => (StatusCode::CONFLICT, "duplicate index".into()).into(),
		}
	}
}

pub async fn create(
	u: UserId,
	db: Database,
	r: CreateEntry,
	q: TimeSlotQuery,
) -> anyhow::Result<Result<(), CreateEntryError>> {
	let selected_timeslot = match get_timeslot_by_id(db.clone(), u.clone(), q.id).await? {
		Some(x) => x,
		None => {
			return Ok(Err(CreateEntryError::TimeslotNotFound));
		}
	};

	anyhow::ensure!(
		selected_timeslot.user_id == u.as_str(),
		"timeslot user_id is not equal to clients user_id"
	);

	let entry = match verify_state(&r.state, &selected_timeslot.students) {
		Ok(_) => BsonEntry {
			user_id: u.as_str().to_owned(),
			index: r.index,
			timeslot_id: selected_timeslot.id.into(),
			state: r.state,
		},
		Err(s) => {
			debug!("request contained invalid students");
			return Ok(Err(CreateEntryError::InvalidStudents(s)));
		}
	};

	match insert_entry(db, entry).await {
		Ok(_) => (),
		Err(e) => match e {
			InsertEntryError::Duplicate => {
				return Ok(Err(CreateEntryError::DuplicateIndex));
			}
			InsertEntryError::Other(e) => {
				Err(e)?;
			}
		},
	};

	Ok(Ok(()))
}

pub enum NextEntryError {
	TimeslotNotFound,
}

#[allow(clippy::from_over_into)]
impl Into<WebError<&'static str>> for NextEntryError {
	fn into(self) -> WebError<&'static str> {
		use NextEntryError::*;
		match self {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

pub async fn next(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<Result<(u32, DateTime<FixedOffset>), NextEntryError>> {
	let ts = match get_timeslot_by_id(db, u, q.id).await? {
		Some(d) => d,
		None => return Ok(Err(NextEntryError::TimeslotNotFound)),
	};

	Ok(Ok(next_entry_date_timeslot(&ts)
		.map(|(i, d)| (i, d.fixed_offset()))
		.context("timezone issue")?))
}
