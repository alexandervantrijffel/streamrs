#![allow(tail_expr_drop_order)]
#![allow(if_let_rescope)]
mod consumer;
mod mock_consumer;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bilrost::BorrowedMessage;
use consumer::{Consumer, FluvioConsumer};
use fluvio::{
  consumer::{OffsetManagementStrategy, Record as ConsumerRecord},
  Offset,
};
use std::{sync::Arc, time::Duration};
use streamitlib::{
  configure_tracing::init,
  message::{MessageKind, MessageWrapper},
  topic::MYIO_TOPIC,
};
use thiserror::Error;
use tokio::{
  signal::unix::{signal, SignalKind},
  sync::{
    mpsc::{self, Sender},
    oneshot,
  },
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

async fn main_consumer<TPinger: Pinger + 'static, TConsumer: Consumer + 'static>(
  pinger: Arc<TPinger>,
  consumer: Arc<TConsumer>,
) -> Result<()> {
  info!("Starting consumer");
  let (new_msg_tx, mut new_msg_rx) = mpsc::channel(100);

  let mut ingest_task = tokio::spawn(async move {
    loop {
      let pinger = pinger.clone();
      let consumer = consumer.clone();
      if let Err(e) = receiver(&new_msg_tx, pinger, consumer).await {
        warn!("receiver error: {e:?}");
      }
      sleep(Duration::from_secs(2)).await;
    }
  });

  let mut signal_terminate = signal(SignalKind::terminate())?;
  let mut signal_interrupt = signal(SignalKind::interrupt())?;

  let mut recv_task = tokio::spawn(async move {
    loop {
      tokio::select! {
          Some((record,msg_processed_tx)) = new_msg_rx.recv() => {
              if let Err(e) = handle_message(&record,msg_processed_tx) {
                    if let Some(ConsumerError::CloseRequested(reason)) = e.downcast_ref::<ConsumerError>() {
                        info!("Consumer close requested: {reason}");
                        break;
                    }
                    error!("Message handling failed with error: {e:?}");
              }
          }

          () = sleep(Duration::from_secs(10)) => trace!("No new messages after 10s"),

          _ = signal_terminate.recv() => {
              break;
          }

          _ = signal_interrupt.recv() => {
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

async fn receiver<TPinger: Pinger + 'static, TConsumer: Consumer + 'static>(
  tx: &Sender<(ConsumerRecord, oneshot::Sender<()>)>,
  pinger: Arc<TPinger>,
  consumer: Arc<TConsumer>,
) -> Result<()> {
  let mut stream = consumer
    .clone()
    .consume(
      MYIO_TOPIC,
      "main_consumer",
      OffsetManagementStrategy::Manual,
      // Start from the last committed offset for this consumer or the beginning of the topic
      // if no offset is committed.
      // docs: https://github.com/infinyon/fluvio/blob/master/rfc/offset-management.md
      Offset::beginning(),
    )
    .await
    .context("Failed to create consumer stream")?;

  while let Some(next) = stream.next().await {
    match next {
      Ok(msg) => {
        let (msg_processed_tx, msg_processed_rx) = oneshot::channel::<()>();
        if let Err(e) = tx.send((msg, msg_processed_tx)).await {
          error!("receiver: Failed to send to the msg_processed channel: {e}");
          continue;
        }

        let msg_processed = msg_processed_rx.await;

        match msg_processed {
          Ok(()) => {
            trace!("receiver: committing offset");
            if let Err(e) = stream.offset_commit().await {
              error!("Failed to commit offset: {e}");
              continue;
            }
            // todo perf: flushing offsets can be improved by batching
            let flushed = stream.offset_flush().await;
            if let Err(e) = flushed {
              error!("Failed to flush offset: {e}");
            }
          }
          Err(e) => error!("receiver: Failed to receive message processed signal: {e}"),
        }
      }
      Err(e) => error!("{e:?}"),
    }
  }

  pinger.ping("ping").await;

  Ok(())
}

fn handle_message(record: &ConsumerRecord, msg_processed_tx: oneshot::Sender<()>) -> Result<()> {
  let data = record.value();
  let wrapper = MessageWrapper::decode_borrowed(data).context("Failed to decode message")?;

  let result = match wrapper.kind {
    MessageKind::Birth(birth) => {
      debug!("Received Birth: {birth:?}");
      Ok(())
    }
    MessageKind::Marriage(marriage) => {
      debug!("Received Marriage: {marriage:?}");
      Ok(())
    }
    MessageKind::CloseConsumers(reason) => {
      debug!("Received CloseServer: {reason:?}");
      Err(ConsumerError::CloseRequested(reason).into())
    }
    MessageKind::None => {
      error!("Received None. Data: {data:?}");
      Ok(())
    }
  };
  if msg_processed_tx.send(()).is_err() {
    error!("Failed to send message processed signal");
  }
  result
}

#[derive(Error, Debug)]
pub enum ConsumerError {
  #[error("CloseServer request received with reason: {0}")]
  CloseRequested(String),
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::mock_consumer::tests::MockConsumer;
  use std::sync::atomic::{AtomicBool, Ordering};
  use streamitlib::message::Birth;
  use tracing_test::traced_test;

  // Mock Pinger that tracks calls
  struct MockPinger {
    called_with_ping: Arc<AtomicBool>,
  }

  #[async_trait]
  impl Pinger for MockPinger {
    async fn ping(&self, arg: &str) -> String {
      if arg == "ping" {
        self.called_with_ping.store(true, Ordering::SeqCst);
      }
      "pong".to_string()
    }
  }

  #[traced_test]
  #[tokio::test]
  async fn test_consumer_calls_ping() {
    let called = Arc::new(AtomicBool::new(false));
    let pinger = Arc::new(MockPinger {
      called_with_ping: called.clone(),
    });

    let records = [
      MessageWrapper::from(Birth::new("AliceMOCK".to_owned())),
      MessageWrapper::new_close_server(),
    ]
    .iter()
    .map(bilrost::Message::encode_to_bytes)
    .collect();
    let consumer_mock = Arc::new(MockConsumer::new(records));

    let _ = main_consumer(pinger, consumer_mock).await;

    assert!(logs_contain("Received Birth"));

    assert!(logs_contain("AliceMOCK"));

    assert!(logs_contain("Received CloseServer"));

    assert!(
      called.load(Ordering::SeqCst),
      "Expected Pinger::ping to be called with 'ping'"
    );
  }
}
