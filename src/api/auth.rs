use axum::extract::{Query, State, TypedHeader};
use axum::headers::Cookie;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use tracing::error;
use url::Url;

use crate::api::util::{WebError, WebResult};
use crate::auth::{AuthenticatorError, UserId};

use super::util::TransposeResult;
use super::AppState;

pub async fn sign_in(
	State(AppState { db, auth, .. }): State<AppState>,
) -> WebResult<OidcData, &'static str> {
	let (auth_url, session_id) = match auth.auth_url(db).await {
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

#[derive(Serialize)]
pub struct OidcData {
	auth_url: Url,
	session_id: uuid::Uuid,
}

pub async fn authenticate(
	State(AppState { db, auth, .. }): State<AppState>,
	Query(q): Query<OidcCallback>,
	TypedHeader(cookies): TypedHeader<Cookie>,
) -> WebResult<&'static str, &'static str> {
	let session_id = extract_session_id(&cookies)?;

	match auth.authenticate(db, session_id, &q.code).await {
		Ok(()) => (),
		Err(e) => {
			error!(?e, "couldn't authenticate user");
			return WebResult::Err(WebError::internal_server_error());
		}
	}

	Ok("success".into())
}

type StrWebResult = WebResult<&'static str, &'static str>;

pub async fn auth_middleware<B>(
	State(AppState { db, auth, .. }): State<AppState>,
	TypedHeader(cookies): TypedHeader<Cookie>,
	mut req: Request<B>,
	next: Next<B>,
) -> Response {
	let session_id = match extract_session_id(&cookies) {
		Ok(s) => s,
		Err(e) => return StrWebResult::Err(e.into()).into_response(),
	};

	let user_id = match auth.verify(db, session_id).await {
		Ok(o) => match o {
			Ok(u) => u,
			Err(AuthenticatorError::NotAuthorized) => {
				return StrWebResult::Err(
					(StatusCode::UNAUTHORIZED, "incomplete or expired session").into(),
				)
				.into_response()
			}
			Err(AuthenticatorError::InvalidSession) => {
				return StrWebResult::Err((StatusCode::UNAUTHORIZED, "invalid session").into())
					.into_response()
			}
		},
		Err(e) => {
			error!(?e, "server error while processing session");
			return StrWebResult::Err(WebError::internal_server_error()).into_response();
		}
	};

	req.extensions_mut().insert(user_id);

	next.run(req).await
}

// TODO: maybe skip the allocation by accessing the Arc<str> inside the user_id directly
pub async fn user_id(Extension(user_id): Extension<UserId>) -> WebResult<String, &'static str> {
	Ok::<_, (StatusCode, &'static str)>(user_id.as_str().to_owned()).transpose_web()
}

fn extract_session_id(cookies: &Cookie) -> Result<uuid::Uuid, (StatusCode, &'static str)> {
	let raw_session_id = match cookies.get("session_id") {
		Some(s) => s,
		None => return Err((StatusCode::UNAUTHORIZED, "missing session_id cookie")),
	};

	let session_id = match uuid::Uuid::parse_str(raw_session_id) {
		Ok(s) => s,
		Err(e) => {
			error!(?e, "failed to parse session id");
			return Err((StatusCode::BAD_REQUEST, "session_id is not a valid uuid"));
		}
	};

	Ok(session_id)
}
