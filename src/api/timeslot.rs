use std::ops::Range;

use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::Extension;

use bson::Uuid as BsonUuid;

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use tracing::error;
use uuid::Uuid;

use crate::db::collection_timeslots;
use crate::db::model::{BsonTimeSlot, Student, TimeSlot};

use crate::api::util::prelude::*;

use super::auth::UserId;
use super::logic::check_timeslots_belong_to_userid;
use super::AppState;

#[derive(Deserialize, Debug)]
pub struct TimeslotQuery {
	id: Option<Uuid>,
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Extension(UserId(t)): Extension<UserId>,
	Query(q): Query<TimeslotQuery>,
) -> WebResult<Vec<TimeSlot>, &'static str> {
	let query = match q.id {
		Some(u) => {
			bson::doc! {
				"user_id": &t,
				"id": BsonUuid::from_uuid_1(u),
			}
		}
		None => {
			bson::doc! {
				"user_id": &t,
			}
		}
	};

	spawn_in_current_span(async move {
		let collection = collection_timeslots(&db).await;

		let r = crate::handle_db!(collection.find(query, None).await, "database error");

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

		match check_timeslots_belong_to_userid(ret.iter(), &t) {
			Fine(..) => (),
			NotFine(c, e) => return NotFine(c, e),
		};

		Fine(StatusCode::OK, ret)
	})
	.await
	.unwrap()
}

#[derive(Deserialize, Debug)]
pub struct TimeslotCreate {
	students: Vec<Student>,
	subject: String,
	weekday: Weekday,
	time: Range<NaiveTime>,
	timerange: Range<NaiveDate>,
}

#[derive(Serialize)]
pub struct TimeslotCreateReturn {
	id: Uuid,
}

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Extension(UserId(t)): Extension<UserId>,
	Json(r): Json<TimeslotCreate>,
) -> WebResult<TimeslotCreateReturn, &'static str> {
	spawn_in_current_span(async move {
		if r.timerange.start.weekday() != r.weekday {
			return NotFine(
				StatusCode::UNPROCESSABLE_ENTITY,
				"timerange start is the first day.",
			);
		}

		let id = Uuid::new_v4();
		let ts = BsonTimeSlot {
			user_id: t,
			id: id.into(),
			subject: r.subject,
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
