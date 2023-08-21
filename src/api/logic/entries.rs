use std::cmp::min;

use std::ops::Range;

use chrono::{NaiveDate, NaiveTime};
use chrono::{DateTime, Utc};

use crate::db::model::{EntryState, Student, TimeSlot};

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
	type Item = (u32, DateTime<Utc>);

	fn next(&mut self) -> Option<Self::Item> {
		let days_from_start = 7 * self.index;

		let days_from_start = chrono::Days::new(days_from_start.into());

		let new_date = self.range.start.checked_add_days(days_from_start)?;

		if new_date > self.range.end {
			return None;
		}

		let ret = self.index;

		self.index += 1;
	
		Some((ret, new_date.and_time(self.time).and_utc()))
	}
}
