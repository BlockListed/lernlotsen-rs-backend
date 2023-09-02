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
use crate::auth::{Authenticator, AuthenticatorError};

use super::AppState;

pub fn router() -> Router<AppState> {
	Router::new().route("/", get(verify))
}

#[axum::debug_handler]
async fn verify(Extension(UserId(u)): Extension<UserId>) -> (StatusCode, String) {
	(StatusCode::OK, format!("health check ok - {}", u))
}

#[derive(Clone)]
pub struct UserId(pub String);

pub async fn auth_middleware<B>(
	State(auth): State<Arc<Authenticator>>,
	TypedHeader(cookies): TypedHeader<Cookie>,
	mut req: Request<B>,
	next: Next<B>,
) -> Response {
	let Some(auth_token) = cookies.get("auth_token") else {
		return WebResult::NotFine::<(), _>(StatusCode::UNAUTHORIZED, "missing authorization cookie").into_response();
	};

	let auth_status = auth.verify(auth_token).await;

	match auth_status {
		Ok(t) => {
			let exts = req.extensions_mut();
			exts.insert(UserId(t));
		}
		Err(e) => match e {
			AuthenticatorError::ClaimsNotVerifiable(v) => {
				warn!(claims=?v, "authentication claims invalid");
				return WebResult::NotFine::<(), _>(
					StatusCode::UNAUTHORIZED,
					"jwt (maybe no longer) valid",
				)
				.into_response();
			}
			e => {
				warn!(%e, "invalid jwt");
				return WebResult::NotFine::<(), _>(
					StatusCode::UNPROCESSABLE_ENTITY,
					"jwt invalid",
				)
				.into_response();
			}
		},
	}

	next.run(req).await
}
