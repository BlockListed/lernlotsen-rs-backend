use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use serde::Serialize;

pub type WebResult<T, E> = Result<WebSuccess<T>, WebError<E>>;

// Probably not the correct term.
pub trait TransposeResult<T, E> {
	fn transpose_web(self) -> Result<T, E>;
}

impl<T: Into<A>, E: Into<B>, A, B> TransposeResult<A, B> for Result<T, E> {
	fn transpose_web(self) -> Result<A, B> {
		match self {
			Ok(ok) => Ok(ok.into()),
			Err(err) => Err(err.into()),
		}
	}
}

#[derive(Serialize)]
struct Msg<T> {
	msg: T,
}

#[derive(Serialize)]
struct Error<T> {
	error: T,
}

pub struct WebSuccess<M: Serialize> {
	pub msg: M,
	pub status: axum::http::StatusCode,
}

impl<M: Serialize> IntoResponse for WebSuccess<M> {
	fn into_response(self) -> axum::response::Response {
		(self.status, Json(Msg { msg: self.msg })).into_response()
	}
}

impl<M: Serialize> From<M> for WebSuccess<M> {
	fn from(value: M) -> Self {
		WebSuccess {
			msg: value,
			status: StatusCode::OK,
		}
	}
}

impl<M: Serialize> From<(StatusCode, M)> for WebSuccess<M> {
	fn from(value: (StatusCode, M)) -> Self {
		WebSuccess {
			msg: value.1,
			status: value.0,
		}
	}
}

pub struct WebError<E: Serialize> {
	pub err: E,
	pub status: axum::http::StatusCode,
}

impl<E: From<&'static str> + Serialize> WebError<E> {
	pub fn internal_server_error() -> Self {
		WebError {
			err: "internal server error".into(),
			status: StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

impl<E: Serialize> IntoResponse for WebError<E> {
	fn into_response(self) -> axum::response::Response {
		(self.status, Json(Error { error: self.err })).into_response()
	}
}

impl<E: Serialize> From<(StatusCode, E)> for WebError<E> {
	fn from(value: (StatusCode, E)) -> Self {
		WebError {
			err: value.1,
			status: value.0,
		}
	}
}

impl<E> From<anyhow::Error> for WebError<E>
where
	E: From<&'static str> + Serialize,
{
	fn from(e: anyhow::Error) -> Self {
		tracing::error!(%e, "error while handling request");
		WebError::internal_server_error()
	}
}

// This shit is actually so annoying it needs it's own implementation
impl<E> From<std::num::TryFromIntError> for WebError<E>
where
	E: From<&'static str> + Serialize,
{
	fn from(e: std::num::TryFromIntError) -> Self {
		let anyhow_err: anyhow::Error = e.into();
		anyhow_err.into()
	}
}

impl<E> From<std::fmt::Error> for WebError<E>
where
	E: From<&'static str> + Serialize,
{
	fn from(e: std::fmt::Error) -> Self {
		let anyhow_err: anyhow::Error = e.into();
		anyhow_err.into()
	}
}

pub mod prelude {
	#![allow(unused_imports)]

	pub use super::TransposeResult;
	pub use super::WebError;
	pub use super::WebResult;
	pub use super::WebSuccess;
}
