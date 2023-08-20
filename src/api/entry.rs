use axum::extract::{Json, State};
use axum::http::StatusCode;

use mongodb::Database;

use serde::Deserialize;
use serde_json::{Value, json};

use tracing::{debug, error};
use uuid::Uuid;

use crate::db::{collection_timeslots, collection_entries};
use crate::db::model::{EntryState, Student, BsonEntry};

use super::util::prelude::*;

#[derive(Deserialize, Debug)]
pub struct CreateEntry {
	timeslot_id: Uuid,
	state: EntryState,
	index: u32,
}

pub async fn create(State(db): State<Database>, Json(r): Json<CreateEntry>) -> WebResult<&'static str, Value> {
	let r = spawn_in_current_span(async move {
		let timeslots = collection_timeslots(&db).await;

		let selected_timeslot = match timeslots.find_one(bson::doc! {
			"id": r.timeslot_id,
		}, None).await {
			Ok(x) => {
				match x {
					Some(x) => x,
					None => {
						return NotFine(StatusCode::NOT_FOUND, json!("timeslot not found"));
					}
				}
			}
			Err(e) => {
				error!(%e, "encountered database error");
				return NotFine(StatusCode::INTERNAL_SERVER_ERROR, json!("database error"));
			}
		};

		let entry = match verify_state(&r.state, &selected_timeslot.students) {
			Ok(_) => {
				BsonEntry {
					index: r.index,
					timeslot: selected_timeslot,
					state: r.state
				}
			}
			Err(s) => {
				debug!("request contained invalid students");
				return NotFine(
					StatusCode::UNPROCESSABLE_ENTITY,
					json!({
						"invalid_students": s,
					})
				);
			}
		};

		let entries = collection_entries(&db).await;

		if let Err(e) = entries.insert_one(entry, None).await {
			use mongodb::error::{ErrorKind, WriteFailure, WriteError};

			match *e.kind {
				ErrorKind::Write(WriteFailure::WriteError(WriteError {code: 11000, ..})) => {
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
	}).await.unwrap();

	match r {
		Fine(..) => Fine(StatusCode::CREATED, "created entry"),
		NotFine(c, e) => NotFine(c, e),
	}

	
}

fn verify_state(state: &EntryState, timeslot_students: &[Student]) -> Result<(), Vec<Student>> {
	let mut invalid_students = Vec::new();

	match state {
		EntryState::Success { students } => {
			for (k, _) in students {
				if !timeslot_students.iter().any(|x| x == k) {
					invalid_students.push(k.clone());
				}
			}

			if invalid_students.is_empty() {
				Ok(())
			} else {
				Err(invalid_students)
			}
		}
		_ => Ok(())
	}
}