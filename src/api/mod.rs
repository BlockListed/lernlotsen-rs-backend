use axum::routing::{get, post};
use axum::{Router, Server};
use mongodb::Database;

use crate::configuration::Config;

mod timeslot;
mod entry;
mod util;

pub async fn run(db: Database, cfg: &Config) {
	let app = Router::new()
		.route("/timeslot", get(timeslot::query).post(timeslot::create))
		.route("/entries", post(entry::create))
		.with_state(db);

	Server::bind(&cfg.hosturl)
		.serve(app.into_make_service())
		.await
		.unwrap();
}
