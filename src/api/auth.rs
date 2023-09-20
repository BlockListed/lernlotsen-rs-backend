use std::sync::Arc;

use axum::extract::{State, TypedHeader};
use axum::headers::Cookie;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Router};
use reqwest::StatusCode;

use tracing::warn;

use crate::api::util::WebResult;
use crate::auth::{Authenticator, AuthenticatorError, UserId};

use super::AppState;

pub fn router() -> Router<AppState> {
	Router::new().route("/", get(verify))
}

#[allow(clippy::unused_async)]
async fn verify(Extension(u): Extension<UserId>) -> (StatusCode, String) {
	(StatusCode::OK, format!("health check ok - {}", u.as_str()))
}

pub async fn auth_middleware<B>(
	State(auth): State<Arc<Authenticator>>,
	TypedHeader(cookies): TypedHeader<Cookie>,
	mut req: Request<B>,
	next: Next<B>,
) -> Response {
	let Some(auth_token) = cookies.get("auth_token") else {
		return WebResult::<&'static str, &'static str>::Err((StatusCode::UNAUTHORIZED, "missing authorization cookie").into()).into_response();
	};

	let auth_status = auth.verify(auth_token).await;

	match auth_status {
		Ok(u) => {
			let exts = req.extensions_mut();
			exts.insert(u);
		}
		Err(e) => match e {
			AuthenticatorError::ClaimsNotVerifiable(v) => {
				warn!(claims=?v, "authentication claims invalid");
				return WebResult::<&'static str, &'static str>::Err(
					(StatusCode::UNAUTHORIZED, "jwt (maybe no longer) valid").into(),
				)
				.into_response();
			}
			e => {
				warn!(%e, "invalid jwt");
				return WebResult::<&'static str, &'static str>::Err(
					(StatusCode::UNPROCESSABLE_ENTITY, "jwt invalid").into(),
				)
				.into_response();
			}
		},
	}

	next.run(req).await
}
