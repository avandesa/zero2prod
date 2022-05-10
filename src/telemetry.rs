use {
    tokio::task,
    tracing::{subscriber::set_global_default, Span, Subscriber},
    tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer},
    tracing_log::LogTracer,
    tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry},
};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|__| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber")
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = Span::current();
    task::spawn_blocking(move || current_span.in_scope(f))
}
