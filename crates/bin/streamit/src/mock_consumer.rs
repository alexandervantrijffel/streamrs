#[cfg(test)]
pub mod tests {
  use crate::consumer::{Consumer, FluvioStreamer};
  use anyhow::Result;
  use async_trait::async_trait;
  use fluvio::consumer::{OffsetManagementStrategy, Record as ConsumerRecord};
  use fluvio::{consumer::ConsumerStream, Offset};
  use fluvio_protocol::link::ErrorCode;
  use fluvio_protocol::record::{Batch, Record, RecordData};
  use futures::{stream, Stream};
  use std::task::ready;

  // Mock Consumer that emits all records in record_values without a delay
  #[derive(Clone)]
  pub struct MockConsumer {
    record_values: Vec<Record>,
  }

  impl MockConsumer {
    pub fn new(record_values: Vec<impl Into<RecordData>>) -> Self {
      Self {
        record_values: record_values
          .into_iter()
          .map(|record_value| Record::new(record_value.into()))
          .collect(),
      }
    }
  }

  #[async_trait]
  impl Consumer for MockConsumer {
    async fn consume(
      &self,
      _topic: &str,
      _consumer_name: &str,
      _offset_strategry: OffsetManagementStrategy,
      _offset_start: Offset,
    ) -> Result<FluvioStreamer> {
      let mut batch = Batch::new();
      self
        .record_values
        .iter()
        .for_each(|record| batch.add_record(record.clone()));

      // we could also do this if we change &self to &mut self and pass the consumer around as
      // Arc<Mutex<dyn Consumer> so that we have a mutable self.record_values
      // batch.add_records(&mut self.record_values);

      let stream = SinglePartitionConsumerStream {
        // inner: stream::iter(vec![Ok(record)]),
        inner: stream::iter(batch.into_consumer_records_iter(0).map(Ok)),
      };

      Ok(FluvioStreamer::new(Box::pin(stream)))
    }
  }

  pub struct SinglePartitionConsumerStream<T> {
    inner: T,
  }

  impl<T: Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin> ConsumerStream for SinglePartitionConsumerStream<T> {
    fn offset_commit(&mut self) -> std::result::Result<(), ErrorCode> {
      Ok(())
    }

    fn offset_flush(&mut self) -> futures::future::BoxFuture<'_, std::result::Result<(), ErrorCode>> {
      Box::pin(async { Ok(()) })
    }
  }

  impl<T: Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin> Stream for SinglePartitionConsumerStream<T> {
    type Item = Result<ConsumerRecord, ErrorCode>;

    fn poll_next(
      self: std::pin::Pin<&mut Self>,
      cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
      let self_mut = self.get_mut();
      let pinned = std::pin::pin!(&mut self_mut.inner);
      match ready!(pinned.poll_next(cx)) {
        Some(Ok(last)) => std::task::Poll::Ready(Some(Ok(last))),
        other => std::task::Poll::Ready(other),
      }
    }
  }
}
