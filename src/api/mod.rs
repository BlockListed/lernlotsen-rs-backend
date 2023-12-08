use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::routing::{delete, get};
use axum::Router;

use sqlx::PgPool;
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
	pub db: PgPool,
	pub auth: Arc<Authenticator>,
	pub cfg: Arc<Config>,
}

pub async fn run(db: PgPool, cfg: Config, auth: Authenticator) {
	let hosturl = cfg.hosturl;

	let cfg = Arc::new(cfg);

	let state = AppState {
		db,
		auth: Arc::new(auth),
		cfg: cfg.clone(),
	};

	let app = Router::new()
		.route("/timeslots", get(timeslot::query).post(timeslot::create))
		.route("/timeslots/export", get(timeslot::export))
		.route("/timeslots/:id", delete(timeslot::delete))
		.route(
			"/timeslots/:id/entries",
			get(entry::query).post(entry::create),
		)
		.route("/timeslots/:id/entries/next", get(entry::next))
		.route("/timeslots/:id/entries/missing", get(entry::missing))
		.route("/timeslots/:id/entries/:index", delete(entry::delete))
		.route("/timeslots/information", get(timeslot::information))
		.route("/auth/user_id", get(auth::user_id))
		.layer(axum::middleware::from_fn_with_state(
			state.clone(),
			auth::auth_middleware,
		))
		.route("/auth/oidc_callback", get(auth::authenticate))
		.route("/auth/oidc_login", get(auth::sign_in))
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
