use axum::extract::State;
use axum::http::StatusCode;

use sqlx::PgPool;

use tracing::error;

use crate::api::AppState;
use crate::api::util::prelude::*;

pub enum HealthCheckError {
	DatabaseUnavailable,
}

impl From<HealthCheckError> for WebError<&'static str> {
	fn from(v: HealthCheckError) -> WebError<&'static str> {
		use HealthCheckError::*;
		match v {
			DatabaseUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "database unavailable").into(),
		}
	}
}

pub async fn database_test(db: &PgPool) -> Result<(), ()> {
	if let Err(e) = sqlx::query!("SELECT (1) as test").fetch_optional(db).await {
		error!(err=%e, "health check error");
		return Err(());
	}

	Ok(())
}

pub async fn health_check(
	State(AppState { db, .. }): State<AppState>,
) -> WebResult<&'static str, &'static str> {
	if database_test(&db).await.is_err() {
		return Err(HealthCheckError::DatabaseUnavailable)?;
	}

	Ok("healthy".into())
}
