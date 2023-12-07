use chrono::{Datelike, IsoWeek, NaiveDate};

pub mod logging;

// Grabbed from `https://stackoverflow.com/questions/3407012/rounding-up-to-the-nearest-multiple-of-a-number`
pub fn round_up_to_multiple(n: i64, multiple: i64) -> i64 {
	if multiple == 0 {
		return n;
	};

	let remainder = n.abs() % multiple;
	if remainder == 0 {
		return n;
	};

	if n < 0 {
		-(n.abs() - remainder)
	} else {
		n + multiple - remainder
	}
}

pub fn create_isoweek(year: i32, week: u32) -> Option<IsoWeek> {
	let date = NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon)?;

	Some(date.iso_week())
}
