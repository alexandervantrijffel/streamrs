use anyhow::Context;
use bilrost::BorrowedMessage;
use fluvio::{
  consumer::{ConsumerConfigExtBuilder, Record},
  Fluvio, Offset,
};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
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
  init();
  info!("Starting Consumer");
  _ = consumer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}

pub async fn consumer() -> anyhow::Result<()> {
  let (tx, mut rx) = mpsc::channel(100);

  let mut receiver_task = tokio::spawn(async move {
    let tx = Arc::new(tx);
    loop {
      if let Err(e) = receiver(tx.clone()).await {
        warn!("receiver error: {e:?}");
        sleep(Duration::from_secs(2)).await;
      }
    }
  });

  let mut recv_task = tokio::spawn(async move {
    loop {
      tokio::select! {
          Some(data) = rx.recv() => {
              _ = handle_message(data).await.inspect_err(|e| {
                debug!("Failed to handle message: {:?}", e);
              });
          }
          _ = sleep(Duration::from_secs(10)) => trace!("No new messages after 10s"),
          _ = handle_signals() => {
              break;
          }
      };
    }
  });

  // If any one of the tasks run to completion, we abort the other.
  tokio::select! {
      _ = (&mut receiver_task) => receiver_task.abort(),
      _ = (&mut recv_task) => recv_task.abort(),
  };

  Ok(())
}

async fn receiver(tx: Arc<Sender<Record>>) -> anyhow::Result<()> {
  loop {
    let fluvio = Fluvio::connect().await?;
    let mut stream = fluvio
      .consumer_with_config(
        ConsumerConfigExtBuilder::default()
          .topic(MYIO_TOPIC)
          .partition(0)
          // TODO store last processed offset in a topic
          .offset_start(Offset::beginning())
          .build()?,
      )
      .await?;
    while let Some(msg) = stream.next().await {
      match msg {
        Ok(msg) => tx.send(msg).await.context("Failed to send to the mpsc channel")?,
        Err(e) => error!("{e:?}"),
      }
    }
  }
}

async fn handle_message(record: Record) -> anyhow::Result<()> {
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
  let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
  let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();

  tokio::select! {
      _ = signal_terminate.recv() => debug!("Received SIGTERM."),
      _ = signal_interrupt.recv() => debug!("Received SIGINT."),
  };
  Ok(())
}
