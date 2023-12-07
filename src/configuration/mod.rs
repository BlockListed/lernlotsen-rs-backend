use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
	pub database: DatabaseConfig,
	pub auth: Authorization,
	pub hosturl: SocketAddr,
	pub tls: Option<Tls>,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
	pub uri: String,
}

#[derive(Deserialize)]
pub struct Authorization {
	pub client_id: String,
	pub client_secret: String,
	pub redirect: String,
	pub issuer: url::Url,
}

#[derive(Deserialize)]
pub struct Tls {
	pub certpath: String,
	pub keypath: String,
}
