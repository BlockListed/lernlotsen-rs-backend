use std::ops::Range;

use axum::extract::{Json, Query, State};
use axum::http::StatusCode;

use bson::Uuid as BsonUuid;

use chrono::{DateTime, NaiveTime, Utc, Weekday};

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

		let r = collection.find(query, None).await;

		let ret = match r {
			Ok(x) => {
				x.filter_map(|i| async {
					if let Ok(x) = i {
						Some(x)
					} else {
						error!("invalid data in database");
						None
					}
				})
				.map(|v| v.into())
				.collect::<Vec<_>>()
				.await
			}
			Err(e) => {
				error!(%e, "encountered database error");

				return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
			}
		};

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
	timerange: Range<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct TimeslotCreateReturn {
	id: Uuid,
}

pub async fn create(
	State(db): State<Database>,
	Json(r): Json<TimeslotCreate>,
) -> WebResult<TimeslotCreateReturn, &'static str> {
	let id = Uuid::new_v4();
	let ts = BsonTimeSlot {
		id: id.into(),
		students: r.students,
		weekday: r.weekday,
		time: r.time,
		timerange: Range {
			start: r.timerange.start.into(),
			end: r.timerange.end.into(),
		},
	};

	let r = spawn_in_current_span(async move {
		let collection = collection_timeslots(&db).await;

		collection.insert_one(ts, None).await
	})
	.await
	.unwrap();

	match r {
		Ok(_) => Fine(StatusCode::OK, TimeslotCreateReturn { id }),
		Err(e) => {
			error!(%e, "encountered database error");

			NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error")
		}
	}
}
