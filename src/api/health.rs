use axum::extract::State;

use super::handlers::health;
use super::AppState;
use crate::api::util::{TransposeResult, WebResult};

pub async fn health_check(
	State(AppState { db, .. }): State<AppState>,
) -> WebResult<&'static str, &'static str> {
	health::health_check(db).await.transpose_web()
}
