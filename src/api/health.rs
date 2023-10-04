use axum::extract::State;

use crate::api::util::{TransposeResult, WebResult};
use super::AppState;
use super::handlers::health;

pub async fn health_check(
	State(AppState { db, .. }): State<AppState>,
) -> WebResult<&'static str, &'static str> {
	health::health_check(db).await.transpose_web()
}
