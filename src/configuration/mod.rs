use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
	pub database: DatabaseConfig,
	pub hosturl: SocketAddr,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
	pub uri: String,
	pub database: String,
}
