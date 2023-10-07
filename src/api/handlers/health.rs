use axum::http::StatusCode;
use sqlx::PgPool;
use tracing::error;

use crate::api::util::WebError;

pub enum HealthCheckError {
	DatabaseUnavailable,
}

#[allow(clippy::from_over_into)]
impl Into<WebError<&'static str>> for HealthCheckError {
	fn into(self) -> WebError<&'static str> {
		use HealthCheckError::*;
		match self {
			DatabaseUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "database unavailable").into(),
		}
	}
}

pub async fn health_check(db: PgPool) -> Result<&'static str, HealthCheckError> {
	if database_test(db).await.is_err() {
		return Err(HealthCheckError::DatabaseUnavailable);
	}

	Ok("healthy")
}

pub async fn database_test(db: PgPool) -> Result<(), ()> {
	if let Err(e) = sqlx::query!("SELECT (1) as test").fetch_optional(&db).await
	{
		error!(err=%e, "health check error");
		return Err(());
	}

	Ok(())
}
