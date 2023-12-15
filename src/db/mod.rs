use std::{str::FromStr, time::Duration};

use futures_util::StreamExt;
use sqlx::{
	postgres::{PgConnectOptions, PgPoolOptions},
	PgPool,
};

use crate::configuration::Config;

pub mod model;
pub mod queries;

pub async fn get_pool(cfg: &Config) -> PgPool {
	let pg_options = PgConnectOptions::from_str(&cfg.database.uri)
		.expect("Invalid database URI")
		.application_name("lelo_backend");

	let pool_options = PgPoolOptions::new()
		.max_connections(5)
		.min_connections(1)
		.max_lifetime(Duration::from_secs(3600))
		.acquire_timeout(Duration::from_secs(10));

	let pool = pool_options
		.connect_with(pg_options)
		.await
		.expect("Couldn not connect to db!");

	sqlx::migrate!()
		.run(&pool)
		.await
		.expect("Couldn't complete migrations");

	migrate_entry_state(&pool).await;

	pool
}

async fn migrate_entry_state(db: &PgPool) {
	use model::{EntryState, OldEntryState, StudentState, StudentStatus};

	sqlx::query_as!(
		model::Entry,
		r#"SELECT user_id, index, timeslot_id, state as "state: sqlx::types::Json<OldEntryState>", state_enum as "state_enum: EntryState", students as "students: Vec<StudentState>" FROM entries WHERE state IS NOT NULL"#
	)
	.fetch(db)
	.for_each_concurrent(8, |e| async {
		let entry = e.unwrap();

		let Some(state) = entry.state.map(|v| v.0) else {
			tracing::error!("state should have been not null");
			return;
		};

		let state_enum = match state {
			OldEntryState::Success { ref students } => {
				let present_count = students.iter().filter(|v| v.1 == StudentStatus::Present).count();
				let pardoned_count = students.iter().filter(|v| v.1 == StudentStatus::Pardoned).count();

				if present_count == 0 {
					if pardoned_count == 0 {
						EntryState::StudentsMissing
					} else {
						EntryState::CancelledByStudents
					}
				} else {
					EntryState::Success
				}
			},
			OldEntryState::CancelledByStudents => EntryState::CancelledByStudents,
			OldEntryState::StudentsMissing => EntryState::StudentsMissing,
			OldEntryState::CancelledByTutor => EntryState::CancelledByTutor,
			OldEntryState::Holidays => EntryState::Holidays,
			OldEntryState::Other => EntryState::Other,
			OldEntryState::InvalidData => panic!("invalid data in field")
		};

		let students = match state {
			OldEntryState::Success { students } => Some(students.into_iter().map(|(s, status)| StudentState { student: s.name, status }).collect::<Vec<_>>()),
			_ => None,
		};

		sqlx::query!("UPDATE entries SET state = NULL, state_enum = $3, students = $4 WHERE timeslot_id = $1 AND index = $2", entry.timeslot_id, entry.index, state_enum as EntryState, students as Option<Vec<StudentState>>).execute(db).await.unwrap();
	})
	.await;
}
