use axum::extract::{Json, State, Query};
use axum::http::StatusCode;

use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use mongodb::Database;

use serde::Deserialize;
use serde_json::{json, Value};

use tracing::{debug, error};
use uuid::Uuid;

use crate::api::logic;
use crate::db::model::{BsonEntry, Entry, EntryState};
use crate::db::{collection_entries, collection_timeslots};

use super::util::prelude::*;

use super::logic::entries::verify_state;

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	timeslot_id: Uuid,
	state: EntryState,
	index: u32,
}

pub async fn create(
	State(db): State<Database>,
	Json(r): Json<CreateEntry>,
) -> WebResult<&'static str, Value> {
	let r = spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let selected_timeslot = match timeslots
			.find_one(
				bson::doc! {
					"id": r.timeslot_id,
				},
				None,
			)
			.await
		{
			Ok(x) => match x {
				Some(x) => x,
				None => {
					return NotFine(StatusCode::NOT_FOUND, json!("timeslot not found"));
				}
			},
			Err(e) => {
				error!(%e, "encountered database error");
				return NotFine(StatusCode::INTERNAL_SERVER_ERROR, json!("database error"));
			}
		};

		let entry = match verify_state(&r.state, &selected_timeslot.students) {
			Ok(_) => BsonEntry {
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

		Fine(StatusCode::OK, ())
	})
	.await
	.unwrap();

	match r {
		Fine(..) => Fine(StatusCode::CREATED, "created entry"),
		NotFine(c, e) => NotFine(c, e),
	}
}

pub async fn query(State(db): State<Database>) -> WebResult<Vec<Entry>, &'static str> {
	spawn_in_current_span(async move {
		let entries = collection_entries(&db).await;

		let res: Vec<Entry> = match entries.find(None, None).await {
			Ok(c) => {
				c.filter_map(|v| async {
					match v {
						Ok(x) => Some(x),
						Err(e) => {
							error!(%e, "invalid data in database");
							None
						}
					}
				})
				.map(|v| v.into())
				.collect::<Vec<_>>()
				.await
			}
			Err(e) => {
				error!(%e, "encountered database error");
				return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
			}
		};

		Fine(StatusCode::OK, res)
	})
	.await
	.unwrap()
}

#[derive(Deserialize)]
pub struct MissingQuery {
	timeslot_id: Uuid,
}

pub async fn missing(State(db): State<Database>, Query(q): Query<MissingQuery>) -> WebResult<Vec<DateTime<Utc>>, &'static str> {
	spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let timeslot = match timeslots.find_one(Some(bson::doc! {
			"id": q.timeslot_id,
		}), None).await {
			Ok(x) => {
				match x {
					Some(v) => v,
					None => {
						return NotFine(StatusCode::NOT_FOUND, "timeslot not found");
					}
				}
			}
			Err(e) => {
				error!(%e, "encountered database error");
				return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
			}
		};

		let entries = collection_entries(&db).await;

		let timeslot_id = timeslot.id;

		let required_entries = logic::entries::get_entries(&timeslot.into());

		let mut missing = Vec::<DateTime<Utc>>::new();

		// TODO
		// Optimise this, so it doesn't perform this many queries.
		for (i, d) in required_entries {
			match entries.find_one(Some(bson::doc! {
				"timeslot_id": timeslot_id,
				"index": i,
			}), None).await {
				Ok(x) => {
					if x.is_none() {
						missing.push(d);
					}
				}
				Err(e) => {
					error!(%e, "encountered database error");
					return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
				}
			}
		}

		Fine(StatusCode::OK, missing)
	}).await.unwrap()
}