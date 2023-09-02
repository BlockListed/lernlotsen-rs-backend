use std::time::{Instant, Duration};

use tokio::sync::RwLock;

use tracing::debug;
use url::Url;
use alcoholic_jwt::{JWKS, Validation, ValidationError, token_kid, validate};

pub struct Authenticator {
	jwks_url: Url,
	cached_jwks: RwLock<(JWKS, Instant)>,
	validations: Vec<Validation>,
	max_age: Duration,
}

fn clone_validation(v: &Validation) -> Validation {
	match v {
		Validation::Audience(x) => Validation::Audience(x.clone()),
		Validation::Issuer(x) => Validation::Issuer(x.clone()),
		Validation::NotExpired => Validation::NotExpired,
		Validation::SubjectPresent => Validation::SubjectPresent,
	}
}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticatorError {
	#[error("JWT is expired")]
	ClaimsNotVerifiable(Vec<&'static str>),
	#[error("Invalid JWT")]
	Invalid(#[from] ValidationError),
	#[error("Invalid claims")]
	Claims(&'static str),
}

impl Authenticator {
	pub async fn new(jwks_domain: &str, max_age: Duration, audience: &str) -> Self {
		let jwks_url = get_jwks_url(jwks_domain);

		let jwks = (fetch_keys(&jwks_url).await, Instant::now());

		let validations = Vec::from([Validation::Audience(audience.to_string()), Validation::SubjectPresent, Validation::NotExpired]);

		Self {
			jwks_url,
			cached_jwks: RwLock::new(jwks),
			validations,
			max_age,
		}
	}

	pub async fn force_refetch(&self) {
		*self.cached_jwks.write().await = (fetch_keys(&self.jwks_url).await, Instant::now());
	}

	pub async fn refetch(&self) -> bool {
		if self.cached_jwks.read().await.1.elapsed() < self.max_age {
			return false
		}

		self.force_refetch().await;
		true
	}

	pub async fn verify(&self, token: &str) -> Result<String, AuthenticatorError> {
		let kid = token_kid(token)?.ok_or(AuthenticatorError::Claims("missing kid"))?;

		let jwt = {
			let cached_jwks_r = &self.cached_jwks.read().await.0;

			let jwk = cached_jwks_r.find(&kid).ok_or(AuthenticatorError::Claims("kid doesn't exist"))?;

			match validate(token, jwk, self.validations.iter().map(clone_validation).collect()) {
				Ok(jwt) => Ok(jwt),
				Err(e) => {
					match e {
						ValidationError::InvalidClaims(invalid) => {
							Err(AuthenticatorError::ClaimsNotVerifiable(invalid))
						}
						e => Err(AuthenticatorError::Invalid(e)),	
					}
				},
			}
		}?;

		let claims = jwt.claims.as_object().ok_or(AuthenticatorError::Claims("jwt claims aren't an object"))?;

		let issuer = claims.get("iss").map(|i| i.as_str()).flatten().ok_or(AuthenticatorError::Claims("invalid iss"))?;

		let subject = claims.get("sub").map(|i| i.as_str()).flatten().ok_or(AuthenticatorError::Claims("invalid sub"))?;

		Ok(format!("{}:{}", issuer, subject))
	}
}

async fn fetch_keys(jwks_url: &Url) -> JWKS {
	debug!(%jwks_url, "fetching jwt keys");

	reqwest::get(jwks_url.as_str()).await
		.unwrap()
		.json()
		.await
		.unwrap()
}

fn get_jwks_url(base: &str) -> Url {
	Url::options()
		.base_url(Some(Url::parse(base).unwrap()).as_ref())
		.parse("/.well-known/jwks.json")
		.expect("Invalid JWKS url")
}