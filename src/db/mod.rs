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
	migrate_fix_empty_students(&pool).await;

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

async fn migrate_fix_empty_students(db: &PgPool) {
	use model::{Entry, EntryState, OldEntryState, StudentState, StudentStatus};

	sqlx::query_as!(
		Entry,
		r#"SELECT user_id, index, timeslot_id, state_enum AS "state_enum: EntryState", students AS "students: Vec<StudentState>", NULL AS "state: sqlx::types::Json<OldEntryState>" FROM entries WHERE state_enum IS NOT NULL AND students IS NULL"#
	)
	.fetch(db)
	.for_each_concurrent(8, |e| async {
		let entry: Entry = e.unwrap();

		let students: Vec<StudentState> = match entry.state_enum.unwrap() {
			EntryState::Success => {
				get_students_for_timeslot_id_with_status(db, entry.timeslot_id, StudentStatus::Present).await
			},
			EntryState::CancelledByStudents => {
				get_students_for_timeslot_id_with_status(db, entry.timeslot_id, StudentStatus::Pardoned).await
			},
			EntryState::StudentsMissing => {
				get_students_for_timeslot_id_with_status(db, entry.timeslot_id, StudentStatus::Missing).await
			}
			EntryState::CancelledByTutor | EntryState::Holidays | EntryState::Other => {
				Vec::new()
			},
		};

		sqlx::query!("UPDATE entries SET students = $3 WHERE timeslot_id = $1 AND index = $2", entry.timeslot_id, entry.index, students as Vec<StudentState>).execute(db).await.unwrap();
	})
	.await;
}

async fn get_students_for_timeslot_id_with_status(
	db: &PgPool,
	id: uuid::Uuid,
	status: model::StudentStatus,
) -> Vec<model::StudentState> {
	get_students_for_timeslot_id(db, id)
		.await
		.into_iter()
		.map(|v| model::StudentState { student: v, status })
		.collect()
}

async fn get_students_for_timeslot_id(db: &PgPool, id: uuid::Uuid) -> Vec<String> {
	sqlx::query!(
		r#"SELECT students AS "students: Vec<String>" FROM timeslots WHERE id = $1"#,
		id
	)
	.fetch_one(db)
	.await
	.unwrap()
	.students
}
