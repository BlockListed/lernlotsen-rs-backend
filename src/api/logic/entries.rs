use std::cmp::min;

use std::collections::HashMap;
use std::ops::Range;

use axum::http::StatusCode;
use chrono::{NaiveDate, NaiveTime};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use mongodb::Database;

use tracing::error;

use crate::api::util::prelude::*;
use crate::db::collection_entries;
use crate::db::model::{EntryState, Student, TimeSlot, BsonTimeSlot};

pub fn verify_state(state: &EntryState, timeslot_students: &[Student]) -> Result<(), Vec<Student>> {
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
		_ => Ok(()),
	}
}


pub struct EntriesForTimeslot {
	range: Range<NaiveDate>,
	time: NaiveTime,
	index: u32,
}

pub fn get_entries(ts: &TimeSlot) -> EntriesForTimeslot {
	let now = Utc::now().date_naive();

	let start = ts.timerange.start;
	let end = min(now, ts.timerange.end);

	let range = Range { start, end };

	EntriesForTimeslot {
		range,
		time: ts.time.start,
		index: 0,
	}
}

impl Iterator for EntriesForTimeslot {
	type Item = DateTime<Utc>;

	fn next(&mut self) -> Option<Self::Item> {
		let days_from_start = 7 * self.index;

		let days_from_start = chrono::Days::new(days_from_start.into());

		let new_date = self.range.start.checked_add_days(days_from_start)?;

		if new_date > self.range.end {
			return None;
		}

		self.index += 1;
	
		Some(new_date.and_time(self.time).and_utc())
	}
}

pub async fn missing_entries(timeslot: BsonTimeSlot, db: &Database) -> WebResult<Vec<(usize, DateTime<Utc>)>, &'static str> {
	let entries = collection_entries(db).await;

	let timeslot_id = timeslot.id;

	let mut required_entries = get_entries(&timeslot.into()).enumerate().collect::<HashMap<_, _>>();
	
	let required_indexes = required_entries.keys().map(|x| *x as i32).collect::<Vec<_>>();

	let found_indexes = crate::handle_db!(entries.find(bson::doc! {
		"timeslot_id": timeslot_id,
		"index": {
			"$in": required_indexes,
		}
	}, None).await, "database error")
		.filter_map(|v| async {
			match v {
				Ok(x) => {
					Some(x.index)
				}
				Err(e) => {
					error!(%e, "invalid data in database");
					None
				}
			}
		})
		.collect::<Vec<_>>().await;

	for i in found_indexes {
		required_entries.remove(&(i as usize));
	}

	// All found entries have already been removed.
	let mut missing_entries = required_entries.into_iter().collect::<Vec<_>>();

	missing_entries.sort_unstable_by_key(|x| x.0);

	Fine(StatusCode::OK, missing_entries)
}