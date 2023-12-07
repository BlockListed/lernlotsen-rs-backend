#![deny(clippy::todo)]
#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
// This lint is stupid
#![allow(clippy::module_name_repetitions)]
// I like the match operator
#![allow(clippy::single_match_else)]
#![allow(clippy::manual_let_else)]
// I like using globbed enums
#![allow(clippy::enum_glob_use)]

use auth::Authenticator;

use db::get_pool;

mod api;
mod auth;
mod configuration;
mod db;
mod util;

#[tokio::main]
async fn main() {
	if let Err(e) = dotenvy::dotenv() {
		println!("WARN: Error getting dotenv file: {e}");
	}

	match std::env::var("LOGGING").as_ref().map(String::as_str) {
		Ok("basic") | Err(std::env::VarError::NotPresent) => util::logging::basic_logging(),
		Ok("bunyan") => util::logging::bunyan_logging(),
		Ok("json") => util::logging::json_logging(),
		Ok(v) => {
			util::logging::basic_logging();
			tracing::warn!(
				v,
				"unknown value for LOGGING env var, using basic formatter"
			);
		}
		Err(std::env::VarError::NotUnicode(v)) => {
			util::logging::basic_logging();
			tracing::warn!(v=%v.to_string_lossy(), "invalid Unicode value for LOGGING env var, using basic formatter");
		}
	}

	let cfg_builder = config::Config::builder()
		.add_source(
			config::Environment::with_prefix("LELO")
				.separator("_")
				.try_parsing(true),
		)
		.build()
		.unwrap();

	let cfg: configuration::Config = cfg_builder.try_deserialize().unwrap();

	let db = get_pool(&cfg).await;

	let auth = Authenticator::new(
		cfg.auth.clientid.clone(),
		cfg.auth.clientsecret.clone(),
		cfg.auth.redirect.clone(),
		cfg.auth.issuer.clone(),
	)
	.await;

	api::run(db, cfg, auth).await;
}
