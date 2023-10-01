use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::Extension;

use serde::Serialize;
use serde_json::Value;
use tracing::error;
use uuid::Uuid;

use crate::db::model::TimeSlot;

use crate::api::util::{prelude::*, WebError};
use crate::auth::UserId;

use super::logic::check_object_belong_to_userid_weberror;
use super::AppState;

use super::handlers::timeslot::{self, ExportRequest, TimeSlotQuery, TimeslotCreate};

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Query(q): Query<TimeSlotQuery>,
) -> WebResult<Vec<TimeSlot>, &'static str> {
	let data = match timeslot::query(u.clone(), db, q).await {
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return Err(WebError::internal_server_error());
		}
	};

	check_object_belong_to_userid_weberror(data.iter(), &u)?;

	Ok(data.into())
}

#[derive(Serialize)]
pub struct TimeslotCreateReturn {
	id: Uuid,
}

impl From<Uuid> for TimeslotCreateReturn {
	fn from(id: Uuid) -> Self {
		Self { id }
	}
}

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Json(r): Json<TimeslotCreate>,
) -> WebResult<TimeslotCreateReturn, &'static str> {
	match timeslot::create(u, db, r).await {
		Ok(d) => d.map(|id| (StatusCode::CREATED, id.into())).transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn export(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
	Query(q): Query<ExportRequest>,
) -> WebResult<String, Value> {
	match timeslot::export(u, db, q).await {
		Ok(v) => v.transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn information_length(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(TimeSlot, (u32, String), u32)>, &'static str> {
	match timeslot::information(u, db).await {
		Ok(v) => {
			let res: Vec<_> = v
				.into_iter()
				.map(|(ts, next, missing)| (ts, next, missing.len().try_into().unwrap()))
				.collect();

			Ok(res.into())
		}
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn information_length_v2(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
) -> WebResult<timeslot::InformationV2Response, &'static str> {
	match timeslot::information_v2(u, db).await {
		Ok(v) => Ok(v.into()),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn information_length_v3(
	State(AppState { db, .. }): State<AppState>,
	Extension(u): Extension<UserId>,
) -> WebResult<timeslot::InformationV3Response, &'static str> {
	match timeslot::information_v3(u, db).await {
		Ok(v) => Ok(v.into()),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}