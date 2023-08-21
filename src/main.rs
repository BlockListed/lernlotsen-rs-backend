use db::get_db;

mod api;
mod configuration;
mod db;

#[tokio::main]
async fn main() {
	let _ = dotenvy::dotenv();

	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "info,tower_http=debug".into()),
		)
		.init();

	let cfg_builder = config::Config::builder()
		.add_source(
			config::Environment::with_prefix("LELO")
				.separator("_")
				.try_parsing(true),
		)
		.build()
		.unwrap();

	let cfg: configuration::Config = cfg_builder.try_deserialize().unwrap();

	let db = get_db(&cfg).await;

	api::run(db, &cfg).await;
}
