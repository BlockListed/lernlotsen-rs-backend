use std::future::Future;

use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use serde::Serialize;

use tokio::task::JoinHandle;

use tracing::{Instrument, Span};

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

impl<O, Er, M: Serialize, E: Serialize> From<Result<O, Er>> for WebResult<M, E>
where
	O: Into<M>,
	Er: Into<E>,
{
	fn from(value: Result<O, Er>) -> Self {
		match value {
			Ok(m) => WebResult::Fine(StatusCode::OK, m.into()),
			Err(e) => WebResult::NotFine(StatusCode::INTERNAL_SERVER_ERROR, e.into()),
		}
	}
}

pub fn cvt_res<O, Er, M: Serialize, E: Serialize>(
	value: Result<(StatusCode, O), (StatusCode, Er)>,
) -> WebResult<M, E>
where
	O: Into<M>,
	Er: Into<E>,
{
	match value {
		Ok((c, m)) => WebResult::Fine(c, m.into()),
		Err((c, e)) => WebResult::NotFine(c, e.into()),
	}
}

pub fn spawn_in_current_span<T: Send + 'static>(
	f: impl Future<Output = T> + Send + 'static,
) -> JoinHandle<T> {
	tokio::spawn(f.instrument(Span::current()))
}

pub mod prelude {
	pub use super::WebResult;
	pub use super::WebResult::*;

	pub use super::spawn_in_current_span;
}

#[macro_export]
macro_rules! handle_db {
	($e:expr, $err:expr) => {
		match $e {
			Ok(x) => x,
			Err(e) => {
				::tracing::error!(%e, "encountered database error");
				return $crate::api::util::WebResult::NotFine(::axum::http::StatusCode::INTERNAL_SERVER_ERROR, $err);
			}
		}
	};
}
