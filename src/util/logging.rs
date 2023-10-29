use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{FmtSpan, json};
use tracing_subscriber::layer::SubscriberExt;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};

fn env_filter() -> EnvFilter {
	EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into())
}

pub fn bunyan_logging() {
	let formatting_layer = BunyanFormattingLayer::new("lernlotsen".into(), std::io::stdout);

	let registry = tracing_subscriber::registry()
		.with(env_filter())
		.with(JsonStorageLayer)
		.with(formatting_layer);

	tracing::subscriber::set_global_default(registry).unwrap();
}

pub fn json_logging() {
	tracing_subscriber::fmt()
		.with_env_filter(env_filter())
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.event_format(json().flatten_event(true).with_current_span(true).with_span_list(true))
		.init();
}

pub fn basic_logging() {
	tracing_subscriber::fmt()
		.with_env_filter(env_filter())
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.init();
}