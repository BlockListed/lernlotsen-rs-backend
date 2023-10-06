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

use std::time::Duration;

use auth::Authenticator;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use db::get_client;

mod api;
mod auth;
mod configuration;
mod db;
mod util;

#[tokio::main]
async fn main() {
	let _ = dotenvy::dotenv();

	{
		let env_filter =
			EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into());

		let formatting_layer = BunyanFormattingLayer::new("lernlotsen".into(), std::io::stdout);

		let registry = tracing_subscriber::registry()
			.with(env_filter)
			.with(JsonStorageLayer)
			.with(formatting_layer);

		tracing::subscriber::set_global_default(registry).unwrap();
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

	let c = get_client(&cfg).await;

	let auth = Authenticator::new(
		cfg.auth.domain.as_str(),
		Duration::from_secs(1800),
		&cfg.auth.audience,
	)
	.await;

	api::run(c, cfg, auth).await;
}
