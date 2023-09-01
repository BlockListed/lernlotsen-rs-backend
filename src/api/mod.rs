use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::{Router, Server};

use axum_session::{SessionConfig, SessionLayer, SessionNullSessionStore};
use mongodb::Database;

use openidconnect::core::CoreClient;
use tower_http::trace::TraceLayer;
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{info, info_span};

use crate::configuration::Config;

mod auth;
mod entry;
mod logic;
mod timeslot;
#[macro_use]
mod util;

#[derive(Clone)]
pub struct AppState {
	pub db: Database,
	pub auth: CoreClient,
	pub cfg: Arc<Config>,
}

pub async fn run(db: Database, cfg: Config) {
	let hosturl = cfg.hosturl.clone();

	let state = AppState {
		db,
		auth: auth::util::discover(&cfg.auth).await,
		cfg: cfg.into(),
	};

	let session_config = SessionConfig::new();

	let session_store = SessionNullSessionStore::new(None, session_config).await.unwrap();

	let app = Router::new()
		.route("/timeslots", get(timeslot::query).post(timeslot::create))
		.route("/timeslots/:id/entries", get(entry::query).post(entry::create))
		.route("/timeslots/:id/entries/missing", get(entry::missing))
		.route("/login", get(auth::login))
		.route("/login_callback", get(auth::login_callback))
		.with_state(state)
		.layer(SessionLayer::new(session_store))
		.layer(
			TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
				let request_id = request
					.extensions()
					.get::<RequestId>()
					.map(ToString::to_string)
					.unwrap_or_else(|| "unknown".into());

				info_span!("request", id = %request_id, method = %request.method(), uri = %request.uri())
			}),
		)
		.layer(RequestIdLayer);

	info!(uri=%hosturl, "Starting server");

	Server::bind(&hosturl)
		.serve(app.into_make_service())
		.await
		.unwrap();
}
