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

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
	Json(r): Json<entry::CreateEntry>,
) -> WebResult<&'static str, Value> {
	match entry::create(u, db, r, q).await {
		Ok(d) => d
			.map(|_| (StatusCode::CREATED, "created entry"))
			.transpose_web(),
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
) -> WebResult<Vec<entry::QueryReturn>, &'static str> {
	let data = match entry::query(u.clone(), db, q).await {
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return Err(WebError::internal_server_error());
		}
	};

	if let Ok(d) = data.as_ref() {
		check_object_belong_to_userid_weberror(d.iter().map(|v| &v.entry), &u)?;
	}

	data.transpose_web()
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<UnfilledEntry>, &'static str> {
	match entry::missing(u, db, q).await {
		Ok(d) => d.transpose_web(),
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
) -> WebResult<UnfilledEntry, &'static str> {
	match entry::next(u, db, q).await {
		Ok(d) => d.transpose_web(),
		Err(e) => {
			error!(%e, "error while handling request");
			Err(WebError::internal_server_error())
		}
	}
}
