use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::Extension;

use serde::Serialize;
use tracing::error;
use uuid::Uuid;

use crate::db::model::TimeSlot;

use crate::api::util::prelude::*;
use crate::try_web;

use super::auth::UserId;
use super::logic::check_object_belong_to_userid;
use super::AppState;

use super::handlers::timeslot::{self, TimeSlotQuery, TimeslotCreate};

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Query(q): Query<TimeSlotQuery>,
) -> WebResult<Vec<TimeSlot>, &'static str> {
	let data = match spawn_in_current_span(timeslot::query(u.clone(), db, q))
		.await
		.unwrap()
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
		}
	};

	try_web!(check_object_belong_to_userid(data.iter(), &u));

	Fine(StatusCode::OK, data)
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
	let data = match spawn_in_current_span(timeslot::create(u, db, r))
		.await
		.unwrap()
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
		}
	};

	match data {
		Fine(c, id) => Fine(c, TimeslotCreateReturn { id }),
		NotFine(c, e) => NotFine(c, e),
	}
}
