use std::ops::Range;

use chrono::{IsoWeek, NaiveDate, Weekday};

use crate::db::model::TimeSlot;

pub fn get_index_range_timeslot(ts: &TimeSlot, range: Range<IsoWeek>) -> Option<Range<u32>> {
	let start = NaiveDate::from_isoywd_opt(range.start.year(), range.start.week(), Weekday::Mon)?;
	let end = NaiveDate::from_isoywd_opt(range.end.year(), range.end.week(), Weekday::Sun)?;

	if start < ts.timerange.start {
		return None;
	}

	if end > ts.timerange.end {
		return None;
	}

	// Both should always be positive
	// Since ts.timerange.start <= ts.timerange.end is checked for in creation function.
	let start_index: u32 = (start - ts.timerange.start).num_weeks().try_into().ok()?;
	let end_index: u32 = (end - ts.timerange.start).num_weeks().try_into().ok()?;

	Some(start_index..end_index)
}
