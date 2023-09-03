use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Extension;

use serde_json::{json, Value};

use tracing::error;

use super::handlers::entry;
use super::logic::check_object_belong_to_userid;
use super::util::prelude::*;
use super::AppState;

use crate::auth::UserId;
use crate::db::model::Entry;
use crate::try_web;

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
	Json(r): Json<entry::CreateEntry>,
) -> WebResult<&'static str, Value> {
	match entry::create(u, db, r, q).await
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			NotFine(
				StatusCode::INTERNAL_SERVER_ERROR,
				json!("internal server error"),
			)
		}
	}
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(Entry, String)>, &'static str> {
	let data = match entry::query(u.clone(), db, q).await
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
		}
	};

	match data {
		Fine(c, d) => {
			try_web!(check_object_belong_to_userid(d.iter().map(|i| &i.0), &u));

			Fine(c, d)
		}
		NotFine(c, e) => NotFine(c, e),
	}
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(u32, String)>, &'static str> {
	match entry::missing(u, db, q).await
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
		}
	}
}

pub async fn next(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<(u32, String), &'static str> {
	match entry::next(u, db, q).await {
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
		}
	}
}
