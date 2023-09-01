use openidconnect::{core::{CoreClient, CoreProviderMetadata}, IssuerUrl, ClientId, ClientSecret, RedirectUrl};
use openidconnect::reqwest::async_http_client;

use crate::configuration::Authorization;

pub async fn discover(auth_conf: &Authorization) -> CoreClient {
	let metadata = CoreProviderMetadata::discover_async(IssuerUrl::from_url(auth_conf.domain.clone()), async_http_client).await.unwrap();

	CoreClient::from_provider_metadata(metadata, ClientId::new(auth_conf.clientid.clone()), Some(ClientSecret::new(auth_conf.clientsecret.clone())))
		.set_redirect_uri(RedirectUrl::from_url(auth_conf.redirect.clone()))
}