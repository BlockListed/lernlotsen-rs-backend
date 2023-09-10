use axum::http::StatusCode;
use tracing::error;

use crate::api::util::prelude::*;

use crate::auth::UserId;
use crate::db::model::HasUserId;

pub mod entry;
pub mod timeslot;
pub mod export;

pub fn check_object_belong_to_userid<'a, T: HasUserId + 'a>(
	mut entries: impl Iterator<Item = &'a T>,
	user_id: &UserId,
) -> WebResult<(), &'static str> {
	let Some(invalid) = entries.find(|i| i.user_id() != user_id.0) else {
		return Fine(StatusCode::OK, ())
	};

	error!(
		id = invalid.identifier(),
		"application attempted to return entry, which does not belong to user!"
	);

	// technically true and avoids leaking this information.
	NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}
