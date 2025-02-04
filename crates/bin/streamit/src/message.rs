use bilrost::Message;
use bilrost::Oneof;
use time::OffsetDateTime;

// bilrost scored well in this Rust serialization benchmark
// https://github.com/djkoloski/rust_serialization_benchmark

#[derive(Debug, Clone, Eq, Message, PartialEq)]
// #[bilrost(distinguished)]
pub struct MessageWrapper {
  #[bilrost(oneof(100, 101))]
  pub kind: MessageKind,
}

#[derive(Debug, Eq, Clone, Oneof, PartialEq)]
// #[bilrost(distinguished)]
pub enum MessageKind {
  None,
  #[bilrost(100)]
  Birth(Birth),
  #[bilrost(101)]
  Marriage(String),
}

#[derive(Debug, Eq, PartialEq, Clone, Message)]
pub struct Birth {
  name: String,
  born_at_epoch: i64,
}

impl Birth {
  pub fn new(name: String) -> Self {
    Self {
      born_at_epoch: OffsetDateTime::now_utc().unix_timestamp(),
      name,
    }
  }
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
