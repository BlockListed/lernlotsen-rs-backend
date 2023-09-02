use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
	pub database: DatabaseConfig,
	pub auth: Authorization,
	pub hosturl: SocketAddr,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
	pub uri: String,
}

#[derive(Deserialize)]
pub struct Authorization {
	pub domain: url::Url,
	pub audience: String,
}
