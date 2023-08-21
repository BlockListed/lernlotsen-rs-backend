use std::ops::Range;

use axum::extract::{Json, Query, State};
use axum::http::StatusCode;

use bson::Uuid as BsonUuid;

use chrono::{NaiveDate, NaiveTime, Weekday, Datelike};

use futures_util::StreamExt;
use mongodb::Database;
use serde::{Deserialize, Serialize};

use tracing::error;
use uuid::Uuid;

use crate::db::collection_timeslots;
use crate::db::model::{BsonTimeSlot, Student, TimeSlot};

use crate::api::util::prelude::*;

#[derive(Deserialize, Debug)]
pub struct TimeslotQuery {
	id: Option<Uuid>,
}

pub async fn query(
	State(db): State<Database>,
	Query(q): Query<TimeslotQuery>,
) -> WebResult<Vec<TimeSlot>, &'static str> {
	let query = q.id.map(|x| {
		bson::doc! {
			"id": BsonUuid::from_uuid_1(x),
		}
	});

	spawn_in_current_span(async move {
		let collection = collection_timeslots(&db).await;

		let r = crate::handle_db!(collection.find(query, None).await, "database error");

		let ret = r.filter_map(|i| async {
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

		Fine(StatusCode::OK, ret)
	})
	.await
	.unwrap()
}

#[derive(Deserialize, Debug)]
pub struct TimeslotCreate {
	students: Vec<Student>,
	weekday: Weekday,
	time: Range<NaiveTime>,
	timerange: Range<NaiveDate>,
}

#[derive(Serialize)]
pub struct TimeslotCreateReturn {
	id: Uuid,
}

pub async fn create(
	State(db): State<Database>,
	Json(r): Json<TimeslotCreate>,
) -> WebResult<TimeslotCreateReturn, &'static str> {
	spawn_in_current_span(async move {
		if r.timerange.start.weekday() != r.weekday {
			return NotFine(StatusCode::UNPROCESSABLE_ENTITY, "timerange start is the first day.");
		}
	
		let id = Uuid::new_v4();
		let ts = BsonTimeSlot {
			id: id.into(),
			students: r.students,
			time: r.time,
			timerange: r.timerange,
			weekday: r.weekday,
		};

		let collection = collection_timeslots(&db).await;

		crate::handle_db!(collection.insert_one(ts, None).await, "database error");

		Fine(StatusCode::CREATED, TimeslotCreateReturn { id })
	})
	.await
	.unwrap()
}
