mod birth;
mod configure_tracing;

use crate::configure_tracing::configure_tracing;
use anyhow::Context;
use bilrost::Message;
use bilrost::Oneof;
use birth::Birth;
use fluvio::RecordKey;
use tracing::{debug, info};

pub fn consumer() {
  configure_tracing();
  info!("Started Consumer");
}

pub async fn producer() {
  configure_tracing();
  info!("Started Producer");

  let birth = birth::Birth::new_now("Alice".to_owned());
  let msg = format!("Message sent to Fluvio: {:?}", birth);

  // bilrost scored well in this Rust serialization benchmark
  // https://github.com/djkoloski/rust_serialization_benchmark
  // let encoded = birth.encode_to_bytes();

  let wrapper = MessageWrapper {
    kind: MessageKind::Birth(birth),
  };

  let producer = fluvio::producer("myio").await.expect("Failed to create producer");

  producer
    .send(RecordKey::NULL, wrapper.encode_to_bytes())
    // .send(nanoid::nanoid!(8), encoded)
    .await
    .expect("Failed to send message");

  producer.flush().await.context("Failed to flush producer").unwrap();
  debug!(msg);
}

#[derive(Clone, Message, PartialEq)]
pub struct MessageWrapper {
  #[bilrost(oneof(100, 101))]
  pub kind: MessageKind,
}

#[derive(Clone, Oneof, PartialEq)]
pub enum MessageKind {
  None,
  #[bilrost(100)]
  Birth(Birth),
  #[bilrost(101)]
  Marriage(String),
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use bilrost::{BorrowedMessage, Message};
  use birth::Birth;
  use tokio::test;

  #[test]
  pub async fn test_serialize_roundtrip() {
    let name = (0..10_000).map(|_| "Alice").collect::<String>();
    let birth = Birth::new_now(name);
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
