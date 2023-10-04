use std::time::Duration;

use axum::http::StatusCode;
use futures_util::FutureExt;
use mongodb::Database;
use tokio::time::timeout;

use crate::api::util::WebError;

pub enum HealthCheckError {
	DatabaseUnavailable,
}

impl Into<WebError<&'static str>> for HealthCheckError {
	fn into(self) -> WebError<&'static str> {
		use HealthCheckError::*;
		match self {
			DatabaseUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "database unavailable").into(),
		}
	}
}

pub async fn health_check(db: Database) -> Result<&'static str, HealthCheckError> {
	if !database_test(db).await {
		return Err(HealthCheckError::DatabaseUnavailable);
	}

	Ok("healthy")
}

pub async fn database_test(db: Database) -> bool {
	let res = timeout(
		Duration::from_millis(500),
		tokio::spawn(async move { db.list_collections(None, None).map(|v| v.unwrap()).await }),
	)
	.await;
	if let Ok(res) = res {
		res.is_ok()
	} else {
		false
	}
}
