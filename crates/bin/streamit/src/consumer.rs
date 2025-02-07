use async_trait::async_trait;
use fluvio::{
  consumer::{ConsumerConfigExtBuilder, ConsumerStream, Record as ConsumerRecord},
  Fluvio, Offset,
};
use fluvio_protocol::link::ErrorCode;
use std::pin::Pin;

#[async_trait]
pub trait Consumer: Send + Sync {
  async fn consume(
    &self,
    topic: &str,
    offset: Offset,
  ) -> std::result::Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error>;
}

pub struct FluvioConsumer {}

#[async_trait]
impl Consumer for FluvioConsumer {
  async fn consume(
    &self,
    topic: &str,
    offset: Offset,
  ) -> Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error> {
    Ok(Box::pin(
      Fluvio::connect()
        .await?
        .consumer_with_config(
          ConsumerConfigExtBuilder::default()
            .topic(topic)
            .partition(0)
            .offset_start(offset)
            .build()?,
        )
        .await?,
    ))
  }
}
