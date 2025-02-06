use anyhow::Context;
use bilrost::Message;
use fluvio::RecordKey;
use streamitlib::{
  configure_tracing::init,
  message::{Birth, MessageWrapper},
  topic::MYIO_TOPIC,
};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
  _ = init();
  info!("Starting Producer");
  _ = producer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}

async fn producer() -> anyhow::Result<()> {
  let birth = Birth::new("Alice".to_owned());
  let msg = format!("Message sent to Fluvio: {birth:?}");
  let wrapper = MessageWrapper::from(birth);

  let producer = fluvio::producer(MYIO_TOPIC)
    .await
    .context("Failed to create producer")?;

  producer
    .send(RecordKey::NULL, wrapper.encode_to_bytes())
    // or send a record with a key:
    // .send(nanoid::nanoid!(8), encoded)
    .await
    .context("Failed to send message")?;

  producer.flush().await.context("Failed to flush producer")?;
  debug!(msg);
  Ok(())
}
