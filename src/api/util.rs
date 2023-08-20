use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use serde::Serialize;

#[must_use]
pub enum WebResult<M: Serialize, E: Serialize> {
	Fine(StatusCode, M),
	NotFine(StatusCode, E)
}

#[derive(Serialize)]
struct Msg<T> {
	msg: T,
}

#[derive(Serialize)]
struct Error<T> {
	error: T,
}

impl<M, E> IntoResponse for WebResult<M, E> where M: Serialize, E: Serialize {
	fn into_response(self) -> axum::response::Response {
		match self {
			Self::Fine(c, m) => (c, Json(Msg { msg: m })).into_response(),
			Self::NotFine(c, e) => (c, Json(Error { error: e })).into_response(),
		}
	}
}

pub mod prelude {
	pub use super::WebResult;
	pub use super::WebResult::*;
}