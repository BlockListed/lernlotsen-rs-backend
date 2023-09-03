use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use chrono::{Duration, NaiveDate};
use chrono_tz::Tz;
use futures_util::StreamExt;
use mongodb::Database;

use tracing::{debug, error, trace, warn};

use crate::db::collection_entries;
use crate::db::model::{BsonTimeSlot, EntryState, Student, TimeSlot};

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

pub struct EntriesForTimeslot<'a> {
	timeslot: &'a TimeSlot,
	until: NaiveDate,
	index: u64,
}

pub fn get_entries(timeslot: &TimeSlot) -> EntriesForTimeslot {
	let now = Utc::now().date_naive();

	trace!(until=%now, "calculating missing entries");

	EntriesForTimeslot {
		timeslot,
		until: now,
		index: 0,
	}
}

impl<'a> Iterator for EntriesForTimeslot<'a> {
	type Item = DateTime<chrono_tz::Tz>;

	fn next(&mut self) -> Option<Self::Item> {
		let date_opt = get_time_from_index_and_timeslot(self.timeslot, self.index).and_then(|x| {
			if x.date_naive() < self.until {
				Some(x)
			} else {
				None
			}
		});

		let date = match date_opt {
			Some(d) => d,
			None => {
				trace!(index = self.index, "missing entry iterator finished");
				return None;
			}
		};

		self.index += 1;

		Some(date)
	}
}

pub fn get_time_from_index_and_timeslot(
	timeslot: &TimeSlot,
	index: u64,
) -> Option<DateTime<chrono_tz::Tz>> {
	let days_from_start = chrono::Days::new(7 * index);

	let new_date = timeslot.timerange.start.checked_add_days(days_from_start)?;

	if new_date > timeslot.timerange.end {
		return Option::None;
	}

	let time = new_date.and_time(timeslot.time.start);

	let local_time = time.and_local_timezone(timeslot.timezone);

	use chrono::LocalResult::*;
	match local_time {
		None => {
			warn!(%time, "could not convert date to local");
			Option::None
		}
		Single(t) => Some(t),
		Ambiguous(s, e) => {
			warn!(%time, start=%s, end=%e, "time in local timezone is ambiguous");
			Option::None
		}
	}
}

pub async fn missing_entries(
	timeslot: BsonTimeSlot,
	db: &Database,
) -> anyhow::Result<Vec<(usize, String)>> {
	let entries = collection_entries(db).await;

	let timeslot_id = timeslot.id;

	let mut required_entries = get_entries(&timeslot.into())
		.enumerate()
		.collect::<HashMap<_, _>>();

	debug!(
		?required_entries,
		"calculated required entries for timeslot"
	);

	let required_indexes = required_entries
		.keys()
		.map(|x| *x as i32)
		.collect::<Vec<_>>();

	let found_indexes =
		entries
			.find(
				bson::doc! {
					"timeslot_id": timeslot_id,
					"index": {
						"$in": required_indexes,
					}
				},
				None
			)
			.await?
	.filter_map(|v| async {
		match v {
			Ok(x) => Some(x.index),
			Err(e) => {
				error!(%e, "invalid data in database");
				None
			}
		}
	})
	.collect::<Vec<_>>()
	.await;

	for i in found_indexes {
		required_entries.remove(&(i as usize));
	}

	// All found entries have already been removed.
	let mut missing_entries = required_entries
		.into_iter()
		.map(|(i, d)| (i, d.to_rfc3339()))
		.collect::<Vec<_>>();

	missing_entries.sort_unstable_by_key(|x| x.0);

	Ok(missing_entries)
}

pub fn next_entry_date_timeslot(ts: &TimeSlot) -> Option<(u32, DateTime<chrono_tz::Tz>)> {
	let start = ts.timerange.start.and_time(ts.time.start);

	let start: DateTime<Tz> = ts.timezone.from_local_datetime(&start).single()?;

	let now = Utc::now();

	let since = now - start.with_timezone(&Utc);

	// The first entry hasn't happened yet.
	// So the first entry is the next entry.
	if since < Duration::zero() {
		return Some((0, start));
	}

	let seconds = since.num_seconds();
	assert!(seconds >= 0);

	// Number of days from `start` until the next (from now) event
	let next_seconds = crate::util::round_up_to_multiple(seconds, Duration::weeks(1).num_seconds());

	let next_date = start + Duration::seconds(next_seconds);

	// Will never be negative
	let index = next_seconds / Duration::weeks(1).num_seconds();
	assert!(index >= 0);

	Some((index as u32, next_date))
}
