use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

// TODO: move these types to model.rs
#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "session_status")]
#[sqlx(rename_all = "lowercase")]
pub enum SessionStatus {
	Initiated,
	Authenticated,
}

pub struct Session {
	pub id: Uuid,
	pub authenticated: SessionStatus,
	pub nonce: String,
	pub user_id: Option<String>,
	pub expires: DateTime<Utc>,
}

pub async fn create(db: &PgPool, nonce: &str) -> anyhow::Result<Uuid> {
	let id = Uuid::new_v4();
	let expires = Utc::now().checked_add_days(chrono::Days::new(7)).unwrap();
	let authenticated = SessionStatus::Initiated;

	sqlx::query!(
		"INSERT INTO sessions (id, authenticated, nonce, expires) VALUES ($1, $2, $3, $4)",
		id,
		authenticated as SessionStatus,
		nonce,
		expires
	)
	.execute(db)
	.await?;

	Ok(id)
}

pub async fn authenticate(db: &PgPool, id: Uuid, user_id: &str) -> anyhow::Result<()> {
	let authenticated = SessionStatus::Authenticated;
	sqlx::query!(
		"UPDATE sessions SET authenticated = $2, user_id = $3 WHERE id = $1",
		id,
		authenticated as SessionStatus,
		user_id
	)
	.execute(db)
	.await?;

	Ok(())
}

pub async fn get_session(db: &PgPool, id: Uuid) -> anyhow::Result<Session> {
	Ok(sqlx::query_as!(
		Session,
		r#"SELECT id, authenticated as "authenticated: SessionStatus", nonce, user_id, expires FROM sessions WHERE id = $1"#,
		id
	)
	.fetch_one(db)
	.await?)
}

pub async fn maybe_get_session(db: &PgPool, id: Uuid) -> anyhow::Result<Option<Session>> {
	Ok(sqlx::query_as!(
		Session,
		r#"SELECT id, authenticated as "authenticated: SessionStatus", nonce, user_id, expires FROM sessions WHERE id = $1"#,
		id
	)
	.fetch_optional(db)
	.await?)
}

// TODO: run this regularly
#[allow(dead_code)]
pub async fn delete_expired(db: &PgPool) -> anyhow::Result<()> {
	sqlx::query!("DELETE FROM sessions WHERE expires < NOW()")
		.execute(db)
		.await?;

	Ok(())
}
