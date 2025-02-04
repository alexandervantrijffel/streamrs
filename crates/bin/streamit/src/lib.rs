mod birth;

use anyhow::Context;
use bilrost::Message;
use fluvio::RecordKey;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn consumer() {
  configure_tracing();
  info!("Started Consumer");
}

pub async fn producer() {
  configure_tracing();
  info!("Started Producer");

  let birth = birth::Birth::new_now("Alice".to_owned());
  let encoded = birth.encode_to_bytes();

  let producer = fluvio::producer("myio").await.expect("Failed to create producer");

  producer
    .send(RecordKey::NULL, encoded)
    // .send(nanoid::nanoid!(8), encoded)
    .await
    .expect("Failed to send message");

  producer.flush().await.context("Failed to flush producer").unwrap();

  debug!("Message sent to Fluvio: {:?}", birth);
}

fn configure_tracing() {
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
