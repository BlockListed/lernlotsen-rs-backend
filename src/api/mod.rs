use axum::routing::get;
use axum::{Router, Server};
use mongodb::Database;

use crate::configuration::Config;

mod timeslot;
mod entry;

pub async fn run(db: Database, cfg: &Config) {
	let app = Router::new()
		.route("/timeslot", get(timeslot::query).post(timeslot::create))
		.with_state(db);

	Server::bind(&cfg.hosturl)
		.serve(app.into_make_service())
		.await
		.unwrap();
}
