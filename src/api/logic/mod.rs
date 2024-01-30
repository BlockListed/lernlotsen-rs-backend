use tracing::error;

use crate::auth::UserId;
use crate::db::model::HasUserId;

pub mod entry;
pub mod export;
pub mod timeslot;

pub fn check_object_belong_to_userid<'a, T: HasUserId + 'a>(
	mut entries: impl Iterator<Item = &'a T>,
	user_id: &UserId,
) -> anyhow::Result<()> {
	let Some(invalid) = entries.find(|i| i.user_id() != user_id.as_str()) else {
		return Ok(());
	};

	error!(
		id = invalid.identifier(),
		"application attempted to return entry, which does not belong to user!"
	);

	// technically true and avoids leaking this information.
	Err(anyhow::anyhow!(
		"application attempt to return object, which doesn't belong to user"
	))
}
