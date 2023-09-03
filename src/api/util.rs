use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use serde::Serialize;

#[must_use]
pub enum WebResult<M: Serialize, E: Serialize> {
	Fine(StatusCode, M),
	NotFine(StatusCode, E),
}

#[derive(Serialize)]
struct Msg<T> {
	msg: T,
}

#[derive(Serialize)]
struct Error<T> {
	error: T,
}

impl<M, E> IntoResponse for WebResult<M, E>
where
	M: Serialize,
	E: Serialize,
{
	fn into_response(self) -> axum::response::Response {
		match self {
			Self::Fine(c, m) => (c, Json(Msg { msg: m })).into_response(),
			Self::NotFine(c, e) => (c, Json(Error { error: e })).into_response(),
		}
	}
}

impl<O, DontCare, M: Serialize> From<Result<O, DontCare>> for WebResult<M, &'static str>
where
	O: Into<M>,
{
	fn from(value: Result<O, DontCare>) -> Self {
		match value {
			Ok(m) => WebResult::Fine(StatusCode::OK, m.into()),
			Err(_) => {
				WebResult::NotFine(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
			}
		}
	}
}

#[macro_export]
macro_rules! try_web {
	($res:expr) => {
		match $res {
			$crate::api::util::WebResult::Fine(c, m) => (c, m),
			$crate::api::util::WebResult::NotFine(c, m) => {
				return $crate::api::util::WebResult::NotFine(c, m)
			}
		}
	};
}

pub mod prelude {
	pub use super::WebResult;
	pub use super::WebResult::*;
}
