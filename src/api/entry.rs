use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Extension;

use serde_json::Value;

use tracing::error;

use super::handlers::entry::{self, UnfilledEntry};
use super::logic::check_object_belong_to_userid_weberror;
use super::util::prelude::*;
use super::AppState;

use crate::auth::UserId;
use crate::db::model::Entry;

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
	Json(r): Json<entry::CreateEntry>,
) -> WebResult<&'static str, Value> {
	match entry::create(u, db, r, q).await {
		Ok(d) => d
			.map(|_| (StatusCode::CREATED, "created entry"))
			.transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(Entry, String)>, &'static str> {
	let data = match entry::query(u.clone(), db, q).await {
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return Err(WebError::internal_server_error());
		}
	};

	if let Ok(d) = data.as_ref() {
		check_object_belong_to_userid_weberror(d.iter().map(|v| &v.0), &u)?;
	}

	data.transpose()
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(u32, String)>, &'static str> {
	match entry::missing(u, db, q).await {
		Ok(d) => d.map(|v| v.into_iter().map(|e| (e.index, e.timestamp.to_rfc3339())).collect::<Vec<_>>()).transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

// V3 cause v3 routes are no-tuple
pub async fn missing_v3(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<UnfilledEntry>, &'static str> {
	match entry::missing(u, db, q).await {
		Ok(d) => d.transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn next(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<(u32, String), &'static str> {
	match entry::next(u, db, q).await {
		Ok(d) => d.map(|e| (e.index, e.timestamp.to_rfc3339())).transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}

pub async fn next_v3(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<UnfilledEntry, &'static str> {
	match entry::next(u, db, q).await {
		Ok(d) => d.transpose(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}
