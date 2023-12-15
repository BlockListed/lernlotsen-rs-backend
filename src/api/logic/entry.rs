use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use chrono::{Duration, NaiveDate};
use chrono_tz::Tz;
use itertools::Itertools;

use sqlx::PgPool;
use tracing::{debug, error, trace, warn};

use crate::api::handlers;
use crate::api::handlers::entry::UnfilledEntry;
use crate::auth::UserId;
use crate::db::model::{Student, StudentState, WebTimeSlot};
use crate::db::queries::entry::get_entries_with_index_in;

pub fn verify_state(
	student_states: &[StudentState],
	timeslot_students: &[Student],
) -> Result<(), Vec<Student>> {
	let mut invalid_students = Vec::with_capacity(timeslot_students.len());

	if student_states.len() != timeslot_students.len() {
		return Err(invalid_students);
	}

	for StudentState { student, .. } in student_states {
		if !timeslot_students.iter().any(|x| &x.name == student) {
			invalid_students.push(Student {
				name: student.clone(),
			});
		}
	}

	if invalid_students.is_empty() {
		Ok(())
	} else {
		Err(invalid_students)
	}
}

pub struct EntriesForTimeslot<'a> {
	timeslot: &'a WebTimeSlot,
	until: NaiveDate,
	index: u32,
}

fn get_entries(timeslot: &WebTimeSlot) -> EntriesForTimeslot {
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
			if x.date_naive() <= self.until {
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
	timeslot: &WebTimeSlot,
	index: u32,
) -> Option<DateTime<chrono_tz::Tz>> {
	let days_from_start = chrono::Days::new(u64::from(7 * index));

	let new_date = timeslot.timerange.start.checked_add_days(days_from_start)?;

	if new_date > timeslot.timerange.end {
		return Option::None;
	}

	let time = new_date.and_time(timeslot.time.start);

	let local_time = time.and_local_timezone(timeslot.timezone);

	// This is cleaner.
	// Also this attribute isn't useless.
	#[allow(clippy::useless_attribute)]
	#[allow(clippy::items_after_statements)]
	use chrono::LocalResult::{Ambiguous, None, Single};
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
	db: PgPool,
	u: UserId,
	timeslot: WebTimeSlot,
) -> anyhow::Result<Vec<handlers::entry::UnfilledEntry>> {
	let mut required_entries = get_entries(&timeslot)
		.enumerate()
		.collect::<HashMap<_, _>>();

	debug!(
		?required_entries,
		"calculated required entries for timeslot"
	);

	// Entry indices are always u32 or smaller.
	let required_indexes = required_entries
		.keys()
		.map(|x| (*x).try_into())
		.try_collect()?;

	let found_indexes = get_entries_with_index_in(db.clone(), u, timeslot.id, required_indexes)
		.await?
		.into_iter()
		.map(|v| v.index)
		.collect::<Vec<_>>();

	for i in found_indexes {
		required_entries.remove(&(i.try_into().expect("Failed to convert u32 into usize")));
	}

	// All found entries have already been removed.
	// Entry indices are always u32 or smaller.
	let mut missing_entries = required_entries
		.into_iter()
		.map(|(i, d)| {
			Ok(UnfilledEntry {
				index: i.try_into()?,
				timestamp: d.fixed_offset(),
			})
		})
		.try_collect::<_, Vec<_>, std::num::TryFromIntError>()?;

	missing_entries.sort_unstable_by_key(|x| x.index);

	Ok(missing_entries)
}

pub fn next_entry_date_timeslot(ts: &WebTimeSlot) -> Option<(u32, DateTime<chrono_tz::Tz>)> {
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

	// Number of seconds from `start` until the next (from now) event
	let next_seconds = crate::util::round_up_to_multiple(seconds, Duration::weeks(1).num_seconds());

	let next_date = start + Duration::seconds(next_seconds);

	// Will never be negative
	let raw_index = next_seconds / Duration::weeks(1).num_seconds();
	assert!(raw_index >= 0);

	let index: u32 = raw_index
		.try_into()
		.map_err(|_| {
			error!(raw_index, "next entry index value not valid u32");
		})
		.ok()?;

	// This is fine, since we check for a negative index.
	Some((index, next_date))
}
