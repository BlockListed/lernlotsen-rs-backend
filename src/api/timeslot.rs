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

use super::logic::check_object_belong_to_userid;
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

	if let Err(e) = check_object_belong_to_userid(data.iter(), &u) {
		error!(%e, "objects don't belong to userid");
		return Err(WebError::internal_server_error());
	}

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
