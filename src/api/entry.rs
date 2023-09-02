use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::Extension;

use futures_util::StreamExt;

use serde::Deserialize;
use serde_json::{json, Value};

use tracing::{debug, error};
use uuid::Uuid;

use crate::api::logic;
use crate::api::logic::entries::get_time_from_index_and_timeslot;
use crate::db::model::{BsonEntry, Entry, EntryState, TimeSlot};
use crate::db::{collection_entries, collection_timeslots};

use super::auth::UserId;
use super::logic::check_entries_belong_to_userid;
use super::util::prelude::*;
use super::AppState;

use logic::entries::missing_entries;
use logic::entries::verify_state;

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	state: EntryState,
	index: u32,
}

pub async fn create(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<TimeSlotQuery>,
	Extension(UserId(t)): Extension<UserId>,
	Json(r): Json<CreateEntry>,
) -> WebResult<&'static str, Value> {
	spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let selected_timeslot = match crate::handle_db!(
			timeslots
				.find_one(
					bson::doc! {
						"user_id": &t,
						"id": q.id,
					},
					None,
				)
				.await,
			json!("database error")
		) {
			Some(x) => x,
			None => {
				return NotFine(StatusCode::NOT_FOUND, json!("timeslot not found"));
			}
		};

		let entry = match verify_state(&r.state, &selected_timeslot.students) {
			Ok(_) => BsonEntry {
				user_id: t,
				index: r.index,
				timeslot_id: selected_timeslot.id,
				state: r.state,
			},
			Err(s) => {
				debug!("request contained invalid students");
				return NotFine(
					StatusCode::UNPROCESSABLE_ENTITY,
					json!({
						"invalid_students": s,
					}),
				);
			}
		};

		let entries = collection_entries(&db).await;

		if let Err(e) = entries.insert_one(entry, None).await {
			use mongodb::error::{ErrorKind, WriteError, WriteFailure};

			match *e.kind {
				ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) => {
					debug!("Duplicated entry.");

					return NotFine(StatusCode::CONFLICT, json!("duplicate index"));
				}
				_ => {
					error!(%e, "encountered database error");

					return NotFine(StatusCode::INTERNAL_SERVER_ERROR, json!("database error"));
				}
			}
		};

		Fine(StatusCode::CREATED, "created entry")
	})
	.await
	.unwrap()
}

#[derive(Deserialize)]
pub struct TimeSlotQuery {
	id: Uuid,
}

pub async fn query(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<TimeSlotQuery>,
	Extension(UserId(t)): Extension<UserId>,
) -> WebResult<Vec<(Entry, String)>, &'static str> {
	spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let timeslot: TimeSlot = match crate::handle_db!(timeslots.find_one(bson::doc! {
			"user_id": &t,
			"id": q.id,
		}, None).await, "database error") {
			Some(x) => x.into(),
			None => {
				return NotFine(StatusCode::NOT_FOUND, "timeslot not found");
			}
		};

		let entries = collection_entries(&db).await;

		let res: Vec<_> = crate::handle_db!(entries.find(bson::doc! {
			"timeslot_id": timeslot.id,
		}, None).await, "database error")
			.filter_map(|v| async {
				match v {
					Ok(x) => Some(x),
					Err(e) => {
						error!(%e, "invalid data in database");
						None
					}
				}
			})
			.filter_map(|v| async {
				let entry: Entry = v.into();
				let Some(time) = get_time_from_index_and_timeslot(&timeslot, entry.index as u64) else {
					error!(timeslot=%entry.timeslot_id, index=%entry.index, "date of entry in database overflows the chrono limits");
					return None;
				};

				Some((entry, time.to_rfc3339()))
			})
			.collect::<Vec<_>>()
			.await;

		match check_entries_belong_to_userid(res.iter().map(|i| &i.0), &t) {
			Fine( .. ) => (),
			NotFine(c, e) => return NotFine(c, e)
		}

		Fine(StatusCode::OK, res)
	})
	.await
	.unwrap()
}

pub async fn missing(
	State(AppState { db, .. }): State<AppState>,
	Path(q): Path<TimeSlotQuery>,
	Extension(UserId(t)): Extension<UserId>,
) -> WebResult<Vec<(usize, String)>, &'static str> {
	spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let timeslot = match crate::handle_db!(
			timeslots
				.find_one(
					Some(bson::doc! {
						"user_id": t,
						"id": q.id,
					}),
					None
				)
				.await,
			"database error"
		) {
			Some(v) => v,
			None => {
				return NotFine(StatusCode::NOT_FOUND, "timeslot not found");
			}
		};

		missing_entries(timeslot, &db).await
	})
	.await
	.unwrap()
}
