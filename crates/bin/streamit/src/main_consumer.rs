use anyhow::{Context, Result};
use async_trait::async_trait;
use bilrost::BorrowedMessage;
use consumer::{Consumer, FluvioConsumer};
use fluvio::{consumer::Record as ConsumerRecord, Offset};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use streamitlib::{
  configure_tracing::init,
  message::{MessageKind, MessageWrapper},
  topic::MYIO_TOPIC,
};
use thiserror::Error;
use tokio::{
  signal::unix::{signal, SignalKind},
  sync::mpsc::{self, Sender},
  time::sleep,
};
use tracing::{debug, error, info, trace, warn};

mod consumer;
mod mock_consumer;

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

  let mut ingest_task = tokio::spawn(async move {
    loop {
      let pinger = pinger.clone();
      let consumer = consumer.clone();
      if let Err(e) = receiver(&tx, &*pinger, consumer).await {
        warn!("receiver error: {e:?}");
      }
      sleep(Duration::from_secs(2)).await;
    }
  });

  let mut recv_task = tokio::spawn(async move {
    loop {
      tokio::select! {
          Some(data) = rx.recv() => {
              if let Err(e) = handle_message(&data) {
                  match e.downcast_ref::<ConsumerError>() {
                      Some(ConsumerError::CloseRequested(reason)) => {
                          info!("Close consumers requested: {reason}");
                          break;
                      }
                      None => {
                          error!("Failed to handle message: {e}");
                      }
                  }
              };
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

async fn receiver(
  tx: &Sender<ConsumerRecord>,
  pinger: &(impl Pinger + ?Sized),
  consumer: Arc<impl Consumer + ?Sized>,
) -> anyhow::Result<()> {
  // TODO commit offset and set consumer name https://github.com/infinyon/fluvio/blob/master/rfc/offset-management.md

  let mut stream = consumer.clone().consume(MYIO_TOPIC, Offset::beginning()).await.unwrap();

  while let Some(msg) = stream.next().await {
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
    MessageKind::CloseConsumers(reason) => {
      debug!("Received CloseServer: {:?}", reason);
      return Err(ConsumerError::CloseRequested(reason).into());
    }
    MessageKind::None => {
      error!("Received None. Data: {:?}", data);
    }
  };
  Ok(())
}

#[derive(Error, Debug)]
pub enum ConsumerError {
  #[error("CloseServer request received with reason: {0}")]
  CloseRequested(String),
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
  use crate::mock_consumer::tests::MockConsumer;
  use bilrost::Message;
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
    .map(|m| m.encode_to_bytes())
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
