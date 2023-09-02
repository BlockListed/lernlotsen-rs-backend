use crate::db::model::HasUserId;

use super::{auth::UserId, util::prelude::*};
use axum::http::StatusCode;
use tracing::error;

pub mod entries;

pub fn check_object_belong_to_userid<'a, T: HasUserId<'a> + 'a>(
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
