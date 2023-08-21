use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::{Router, Server};

use mongodb::Database;

use tower_http::trace::TraceLayer;
use tower_request_id::{RequestId, RequestIdLayer};
use tracing::{info, info_span};

use crate::configuration::Config;

mod entry;
mod logic;
mod timeslot;
#[macro_use]
mod util;

pub async fn run(db: Database, cfg: &Config) {
	let app = Router::new()
		.route("/timeslots", get(timeslot::query).post(timeslot::create))
		.route("/entries", get(entry::query).post(entry::create))
		.route("/entries/missing", get(entry::missing))
		.with_state(db)
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

	info!(uri=%cfg.hosturl, "Starting server");

	Server::bind(&cfg.hosturl)
		.serve(app.into_make_service())
		.await
		.unwrap();
}
