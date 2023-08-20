use std::ops::Range;

use std::cmp::max;

use chrono::{DateTime, Utc};
use chrono::Weekday;

use crate::db::model::TimeSlot;

pub struct EntriesForTimeslot {
	range: Range<DateTime<Utc>>,
	weekday: Weekday,
	index: u32,
}

pub fn get_entries(ts: &TimeSlot) -> EntriesForTimeslot {
	let now = Utc::now();

	let start = ts.timerange.start;
	let end = max(now, ts.timerange.end);

	let range = Range { start, end };

	EntriesForTimeslot { range, weekday: ts.weekday, index: 0 }
}

impl Iterator for EntriesForTimeslot {
	type Item = DateTime<Utc>;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}