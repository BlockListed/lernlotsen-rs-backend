use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;

use mongodb::Database;

use tower_http::trace::TraceLayer;
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{info, info_span};

use crate::auth::Authenticator;
use crate::configuration::Config;

mod auth;
mod entry;
mod handlers;
mod health;
mod logic;
mod timeslot;
#[macro_use]
mod util;

#[derive(Clone)]
pub struct AppState {
	pub db: Database,
	pub auth: Arc<Authenticator>,
	pub cfg: Arc<Config>,
}

pub async fn run(db: Database, cfg: Config, auth: Authenticator) {
	let hosturl = cfg.hosturl;

	let auth = Arc::new(auth);

	let cfg = Arc::new(cfg);

	let state = AppState {
		db,
		auth: auth.clone(),
		cfg: cfg.clone(),
	};

	let app = Router::new()
		.route("/timeslots", get(timeslot::query).post(timeslot::create))
		.route("/timeslots/export", get(timeslot::export))
		.route(
			"/timeslots/:id/entries",
			get(entry::query).post(entry::create),
		)
		.route("/timeslots/:id/entries/next", get(entry::next))
		.route("/timeslots/:id/entries/missing", get(entry::missing))
		.route("/timeslots/information", get(timeslot::information))
		.nest("/verify", auth::router())
		.layer(axum::middleware::from_fn_with_state(
			auth,
			auth::auth_middleware,
		))
		.route("/health_check", get(health::health_check))
		.layer(
			TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
				let request_id = request
					.extensions()
					.get::<RequestId>()
					.map_or_else(|| "unknown".into(), ToString::to_string);

				info_span!("request", id = %request_id, method = %request.method(), uri = %request.uri())
			}),
		)
		.layer(RequestIdLayer)
		.with_state(state);

	info!(uri=%hosturl, "Starting server");

	match &cfg.tls {
		Some(tls) => {
			let tls_config =
				axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.certpath, &tls.keypath)
					.await
					.expect("Couldn't read tls cert/key");

			axum_server::bind_rustls(hosturl, tls_config)
				.serve(app.into_make_service())
				.await
				.unwrap();
		}
		None => {
			axum_server::bind(hosturl)
				.serve(app.into_make_service())
				.await
				.unwrap();
		}
	}
}
