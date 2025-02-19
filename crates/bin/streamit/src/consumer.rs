use anyhow::Result;
use async_trait::async_trait;
use fluvio::{
  consumer::{ConsumerConfigExtBuilder, ConsumerStream, OffsetManagementStrategy, Record as ConsumerRecord},
  Fluvio, Offset,
};
use fluvio_protocol::link::ErrorCode;
use futures::{stream::Next, StreamExt};
use std::{pin::Pin, time::Duration};

pub type ConsumerStreamSend = Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>;

#[async_trait]
pub trait Consumer: Send + Sync {
  async fn consume(
    &self,
    topic: &str,
    consumer_name: &str,
    offset_strategry: OffsetManagementStrategy,
    offset_start: Offset,
  ) -> Result<FluvioStreamer>;
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
  ) -> Result<FluvioStreamer> {
    Ok(FluvioStreamer::new(Box::pin(
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
    )))
  }
}

pub struct FluvioStreamer {
  stream: ConsumerStreamSend,
}

impl FluvioStreamer {
  pub fn new(stream: ConsumerStreamSend) -> Self {
    Self { stream }
  }

  pub fn next(&mut self) -> Next<'_, ConsumerStreamSend> {
    self.stream.next()
  }
  pub fn offset_commit(&mut self) -> std::result::Result<(), ErrorCode> {
    self.stream.offset_commit()
  }
  pub async fn offset_flush(&mut self) -> std::result::Result<(), ErrorCode> {
    self.stream.offset_flush().await
  }
}
