use crate::message::{MessageKind, MessageWrapper};
use anyhow::Context;
use bilrost::BorrowedMessage;
use fluvio::{
  Fluvio, Offset,
  consumer::{ConsumerConfigExtBuilder, Record},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::{
  signal::unix::{SignalKind, signal},
  sync::mpsc::{self, Sender},
};
use tracing::{debug, error, info, warn};

pub async fn consumer() -> anyhow::Result<()> {
  info!("Started Consumer");

  let (tx, mut rx) = mpsc::channel(100);

  let mut connector_task = tokio::spawn(async move {
    let tx = Arc::new(tx);
    loop {
      _ = receiver(tx.clone())
        .await
        .inspect_err(|e| warn!("receiver error: {e:?}"));
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
          _ = handle_signals() => {
              break;
          }
      };
    }
  });

  // If any one of the tasks run to completion, we abort the other.
  tokio::select! {
      _ = (&mut connector_task) => connector_task.abort(),
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
          .topic("myio")
          .partition(0)
          // TODO store offset in a topic
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
    MessageKind::Marriage(name) => {
      debug!("Received Marriage: {:?}", name);
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
