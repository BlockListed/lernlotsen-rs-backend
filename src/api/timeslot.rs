use std::collections::BTreeMap;
use std::fmt::Write;
use std::ops::Range;

use anyhow::Context;

use axum::extract::{Json, Query, State, Path};
use axum::http::StatusCode;
use axum::Extension;

use chrono::{Weekday, NaiveTime, NaiveDate, Datelike, IsoWeek};
use chrono_tz::Tz;

use futures_util::{FutureExt, stream::{FuturesOrdered, StreamExt}};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::api::logic::timeslot::get_index_range_timeslot;
use crate::api::logic::entry::get_time_from_index_and_timeslot;
use crate::api::logic::export::format_entry;
use crate::api::logic::check_object_belong_to_userid;
use crate::api::util::{prelude::*, WebError};
use crate::auth::UserId;

use crate::db::model::{TimeSlot, WebTimeSlot, Student, DbTime, DbTimerange, HasUserId, WebEntry};
use crate::db::queries::timeslot::{delete_timeslot_by_id, get_timeslots, get_timeslot_by_id, insert_timeslot};
use crate::db::queries::entry::get_entry_by_index_range;

use crate::util::create_isoweek;

use super::AppState;

#[derive(Deserialize, Debug)]
pub struct TimeSlotQuery {
	id: Option<Uuid>,
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Query(q): Query<TimeSlotQuery>,
) -> WebResult<Vec<WebTimeSlot>, &'static str> {
	let res = match q.id {
		Some(id) => {
			let mut output = Vec::new();

			if let Some(ts) = get_timeslot_by_id(&db, &u, id).await? {
				output.push(ts);
			}

			output
		}
		None => get_timeslots(&db, &u).await?,
	};

	check_object_belong_to_userid(res.iter(), &u)?;

	Ok(res.into())
}

#[derive(Deserialize, Debug)]
pub struct TimeslotCreate {
	students: Vec<Student>,
	subject: String,
	weekday: Weekday,
	time: Range<NaiveTime>,
	timerange: Range<NaiveDate>,
	timezone: Tz,
}

pub enum TimeslotCreateError {
	TimerangeStartShouldBeWeekday,
	TimerangeStartShouldBeBeforeEnd,
	StartTimeShouldBeBeforeEndTime,
}

#[allow(clippy::from_over_into)]
impl From<TimeslotCreateError> for WebError<&'static str> {
	fn from(v: TimeslotCreateError) -> WebError<&'static str> {
		use TimeslotCreateError::*;
		match v {
			TimerangeStartShouldBeWeekday => (
				StatusCode::UNPROCESSABLE_ENTITY,
				"weekday of timerange.start should be equal to weekday",
			)
				.into(),
			TimerangeStartShouldBeBeforeEnd => (
				StatusCode::UNPROCESSABLE_ENTITY,
				"timerange.start should be before timerange.end",
			)
				.into(),
			StartTimeShouldBeBeforeEndTime => (
				StatusCode::UNPROCESSABLE_ENTITY,
				"time.start should be before time.end",
			)
				.into(),
		}
	}
}

#[derive(Serialize)]
pub struct TimeslotCreateReturn {
	id: Uuid,
}

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Json(r): Json<TimeslotCreate>,
) -> WebResult<TimeslotCreateReturn, &'static str> {
	if r.timerange.start.weekday() != r.weekday {
		return Err(TimeslotCreateError::TimerangeStartShouldBeWeekday)?;
	}

	if r.timerange.start > r.timerange.end {
		return Err(TimeslotCreateError::TimerangeStartShouldBeBeforeEnd)?;
	}

	if r.time.start > r.time.end {
		return Err(TimeslotCreateError::StartTimeShouldBeBeforeEndTime)?;
	}

	let id = Uuid::new_v4();
	let ts = TimeSlot {
		user_id: u.as_str().to_owned(),
		id,
		subject: r.subject,
		students: r.students.into_iter().map(|student| student.name).collect(),
		time: DbTime {
			beginning: r.time.start,
			finish: r.time.end,
		},
		timerange: DbTimerange {
			beginning: r.timerange.start,
			finish: r.timerange.end,
		},
		timezone: r.timezone.name().to_string(),
	};

	insert_timeslot(&db, ts).await?;

	Ok((StatusCode::CREATED, TimeslotCreateReturn { id }).into())
}

#[derive(Deserialize)]
pub struct DeleteRequest {
	pub id: Uuid,
}

pub enum DeleteError {
	NotFound,
}

impl From<DeleteError> for WebError<&'static str> {
	fn from(v: DeleteError) -> Self {
        match v {
			DeleteError::NotFound => (StatusCode::NOT_FOUND, "couldn't find timeslot to delete").into(),
		}
    }
}

pub async fn delete(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Path(r): Path<DeleteRequest>,
) -> WebResult<&'static str, &'static str> {
	if delete_timeslot_by_id(&db, &u, r.id).await? == 0 {
		Err(DeleteError::NotFound)?;
	}

	Ok("deleted".into())
}

#[derive(Deserialize)]
pub struct ExportRequest {
	start_year: i32,
	start_week: u32,
	end_year: i32,
	end_week: u32,
	#[serde(default)]
	allow_incomplete: bool,
}

pub enum ExportError {
	InvalidWeekYear,
	MissingEntries(Vec<(String, Uuid)>),
}

impl From<ExportError> for WebError<Value> {
	fn from(v: ExportError) -> WebError<Value> {
		use ExportError::*;
		match v {
			InvalidWeekYear => {
				(StatusCode::UNPROCESSABLE_ENTITY, "invalid_week/year".into()).into()
			}
			MissingEntries(entries) => {
				let entries_json: Vec<_> = entries
					.into_iter()
					.map(|(subj, id)| serde_json::json!({"subject": subj, "id": id}))
					.collect();

				(
					StatusCode::PRECONDITION_REQUIRED,
					serde_json::json!({"missing_entries": entries_json}),
				)
					.into()
			}
		}
	}
}

// TODO: some of this logic should probably be extracted
pub async fn export(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Query(q): Query<ExportRequest>,
) -> WebResult<String, Value> {
	let Some(start) = create_isoweek(q.start_year, q.start_week) else {
		return Err(ExportError::InvalidWeekYear)?;
	};

	let Some(end) = create_isoweek(q.end_year, q.end_week) else {
		return Err(ExportError::InvalidWeekYear)?;
	};

	let mut user_timeslots = get_timeslots(&db, &u).await?;

	// Make sure we list timeslots in order in export
	user_timeslots.sort_by(|a, b| {
		a.timerange
			.start
			.weekday()
			.num_days_from_monday()
			.cmp(&b.timerange.start.weekday().num_days_from_monday())
			.then(a.time.start.cmp(&b.time.start))
	});

	check_object_belong_to_userid(user_timeslots.iter(), &u)?;

	let index_ranges = user_timeslots
		.into_iter()
		.map(|ts| (get_index_range_timeslot(&ts, start..end), ts));

	let mut timeslot_handles = Vec::new();

	for i in index_ranges {
		match i.0 {
			Some(r) => {
				let db_task = db.clone();
				let u_task = u.clone();
				let expected_entry_count = r.end - r.start + 1;

				let r_task: Range<i32> = r.start.try_into()?..r.end.try_into()?;
				timeslot_handles.push(tokio::spawn(async move {
					get_entry_by_index_range(&db_task, u_task, i.1.id, r_task)
						.map(move |res| res.map(|e| (e, i.1, expected_entry_count)))
						.await
				}));
			}
			None => {
				warn!(ts=%i.1.id, ?start, ?end, "timerange invalid for timeslot");
			}
		}
	}

	let entry_results = timeslot_handles
		.into_iter()
		.collect::<FuturesOrdered<_>>()
		.collect::<Vec<_>>()
		.await;

	// BtreeMap, because we need ordering
	// TODO: Vec<Student> should probably be Arc<[Student]> to save allocations.
	let mut week_map: BTreeMap<IsoWeek, Vec<(WebEntry, Vec<String>)>> = BTreeMap::new();

	let mut missing_entry_errors: Option<Vec<(String, uuid::Uuid)>> = None;

	for res in entry_results {
		let (entries, ts, expected_count) = res.unwrap()?;

		let entries_len: u32 = entries.len().try_into()?;

		debug!(ts=%ts.id, entries_len, "found entries for export");

		// Entry indices are always equal to or less than to u32.
		if entries_len < expected_count {
			if q.allow_incomplete {
				let timeslot = ts.id;
				warn!(%timeslot ,"exporting incomplete timeslot");
			} else {
				match missing_entry_errors.as_mut() {
					Some(v) => v.push((ts.subject, ts.id)),
					None => missing_entry_errors = Some(vec![(ts.subject, ts.id)]),
				}
				continue;
			}
		}

		// We don't want to process anything else if a entry is missing,
		// but we do still want to check the rest of the entries.
		if missing_entry_errors.is_some() {
			continue;
		}

		for e in entries {
			let iso_week = get_time_from_index_and_timeslot(&ts, e.index)
				.context(format!(
					"unable to get time from from entry: {}",
					e.identifier()
				))?
				.iso_week();

			if let Some(week_entries) = week_map.get_mut(&iso_week) {
				week_entries.push((e, ts.students.clone()));
			} else {
				week_map.insert(iso_week, [(e, ts.students.clone())].into());
			}
		}
	}

	if let Some(e) = missing_entry_errors {
		return Err(ExportError::MissingEntries(e))?;
	}

	let mut output = String::new();

	for (w, entries) in &week_map {
		writeln!(output, "KW{}", w.week())?;
		for (e, students) in entries {
			debug!(ts=%e.timeslot_id, idx=e.index, "exporting entry");
			writeln!(output, "{}", format_entry(e.state, &e.students, students))?;
		}
	}

	Ok(output.into())
}
#[derive(Serialize)]
pub struct InformationV3ResponseItem {
	ts: WebTimeSlot,
	next: UnfilledEntry,
	missing: u32,
}

pub type InformationV3Response = Vec<InformationV3ResponseItem>;

// TODO: finish this shit after I get the entry handlers switched over
pub async fn information(
	State(state): State<AppState>,
	Extension(u): Extension<UserId>,
) -> WebResult<InformationV3Response, &'static str> {
	let db = state.db.clone();

	let timeslots = query(State(state.clone()), Extension(u.clone()), Query(TimeSlotQuery { id: None })).await?;

	let next_missing = futures_util::future::join_all(timeslots.msg.iter().map(|ts| {
		let id = ts.id;
		let u = u.clone();
		let db = db.clone();
		tokio::spawn(async move {
			let next =
				handlers::entry::next(u.clone(), db.clone(), handlers::entry::NextQuery { id });
			let missing = handlers::entry::missing(
				u.clone(),
				db.clone(),
				handlers::entry::MissingQuery { id },
			);

			tokio::join!(next, missing)
		})
		.map(Result::unwrap)
	}))
	.await;

	assert!(
		timeslots.len() == next_missing.len(),
		"length of next missing not equal to timeslots"
	);

	let res: anyhow::Result<Vec<_>> = timeslots
		.into_iter()
		.zip(next_missing.into_iter())
		.map(|(ts, (next_res, missing_res))| {
			let next = next_res.and_then(|v| match v {
				Ok(v) => Ok(v),
				Err(e) => match e {
					handlers::entry::NextEntryError::TimeslotNotFound => {
						Err(anyhow::anyhow!("Timeslot went missing during handler call"))
					}
				},
			})?;

			let missing = missing_res.and_then(|v| match v {
				Ok(v) => Ok(v.len().try_into()?),
				Err(e) => match e {
					handlers::entry::MissingEntriesError::TimeslotNotFound => {
						Err(anyhow::anyhow!("Timeslot went missing during handler call"))
					}
				},
			})?;

			anyhow::Result::<_>::Ok(InformationV3ResponseItem { ts, next, missing })
		})
		.try_collect();

	let mut res = res?;

	// I know this is horrible.
	res.sort_unstable_by_key(|v| v.next.timestamp);

	Ok(res)
}
