use axum::extract::{State, Query};
use axum::http::{StatusCode, HeaderMap};
use axum::response::IntoResponse;
use axum_session::SessionNullSession;
use openidconnect::reqwest::async_http_client;
use openidconnect::{AuthorizationCode, Nonce, TokenResponse, AccessTokenHash, OAuth2TokenResponse};
use serde::Deserialize;
use tracing::debug;

use super::AppState;

pub mod util;

pub async fn login(State(AppState { auth, .. }): State<AppState>, session: SessionNullSession) -> impl IntoResponse {
	let (url, csrf, nonce) = auth
		.authorize_url(
			openidconnect::core::CoreAuthenticationFlow::AuthorizationCode,
			openidconnect::CsrfToken::new_random,
			openidconnect::Nonce::new_random,
		)
		.url();

	session.set("csrf", csrf);
	session.set("nonce", nonce);
	
	let mut headers = HeaderMap::new();

	headers.append("Location", url.as_str().try_into().expect("Url contained invalid header characters for some reason"));

	(StatusCode::FOUND, headers)
}

#[derive(Deserialize)]
pub struct CallbackQuery {
	pub code: String,
	pub state: String,
}

pub async fn login_callback(State(AppState { auth, .. }): State<AppState>, session: SessionNullSession, Query(q): Query<CallbackQuery>) -> String {
	let Some(csrf): Option<String> = session.get("csrf") else {
		debug!("Request was missing csrf");
		return "Invalid session data".to_string();
	};

	let Some(nonce): Option<Nonce> = session.get("nonce") else {
		debug!("Request was missing nonce");
		return "Invalid session data".to_string();
	};

	if q.state != csrf {
		return "CSRF error".to_string();
	}
	
	let token_response = auth
		.exchange_code(AuthorizationCode::new(q.code))
		.request_async(async_http_client)
		.await
		.unwrap();

	let id_token = token_response
		.id_token()
		.expect("Server missing id_token");
	let claims = id_token.claims(&auth.id_token_verifier(), &nonce).unwrap();

	if let Some(expected) = claims.access_token_hash() {
		let actual = AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg().unwrap()).unwrap();

		if actual != *expected {
			return "Internal Server Error".to_string();
		}
	}

	let id = format!("{}:{}", claims.issuer().as_str(), claims.subject().as_str());

	session.set("id", id.clone());
	session.remove("csrf");
	session.remove("nonce");

	session.renew();

	id
}