use super::util::prelude::*;
use axum::http::StatusCode;
use tracing::error;

pub mod entries;

pub fn check_entries_belong_to_userid<'a>(
	mut entries: impl Iterator<Item = &'a crate::db::model::Entry>,
	user_id: &str,
) -> WebResult<(), &'static str> {
	let Some(invalid) = entries.find(|i| i.user_id != user_id) else {
		return Fine(StatusCode::OK, ())
	};

	error!(invalid_entry = ?invalid, "application attempted to return entry, which does not belong to user!");

	// technically true and avoids leaking this information.
	return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
}

pub fn check_timeslots_belong_to_userid<'a>(
	mut timeslots: impl Iterator<Item = &'a crate::db::model::TimeSlot>,
	user_id: &str,
) -> WebResult<(), &'static str> {
	let Some(invalid) = timeslots.find(|i| i.user_id != user_id) else {
		return Fine(StatusCode::OK, ());
	};

	error!(invalid_timeslot = ?invalid, "application attempt to return timeslot, which does not belong to user!");

	// technically true and avoids leaking this information.
	return NotFine(StatusCode::INTERNAL_SERVER_ERROR, "database error");
}
