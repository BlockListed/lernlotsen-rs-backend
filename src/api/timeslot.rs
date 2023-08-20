use std::ops::Range;

use axum::extract::{Query, State};
use axum::Json;
use axum::http::StatusCode;

use bson::Uuid as BsonUuid;

use chrono::{DateTime, NaiveTime, Utc, Weekday};

use futures_util::StreamExt;
use mongodb::Database;
use serde::{Serialize, Deserialize};

use uuid::Uuid;

use crate::db::collection_timeslots;
use crate::db::model::{Student, TimeSlot, BsonTimeSlot};

#[derive(Serialize, Deserialize)]
pub struct TimeslotQuery {
	id: Option<Uuid>,
}

pub async fn query(State(db): State<Database>, Query(q): Query<TimeslotQuery>) -> Result<Json<Vec<TimeSlot>>, (StatusCode, String)> {
	let query = q.id.map(|x| {
		bson::doc! {
			"id": BsonUuid::from_uuid_1(x),
		}
	});

	let r = tokio::spawn(async move {
		let collection = collection_timeslots(&db).await;

		collection.find(query, None).await
	}).await.unwrap();

	let ret = match r {
		Ok(x) => {
			x
				.filter_map(|i| async { i.ok() })
				.map(|v| v.into())
				.collect::<Vec<_>>()
				.await
		}
		Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
	};

	Ok(Json(ret))
}

#[derive(Deserialize)]
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

pub async fn create(State(db): State<Database>, Json(r): Json<TimeslotCreate>) -> Result<Json<TimeslotCreateReturn>, (StatusCode, String)> {
	let id = Uuid::new_v4();
	let ts = BsonTimeSlot {
		id: id.into(),
		students: r.students,
		weekday: r.weekday,
		time: r.time,
		timerange: Range { start: r.timerange.start.into(), end: r.timerange.end.into() },
	};
	
	let r = tokio::spawn(async move {
		let collection = collection_timeslots(&db).await;

		collection.insert_one(ts, None).await
	}).await.unwrap();

	match r {
		Ok(_) => Ok(Json(TimeslotCreateReturn { id })),
		Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
	}
}