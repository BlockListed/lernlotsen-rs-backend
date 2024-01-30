use std::sync::Arc;

use base64::Engine;
use chrono::{Duration, Utc};
use openid::{Client, Options};
use rand::RngCore;
use sqlx::PgPool;
use url::Url;

use crate::db::queries::{
	self,
	session::{Session, SessionStatus},
};

pub struct Authenticator {
	client: Client,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticatorError {
	#[error("invalid session")]
	InvalidSession,
	#[error("Not authorized")]
	NotAuthorized,
}

#[derive(Clone)]
pub struct UserId(Arc<str>);

impl UserId {
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl Authenticator {
	pub async fn new(
		client_id: String,
		client_secret: String,
		redirect: String,
		issuer: Url,
	) -> Authenticator {
		let client = Client::discover(client_id, Some(client_secret), Some(redirect), issuer)
			.await
			.unwrap();

		Authenticator { client }
	}

	pub async fn auth_url(&self, db: &PgPool) -> anyhow::Result<(Url, uuid::Uuid)> {
		let nonce = gen_nonce();

		let session = queries::session::create(db, &nonce).await?;

		// TODO: investigate if a state parameter is necesary
		let auth_url = self.client.auth_url(&Options {
			nonce: Some(nonce),
			..Default::default()
		});

		Ok((auth_url, session))
	}

	pub async fn authenticate(
		&self,
		db: &PgPool,
		id: uuid::Uuid,
		code: &str,
	) -> anyhow::Result<()> {
		let session = queries::session::get_session(db, id).await?;

		let token = self
			.client
			.authenticate(code, Some(session.nonce.as_str()), None)
			.await?
			.id_token
			.unwrap();

		let payload = token.payload().unwrap();

		let user_id = format!("{}:{}", payload.iss, payload.sub);

		queries::session::authenticate(db, id, &user_id).await?;

		Ok(())
	}

	pub async fn verify(
		&self,
		db: &PgPool,
		id: uuid::Uuid,
	) -> anyhow::Result<Result<UserId, AuthenticatorError>> {
		let Some(session) = queries::session::maybe_get_session(db, id).await? else {
			return Ok(Err(AuthenticatorError::InvalidSession));
		};

		if verify_session(&session) != Ok(()) {
			return Ok(Err(AuthenticatorError::NotAuthorized));
		}

		let user_id = match session.user_id {
			Some(u) => u,
			None => return Ok(Err(AuthenticatorError::InvalidSession)),
		};

		Ok(Ok(UserId(user_id.into())))
	}
}

fn verify_session(session: &Session) -> Result<(), ()> {
	verify_authenticated(session).and_then(|()| verify_session_expiry(session))
}

fn verify_authenticated(session: &Session) -> Result<(), ()> {
	if session.authenticated != SessionStatus::Authenticated {
		return Err(());
	}

	Ok(())
}

fn verify_session_expiry(session: &Session) -> Result<(), ()> {
	if session.expires.signed_duration_since(Utc::now()) < Duration::zero() {
		return Err(());
	}

	Ok(())
}

fn gen_nonce() -> String {
	let mut state_bytes = [0u8; 64];
	rand::thread_rng().fill_bytes(&mut state_bytes);

	#[allow(clippy::default_trait_access)]
	let engine = base64::engine::GeneralPurpose::new(&base64::alphabet::URL_SAFE, Default::default());

	engine.encode(state_bytes)
}

#[cfg(test)]
mod test {
	use chrono::Utc;

	use crate::db::queries::session::{Session, SessionStatus};

	use super::verify_session_expiry;

	#[test]
	fn test_verify_session_expiry() {
		let valid_session = Session {
			id: uuid::Uuid::new_v4(),
			authenticated: SessionStatus::Authenticated,
			nonce: "".into(),
			user_id: Some("hello".into()),
			expires: Utc::now() + chrono::Days::new(7),
		};

		let invalid_session = Session {
			id: uuid::Uuid::new_v4(),
			authenticated: SessionStatus::Authenticated,
			nonce: "".into(),
			user_id: Some("hello".into()),
			expires: Utc::now() - chrono::Days::new(7),
		};

		assert_eq!(verify_session_expiry(&valid_session), Ok(()));
		assert_eq!(verify_session_expiry(&invalid_session), Err(()));
	}
}
