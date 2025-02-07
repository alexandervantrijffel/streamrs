use async_trait::async_trait;
use fluvio::{
  consumer::{ConsumerConfigExtBuilder, ConsumerStream, OffsetManagementStrategy, Record as ConsumerRecord},
  Fluvio, Offset,
};
use fluvio_protocol::link::ErrorCode;
use std::{pin::Pin, time::Duration};

#[async_trait]
pub trait Consumer: Send + Sync {
  async fn consume(
    &self,
    topic: &str,
    consumer_name: &str,
    offset_strategry: OffsetManagementStrategy,
    offset_start: Offset,
  ) -> std::result::Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error>;
}

pub struct FluvioConsumer {}

#[async_trait]
impl Consumer for FluvioConsumer {
  async fn consume(
    &self,
    topic: &str,
    consumer_name: &str,
    offset_strategry: OffsetManagementStrategy,
    offset_start: Offset,
  ) -> Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error> {
    Ok(Box::pin(
      Fluvio::connect()
        .await?
        .consumer_with_config(
          ConsumerConfigExtBuilder::default()
            .topic(topic)
            .partition(0)
            .offset_consumer(consumer_name)
            .offset_strategy(offset_strategry)
            .offset_flush(Duration::from_secs(5))
            .offset_start(offset_start)
            .build()?,
        )
        .await?,
    ))
  }
}
