use std::ops::Range;

use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;
use futures_util::StreamExt;
use mongodb::Database;
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::api::auth::UserId;
use crate::api::util::prelude::*;
use crate::db::collection_timeslots;
use crate::db::model::{BsonTimeSlot, Student, TimeSlot};

#[derive(Deserialize, Debug)]
pub struct TimeSlotQuery {
	id: Option<Uuid>,
}

pub async fn query(
	user_id: UserId,
	db: Database,
	q: TimeSlotQuery,
) -> anyhow::Result<Vec<TimeSlot>> {
	let query = match q.id {
		Some(u) => {
			bson::doc! {
				"user_id": &user_id.0,
				"id": u,
			}
		}
		None => {
			bson::doc! {
				"user_id": &user_id.0,
			}
		}
	};

	let collection = collection_timeslots(&db).await;

	let r = collection.find(query, None).await?;

	let ret = r
		.filter_map(|i| async {
			if let Ok(x) = i {
				Some(x)
			} else {
				error!("invalid data in database");
				None
			}
		})
		.map(|v| v.into())
		.collect::<Vec<_>>()
		.await;

	Ok(ret)
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

	let collection = collection_timeslots(&db).await;

	collection.insert_one(ts, None).await?;

	Ok(Fine(StatusCode::CREATED, id))
}
