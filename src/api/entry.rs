use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Extension;

use serde_json::{json, Value};

use tracing::error;

use super::auth::UserId;
use super::handlers::entry;
use super::logic::check_object_belong_to_userid;
use super::util::prelude::*;
use super::AppState;

use crate::db::model::Entry;

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
	Json(r): Json<entry::CreateEntry>,
) -> WebResult<&'static str, Value> {
	match spawn_in_current_span(entry::create(u, db, r, q))
		.await
		.unwrap()
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return NotFine(
				StatusCode::INTERNAL_SERVER_ERROR,
				json!("internal server error"),
			);
		}
	}
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(Entry, String)>, &'static str> {
	let data = match spawn_in_current_span(entry::query(u.clone(), db, q))
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
		Fine(c, d) => {
			if let NotFine(c, e) = check_object_belong_to_userid(d.iter().map(|i| &i.0), &u) {
				return NotFine(c, e);
			}

			Fine(c, d)
		}
		NotFine(c, e) => NotFine(c, e),
	}
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<entry::TimeSlotQuery>,
	Extension(u): Extension<UserId>,
) -> WebResult<Vec<(usize, String)>, &'static str> {
	match spawn_in_current_span(entry::missing(u, db, q))
		.await
		.unwrap()
	{
		Ok(d) => d,
		Err(e) => {
			error!(%e, "error while handling request");
			return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
		}
	}
}
