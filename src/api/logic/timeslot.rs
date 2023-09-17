use std::ops::Range;

use chrono::{IsoWeek, NaiveDate, Weekday};
use tracing::trace;

use crate::db::model::TimeSlot;

// Returns all timeslots indices, which fall into timerange
pub fn get_index_range_timeslot(ts: &TimeSlot, range: Range<IsoWeek>) -> Option<Range<u32>> {
	let start = NaiveDate::from_isoywd_opt(range.start.year(), range.start.week(), Weekday::Sun)?;
	let end = NaiveDate::from_isoywd_opt(range.end.year(), range.end.week(), Weekday::Sun)?;

	trace!(%start, %end, ts_id=%ts.id, "Getting timeslots in range.");

	if end < ts.timerange.start {
		return None;
	}

	// Both should always be positive
	// Since ts.timerange.start <= ts.timerange.end is checked for in creation function.
	let start_index: u32 = {
		if start < ts.timerange.start {
			0
		} else {
			(start - ts.timerange.start).num_weeks().try_into().ok()?
		}
	};
	trace!(start_index, "got start index");
	let end_index: u32 = {
		if end > ts.timerange.end {
			(ts.timerange.end - ts.timerange.start)
				.num_weeks()
				.try_into()
				.ok()?
		} else {
			(end - ts.timerange.start).num_weeks().try_into().ok()?
		}
	};
	trace!(end_index, "got end index");

	assert!(start_index <= end_index);

	Some(start_index..end_index)
}
