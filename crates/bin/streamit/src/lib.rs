pub mod configure_tracing;
pub mod consumer;
pub mod message;

use anyhow::Context;
use bilrost::Message;
use fluvio::RecordKey;
use message::{Birth, MessageKind, MessageWrapper};
use tracing::debug;

pub const MYIO_TOPIC: &str = "myio";

pub async fn producer() -> anyhow::Result<()> {
  let birth = Birth::new("Alice".to_owned());
  let msg = format!("Message sent to Fluvio: {:?}", birth);

  let wrapper = MessageWrapper {
    kind: MessageKind::Birth(birth),
  };

  let producer = fluvio::producer(MYIO_TOPIC)
    .await
    .context("Failed to create producer")?;

  producer
    .send(RecordKey::NULL, wrapper.encode_to_bytes())
    // .send(nanoid::nanoid!(8), encoded)
    .await
    .context("Failed to send message")?;

  producer.flush().await.context("Failed to flush producer").unwrap();
  debug!(msg);
  Ok(())
}
