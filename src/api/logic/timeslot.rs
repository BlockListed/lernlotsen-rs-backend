use std::ops::Range;

use chrono::IsoWeek;

use crate::db::model::TimeSlot;

pub fn get_index_range_timeslot(ts: &TimeSlot, range: Range<IsoWeek>) -> Range<u32> {
	todo!()
}
