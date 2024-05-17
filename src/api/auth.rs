use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

use tracing::error;
use url::Url;

use crate::api::util::{WebError, WebResult};
use crate::auth::{AuthenticatorError, UserId};

use super::util::TransposeResult;
use super::AppState;

#[derive(Serialize)]
pub struct OidcData {
	auth_url: Url,
	session_id: uuid::Uuid,
}

pub async fn sign_in(
	State(AppState { db, auth, .. }): State<AppState>,
) -> WebResult<OidcData, &'static str> {
	let (auth_url, session_id) = match auth.auth_url(&db).await {
		Ok(d) => d,
		Err(e) => {
			error!(?e, "error while handling request");
			return Err(WebError::internal_server_error());
		}
	};

	Ok::<_, WebError<&'static str>>(OidcData {
		auth_url,
		session_id,
	})
	.transpose_web()
}

#[derive(Deserialize)]
pub struct OidcCallback {
	code: String,
}

pub async fn authenticate(
	State(AppState { db, auth, .. }): State<AppState>,
	Query(q): Query<OidcCallback>,
	cookies: CookieJar,
) -> WebResult<&'static str, &'static str> {
	let session_id = extract_session_id(&cookies)?;

	match auth.authenticate(&db, session_id, &q.code).await {
		Ok(()) => (),
		Err(e) => {
			error!(?e, "couldn't authenticate user");
			return WebResult::Err(WebError::internal_server_error());
		}
	}

	Ok("success".into())
}

#[allow(clippy::enum_variant_names)]
pub enum AuthError {
	SessionIncompleteOrExpired,
	SessionInvalid,
	SessionMissing,
	SessionIdInvalid,
}

impl From<AuthError> for WebError<&'static str> {
	fn from(value: AuthError) -> WebError<&'static str> {
		match value {
			AuthError::SessionIncompleteOrExpired => {
				(StatusCode::UNAUTHORIZED, "incomplete or expired session").into()
			}
			AuthError::SessionInvalid => (StatusCode::UNAUTHORIZED, "invalid session").into(),
			AuthError::SessionMissing => {
				(StatusCode::UNAUTHORIZED, "session id cookie missing").into()
			}
			AuthError::SessionIdInvalid => (StatusCode::BAD_REQUEST, "invalid sessionid").into(),
		}
	}
}

pub async fn auth_middleware(
	State(AppState { db, auth, .. }): State<AppState>,
	cookies: CookieJar,
	mut req: Request,
	next: Next,
) -> Response {
	let session_id = match extract_session_id(&cookies) {
		Ok(s) => s,
		Err(e) => return WebError::<&'static str>::from(e).into_response(),
	};

	let user_id = match auth.verify(&db, session_id).await {
		Ok(o) => match o {
			Ok(u) => u,
			Err(AuthenticatorError::NotAuthorized) => {
				return WebError::<&'static str>::from(AuthError::SessionIncompleteOrExpired)
					.into_response()
			}
			Err(AuthenticatorError::InvalidSession) => {
				return WebError::<&'static str>::from(AuthError::SessionInvalid).into_response()
			}
		},
		Err(e) => {
			error!(?e, "server error while processing session");
			return WebError::<&'static str>::internal_server_error().into_response();
		}
	};

	req.extensions_mut().insert(user_id);

	next.run(req).await
}

// TODO: maybe skip the allocation by accessing the Arc<str> inside the user_id directly
pub async fn user_id(Extension(user_id): Extension<UserId>) -> WebResult<String, &'static str> {
	Ok::<_, WebError<&'static str>>(user_id.as_str().to_owned().into())
}

fn extract_session_id(cookies: &CookieJar) -> Result<uuid::Uuid, AuthError> {
	let raw_session_id = match cookies.get("session_id") {
		Some(s) => s,
		None => {
            tracing::trace!("session id missing from request");
            return Err(AuthError::SessionMissing)
        },
	};

	let session_id = match uuid::Uuid::parse_str(raw_session_id.value()) {
		Ok(s) => s,
		Err(e) => {
			error!(?e, "failed to parse session id");
			return Err(AuthError::SessionIdInvalid);
		}
	};

	Ok(session_id)
}
