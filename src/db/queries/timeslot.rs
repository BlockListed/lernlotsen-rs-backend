use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	auth::UserId,
	db::model::{self, DbTime, DbTimerange, TimeSlot, WebTimeSlot},
};

pub async fn get_timeslots(db: PgPool, u: UserId) -> anyhow::Result<Vec<WebTimeSlot>> {
	let timeslots_db: Vec<TimeSlot> = sqlx::query_as!(TimeSlot, r#"SELECT id, user_id, subject, students, time AS "time: DbTime", timerange AS "timerange: DbTimerange", timezone FROM timeslots WHERE user_id = $1"#, u.as_str())
		.fetch_all(&db)
		.await?;

	let timeslots: Vec<WebTimeSlot> = timeslots_db
		.into_iter()
		.filter_map(model::convert_ts)
		.collect();

	Ok(timeslots)
}

pub async fn get_timeslot_by_id(
	db: PgPool,
	u: UserId,
	id: Uuid,
) -> anyhow::Result<Option<WebTimeSlot>> {
	let timeslot_db: TimeSlot = match sqlx::query_as!(TimeSlot, r#"SELECT user_id, id, subject, students, time AS "time: DbTime", timerange AS "timerange: DbTimerange", timezone FROM timeslots WHERE user_id = $1 AND id = $2"#, u.as_str(), id)
		.fetch_optional(&db)
		.await {
			Ok(ts_opt) => if let Some(ts) = ts_opt { ts } else { return Ok(None) },
			Err(e) => Err(e)?,
		};

	Ok(Some(
		model::convert_ts(timeslot_db).context("invalid data in db")?,
	))
}

pub async fn insert_timeslot(db: PgPool, ts: TimeSlot) -> anyhow::Result<()> {
	sqlx::query!("INSERT INTO timeslots (id, user_id, subject, students, time, timerange, timezone) VALUES ($1, $2, $3, $4, $5, $6, $7)", ts.id, ts.user_id, ts.subject, ts.students as _, ts.time as _, ts.timerange as _, ts.timezone)
		.execute(&db)
		.await?;
	Ok(())
}

pub async fn delete_timeslot_by_id(db: PgPool, u: UserId, id: Uuid) -> anyhow::Result<()> {
	sqlx::query!(
		"DELETE FROM timeslots WHERE user_id = $1 AND id = $2",
		u.as_str(),
		id
	)
	.execute(&db)
	.await?;

	Ok(())
}
