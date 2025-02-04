use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn configure_tracing() {
  let fmt_layer = fmt::layer()
    // log as json
    // .json()
    // .with_current_span(true)
    // .with_line_number(false)
    // disable printing the name of the module in every log line.
    // .with_target(false)
    .without_time();

  let filter_layer = EnvFilter::try_from_default_env()
    .or_else(|_| {
      EnvFilter::try_new(
        "trace,async_io=debug,fluvio_protocol=debug,fluvio_socket=debug,fluvio=info,fluvio_socket=info,polling=debug,async_std=debug",
      )
    })
    .unwrap();

  tracing_subscriber::registry().with(fmt_layer).with(filter_layer).init();
}
