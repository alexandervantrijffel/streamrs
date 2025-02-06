use anyhow::{Context, Result};
use async_trait::async_trait;
use bilrost::BorrowedMessage;
use fluvio::{
  consumer::{ConsumerConfigExtBuilder, ConsumerStream, Record as ConsumerRecord},
  Fluvio, Offset,
};
use fluvio_protocol::link::ErrorCode;
use futures::StreamExt;
use std::{pin::Pin, sync::Arc, time::Duration};
use streamitlib::{
  configure_tracing::init,
  message::{MessageKind, MessageWrapper},
  topic::MYIO_TOPIC,
};
use tokio::{
  signal::unix::{signal, SignalKind},
  sync::mpsc::{self, Sender},
  time::sleep,
};
use tracing::{debug, error, info, trace, warn};

#[tokio::main]
async fn main() {
  _ = init();
  _ = main_consumer(Arc::new(RealPinger), Arc::new(FluvioConsumer {}))
    .await
    .inspect_err(|e| {
      error!("Unexpected error: {:?}", e);
    });
}

async fn main_consumer(pinger: Arc<dyn Pinger>, consumer: Arc<dyn Consumer>) -> Result<()> {
  info!("Starting consumer");
  let (tx, mut rx) = mpsc::channel(100);

  let pinger = pinger.clone();
  let consumer = consumer.clone();

  let mut ingest_task = tokio::spawn(async move {
    // loop {
    if let Err(e) = receiver(&tx, &*pinger, consumer).await {
      warn!("receiver error: {e:?}");
      sleep(Duration::from_secs(2)).await;
    }
    // }
  });

  let mut recv_task = tokio::spawn(async move {
    loop {
      tokio::select! {
          Some(data) = rx.recv() => {
              _ = handle_message(&data).inspect_err(|e| {
                debug!("Failed to handle message: {:?}", e);
              });
          }

          () = sleep(Duration::from_secs(10)) => trace!("No new messages after 10s"),

          _ = handle_signals() => {
              break;
          }
      };
    }
  });

  // If any one of the tasks run to completion, we abort the other.
  tokio::select! {
      _ = (&mut ingest_task) => ingest_task.abort(),
      _ = (&mut recv_task) => recv_task.abort(),
  };
  sleep(Duration::from_secs(1)).await;

  Ok(())
}

#[async_trait]
trait Pinger: Send + Sync {
  async fn ping(&self, arg: &str) -> String;
}

#[derive(Copy, Clone)]
struct RealPinger;

#[async_trait]
impl Pinger for RealPinger {
  async fn ping(&self, _arg1: &str) -> String {
    String::from("pong")
  }
}

#[async_trait]
trait Consumer: Send + Sync {
  async fn consume(
    &self,
    topic: &str,
    offset: Offset,
  ) -> std::result::Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error>;
}

struct FluvioConsumer {}

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

async fn receiver(
  tx: &Sender<ConsumerRecord>,
  pinger: &(impl Pinger + ?Sized),
  consumer: Arc<impl Consumer + ?Sized>,
) -> anyhow::Result<()> {
  // TODO commit offset and set consumer name https://github.com/infinyon/fluvio/blob/master/rfc/offset-management.md
  trace!("Starting to consume");

  let mut stream = consumer.clone().consume(MYIO_TOPIC, Offset::beginning()).await.unwrap();

  trace!("Waiting for messages");

  while let Some(msg) = stream.next().await {
    trace!("Received message");
    match msg {
      Ok(msg) => tx.send(msg).await.context("Failed to send to the mpsc channel")?,
      Err(e) => error!("{e:?}"),
    }
  }
  pinger.ping("ping").await;
  Ok(())
}

fn handle_message(record: &ConsumerRecord) -> anyhow::Result<()> {
  let data = record.value();
  let wrapper = MessageWrapper::decode_borrowed(data).context("Failed to decode message")?;

  match wrapper.kind {
    MessageKind::Birth(birth) => {
      debug!("Received Birth: {:?}", birth);
    }
    MessageKind::Marriage(marriage) => {
      debug!("Received Marriage: {:?}", marriage);
    }
    MessageKind::None => {
      error!("Received None. Data: {:?}", data);
    }
  };
  Ok(())
}

async fn handle_signals() -> anyhow::Result<()> {
  let mut signal_terminate = signal(SignalKind::terminate())?;
  let mut signal_interrupt = signal(SignalKind::interrupt())?;

  tokio::select! {
      _ = signal_terminate.recv() => debug!("Received SIGTERM."),
      _ = signal_interrupt.recv() => debug!("Received SIGINT."),
  };

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use bilrost::Message;
  use fluvio_protocol::record::{Batch, Record, RecordData};
  use futures::{stream, Stream};
  use std::{
    sync::atomic::{AtomicBool, Ordering},
    task::ready,
  };
  use streamitlib::message::Birth;
  use tracing_test::traced_test;

  // Mock Pinger that tracks calls
  struct MockPinger {
    called: Arc<AtomicBool>,
  }

  #[async_trait]
  impl Pinger for MockPinger {
    async fn ping(&self, arg: &str) -> String {
      if arg == "ping" {
        self.called.store(true, Ordering::SeqCst);
      }
      "pong".to_string()
    }
  }

  // Mock Consumer that emits a single message
  #[derive(Clone)]
  struct MockConsumer {
    record_values: Vec<Record>,
  }

  impl MockConsumer {
    fn new(record_values: Vec<impl Into<RecordData>>) -> Self {
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
      _offset: Offset,
    ) -> Result<Pin<Box<dyn ConsumerStream<Item = Result<ConsumerRecord, ErrorCode>> + Send>>, anyhow::Error> {
      info!("MockConsumer::consume");

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
      Ok(Box::pin(stream))
    }
  }

  pub struct SinglePartitionConsumerStream<T> {
    inner: T,
  }

  impl<T: Stream<Item = Result<ConsumerRecord, ErrorCode>> + Unpin> ConsumerStream for SinglePartitionConsumerStream<T> {
    fn offset_commit(&mut self) -> std::result::Result<(), ErrorCode> {
      todo!()
    }

    fn offset_flush(&mut self) -> futures::future::BoxFuture<'_, std::result::Result<(), ErrorCode>> {
      todo!()
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
        Some(Ok(last)) => {
          trace!("LAST");
          std::task::Poll::Ready(Some(Ok(last)))
        }
        other => {
          trace!("OTHER");
          std::task::Poll::Ready(other)
        }
      }
    }
  }

  #[traced_test]
  #[tokio::test]
  async fn test_consumer_calls_ping() {
    let called = Arc::new(AtomicBool::new(false));
    let pinger = Arc::new(MockPinger { called: called.clone() });

    let consumer_mock = Arc::new(MockConsumer::new(vec![MessageWrapper::from(Birth::new(
      "AliceMOCK".to_owned(),
    ))
    .encode_to_bytes()]));

    let _ = main_consumer(pinger, consumer_mock).await;

    assert!(logs_contain("Received Birth"));

    assert!(
      called.load(Ordering::SeqCst),
      "Expected Pinger::ping to be called with 'ping'"
    );
  }
}
