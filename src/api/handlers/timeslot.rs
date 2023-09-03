use std::ops::Range;

use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;
use mongodb::Database;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::UserId;
use crate::api::util::prelude::*;
use crate::db::model::{BsonTimeSlot, Student, TimeSlot};
use crate::db::queries::timeslot::{get_timeslot_by_id, get_timeslots, insert_timeslot};

#[derive(Deserialize, Debug)]
pub struct TimeSlotQuery {
	id: Option<Uuid>,
}

pub async fn query(
	u: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<Vec<TimeSlot>> {
	let res = match q.id {
		Some(id) => {
			let mut output = Vec::new();
			
			if let Some(ts) = get_timeslot_by_id(db, u, id).await? {
				output.push(ts)
			}
			
			output
		}
		None => {
			get_timeslots(db, u).await?
		}
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
