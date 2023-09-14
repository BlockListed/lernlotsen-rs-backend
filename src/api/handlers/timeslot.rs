use std::collections::BTreeMap;
use std::fmt::Write;
use std::ops::Range;

use anyhow::Context;
use axum::http::StatusCode;
use chrono::{Datelike, IsoWeek, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;
use futures_util::stream::{FuturesOrdered, FuturesUnordered};
use futures_util::{FutureExt, StreamExt};
use mongodb::Database;
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::api::logic::check_object_belong_to_userid;
use crate::api::logic::entry::get_time_from_index_and_timeslot;
use crate::api::logic::export::format_entry;
use crate::api::logic::timeslot::get_index_range_timeslot;
use crate::api::util::prelude::*;
use crate::auth::UserId;
use crate::db::model::{BsonTimeSlot, Entry, HasUserId, Student, TimeSlot};
use crate::db::queries::entry::get_entry_by_index_range;
use crate::db::queries::timeslot::{get_timeslot_by_id, get_timeslots, insert_timeslot};
use crate::util::create_isoweek;

#[derive(Deserialize, Debug)]
pub struct TimeSlotQuery {
	id: Option<Uuid>,
}

pub async fn query(u: UserId, db: Database, q: TimeSlotQuery) -> anyhow::Result<Vec<TimeSlot>> {
	let res = match q.id {
		Some(id) => {
			let mut output = Vec::new();

			if let Some(ts) = get_timeslot_by_id(db, u, id).await? {
				output.push(ts)
			}

			output
		}
		None => get_timeslots(db, u).await?,
	};

	Ok(res)
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

pub async fn create(
	u: UserId,
	db: Database,
	r: TimeslotCreate,
) -> anyhow::Result<WebResult<Uuid, &'static str>> {
	if r.timerange.start.weekday() != r.weekday {
		return Ok(NotFine(
			StatusCode::UNPROCESSABLE_ENTITY,
			"timerange start is the first day.",
		));
	}

	if r.timerange.start > r.timerange.end {
		return Ok(NotFine(
			StatusCode::UNPROCESSABLE_ENTITY,
			"timerange start should be before end",
		));
	}

	if r.time.start > r.time.end {
		return Ok(NotFine(
			StatusCode::UNPROCESSABLE_ENTITY,
			"start time should be before end",
		));
	}

	let id = Uuid::new_v4();
	let ts = BsonTimeSlot {
		user_id: u.0.clone(),
		id: id.into(),
		subject: r.subject,
		students: r.students,
		time: r.time,
		timerange: r.timerange,
		weekday: r.weekday,
		timezone: r.timezone,
	};

	insert_timeslot(db, ts).await?;

	Ok(Fine(StatusCode::CREATED, id))
}

#[derive(Deserialize)]
pub struct ExportRequest {
	start_year: i32,
	start_week: u32,
	end_year: i32,
	end_week: u32,
	#[allow(dead_code)]
	#[serde(default)]
	allow_incomplete_week: bool,
}

pub async fn export(
	u: UserId,
	db: Database,
	q: ExportRequest,
) -> anyhow::Result<WebResult<String, &'static str>> {
	let Some(start) = create_isoweek(q.start_year, q.start_week) else {
		return Ok(WebResult::NotFine(StatusCode::UNPROCESSABLE_ENTITY, "invalid week/year"));
	};

	let Some(end) = create_isoweek(q.end_year, q.end_week) else {
		return Ok(WebResult::NotFine(StatusCode::UNPROCESSABLE_ENTITY, "invalid week/year"));
	};

	let mut user_timeslots = get_timeslots(db.clone(), u.clone()).await?;

	// Make sure we list timeslots in order in export
	user_timeslots.sort_by(|a, b| {
		a.timerange
			.start
			.weekday()
			.num_days_from_monday()
			.cmp(&b.timerange.start.weekday().num_days_from_monday())
			.then(a.time.start.cmp(&b.time.start))
	});

	if let WebResult::NotFine(c, e) = check_object_belong_to_userid(user_timeslots.iter(), &u) {
		return Ok(WebResult::NotFine(c, e));
	}

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
				timeslot_handles.push(tokio::spawn(
					get_entry_by_index_range(db_task, u_task, i.1.id, r)
						.map(move |res| res.map(|e| (e, i.1, expected_entry_count))),
				));
			}
			None => {
				let id = i.1.id;
				debug!(ts=%id, ?start, ?end, "timerange invalid for timeslot");
			},
		}
	}

	let entry_results = FuturesOrdered::from_iter(timeslot_handles.into_iter())
		.collect::<Vec<_>>()
		.await;

	// BtreeMap, because we need ordering
	// TODO: Vec<Student> should probably be Arc<[Student]> to save allocations.
	let mut week_map: BTreeMap<IsoWeek, Vec<(Entry, Vec<Student>)>> = BTreeMap::new();

	for res in entry_results {
		let (entries, ts, expected_count) = res.unwrap()?;

		if (entries.len() as u32) < expected_count {
			return Ok(NotFine(
				StatusCode::PRECONDITION_REQUIRED,
				"no entries in range can be missing",
			));
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

	let mut output = String::new();

	for (w, entries) in week_map.iter() {
		writeln!(output, "KW{}", w.week())?;
		for (e, students) in entries {
			writeln!(output, "{}", format_entry(&e.state, students))?;
		}
	}

	Ok(WebResult::Fine(StatusCode::OK, output))
}
