use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Extension;

use chrono::{DateTime, FixedOffset};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use uuid::Uuid;

use tracing::{debug, error};

use crate::api::logic::check_object_belong_to_userid;
use crate::api::logic::entry::{get_time_from_index_and_timeslot, missing_entries, verify_state};
use crate::api::util::prelude::*;
use crate::api::AppState;
use crate::auth::UserId;
use crate::db::model::{Entry, EntryState, Student, StudentState, WebEntry, WebTimeSlot};
use crate::db::queries::entry::{
	delete_entry_by_id, get_entries_by_timeslot_id, insert_entry, InsertEntryError,
};
use crate::db::queries::timeslot::get_timeslot_by_id;

use super::logic::entry::next_entry_timeslot;

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	state: EntryState,
	students: Vec<StudentState>,
	index: u32,
}

pub enum CreateEntryError {
	TimeslotNotFound,
	InvalidStudents(Vec<Student>),
	DuplicateIndex,
}

impl From<CreateEntryError> for WebError<Value> {
	fn from(v: CreateEntryError) -> WebError<Value> {
		use CreateEntryError::*;
		match v {
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
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<MissingQuery>,
	Extension(u): Extension<UserId>,
	Json(r): Json<CreateEntry>,
) -> WebResult<&'static str, Value> {
	let selected_timeslot = match get_timeslot_by_id(&db, &u, q.id).await? {
		Some(x) => x,
		None => {
			return Err(CreateEntryError::TimeslotNotFound)?;
		}
	};

	// Technically, not needed since, we have a constraint, that timeslot_id and user_id must match a single timeslot entry.
	(|| {
		anyhow::ensure!(
			selected_timeslot.user_id == u.as_str(),
			"timeslot user_id is not equal to clients user_id"
		);
		Ok(())
	})()?;

	let entry = match verify_state(r.state, &r.students, &selected_timeslot.students) {
		Ok(()) => Entry {
			user_id: u.as_str().to_owned(),
			index: r.index.try_into()?,
			timeslot_id: selected_timeslot.id,
			state_enum: r.state,
			students: r.students,
		},
		Err(s) => {
			debug!("request contained invalid students");
			return Err(CreateEntryError::InvalidStudents(s))?;
		}
	};

	match insert_entry(&db, entry).await {
		Ok(()) => (),
		Err(e) => match e {
			InsertEntryError::Duplicate => {
				return Err(CreateEntryError::DuplicateIndex)?;
			}
			InsertEntryError::Other(e) => {
				Err(e)?;
			}
		},
	};

	Ok((StatusCode::CREATED, "success").into())
}

#[derive(Deserialize)]
pub struct EntryQuery {
	pub id: Uuid,
}

pub enum TimeslotQueryError {
	TimeslotNotFound,
}

#[allow(clippy::from_over_into)]
impl From<TimeslotQueryError> for WebError<&'static str> {
	fn from(v: TimeslotQueryError) -> WebError<&'static str> {
		use TimeslotQueryError::*;
		match v {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

#[derive(Serialize)]
pub struct QueryReturn {
	pub entry: WebEntry,
	pub timestamp: DateTime<FixedOffset>,
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<EntryQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<QueryReturn>, &'static str> {
	let timeslot: WebTimeSlot = match get_timeslot_by_id(&db, &u, q.id).await? {
		Some(x) => x,
		None => return Err(TimeslotQueryError::TimeslotNotFound)?,
	};

	let mut res: Vec<_> = get_entries_by_timeslot_id(&db, &u, timeslot.id).await?
		.into_iter()
		.filter_map(|entry| {
			let Some(timestamp) = get_time_from_index_and_timeslot(&timeslot, entry.index).map(|v| v.fixed_offset()) else {
				error!(timeslot=%entry.timeslot_id, index=%entry.index, "date of entry in database overflows the chrono limits");
				return None;
			};

			Some(QueryReturn { entry, timestamp })
		})
		.collect::<Vec<_>>();

	res.sort_unstable_by(|a, b| b.entry.index.cmp(&a.entry.index));

	check_object_belong_to_userid(res.iter().map(|v| &v.entry), &u)?;

	Ok(res.into())
}

#[derive(Deserialize)]
pub struct MissingQuery {
	pub id: Uuid,
}

pub enum MissingEntriesError {
	TimeslotNotFound,
}

impl From<MissingEntriesError> for WebError<&'static str> {
	fn from(v: MissingEntriesError) -> WebError<&'static str> {
		use MissingEntriesError::*;
		match v {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

// TODO: define this somewhere like model or together with the logic returning this
#[derive(Serialize)]
pub struct UnfilledEntry {
	pub index: u32,
	pub timestamp: DateTime<FixedOffset>,
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<MissingQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<UnfilledEntry>, &'static str> {
	let timeslot = match get_timeslot_by_id(&db, &u, q.id).await? {
		Some(v) => v,
		None => return Err(MissingEntriesError::TimeslotNotFound)?,
	};

	Ok(missing_entries(&db, &u, &timeslot).await?.into())
}

#[derive(Deserialize)]
pub struct NextQuery {
	pub id: Uuid,
}

pub enum NextEntryError {
	TimeslotNotFound,
}

impl From<NextEntryError> for WebError<&'static str> {
	fn from(v: NextEntryError) -> WebError<&'static str> {
		use NextEntryError::*;
		match v {
			TimeslotNotFound => (StatusCode::NOT_FOUND, "timeslot not found").into(),
		}
	}
}

pub async fn next(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<NextQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<UnfilledEntry, &'static str> {
	let ts = match get_timeslot_by_id(&db, &u, q.id).await? {
		Some(d) => d,
		None => return Err(NextEntryError::TimeslotNotFound)?,
	};

	Ok(next_entry_timeslot(&ts)?.into())
}

#[derive(Deserialize)]
pub struct DeleteQuery {
	pub id: Uuid,
	pub index: u32,
}

pub enum DeleteError {}

impl From<DeleteError> for WebError<&'static str> {
	fn from(_: DeleteError) -> WebError<&'static str> {
		(StatusCode::INTERNAL_SERVER_ERROR, "unknown server error").into()
	}
}

pub async fn delete(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<DeleteQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<&'static str, &'static str> {
	delete_entry_by_id(&db, &u, q.id, q.index.try_into()?).await?;

	Ok("success".into())
}
