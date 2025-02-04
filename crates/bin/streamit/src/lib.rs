pub mod configure_tracing;
pub mod consumer;
pub mod message;

use anyhow::Context;
use bilrost::Message;
use fluvio::RecordKey;
use message::{Birth, MessageKind, MessageWrapper};
use tracing::debug;

pub async fn producer() -> anyhow::Result<()> {
  let birth = Birth::new("Alice".to_owned());
  let msg = format!("Message sent to Fluvio: {:?}", birth);

  let wrapper = MessageWrapper {
    kind: MessageKind::Birth(birth),
  };

  let producer = fluvio::producer("myio").await.context("Failed to create producer")?;

  producer
    .send(RecordKey::NULL, wrapper.encode_to_bytes())
    // .send(nanoid::nanoid!(8), encoded)
    .await
    .context("Failed to send message")?;

  producer.flush().await.context("Failed to flush producer").unwrap();
  debug!(msg);
  Ok(())
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use Birth;
  use bilrost::{BorrowedMessage, Message};
  use tokio::test;

  #[test]
  pub async fn test_serialize_roundtrip() {
    let name = (0..10_000).map(|_| "Alice").collect::<String>();
    let birth = Birth::new(name);
    let wrapper = MessageWrapper {
      kind: MessageKind::Birth(birth),
    };

    let encoded = wrapper.encode_to_bytes();
    let decoded = MessageWrapper::decode_borrowed(&encoded).unwrap();
    match decoded.kind {
      MessageKind::Birth(birth) => {
        assert_eq!(birth, birth);
      }
      _ => panic!("Expected Birth"),
    }
  }
}
