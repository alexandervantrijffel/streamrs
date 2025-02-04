use anyhow::Context;
use bilrost::BorrowedMessage;
use fluvio::{
  Fluvio, Offset,
  consumer::{ConsumerConfigExtBuilder, Record},
};
use futures::StreamExt;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{debug, info};

use crate::message::{MessageKind, MessageWrapper};

pub async fn consumer() -> anyhow::Result<()> {
  info!("Started Consumer");

  // todo add reconnect loop
  // let (tx, mut rx) = mpsc::channel(100);

  let fluvio = Fluvio::connect().await?;
  let mut stream = fluvio
    .consumer_with_config(
      ConsumerConfigExtBuilder::default()
        .topic("myio")
        .partition(0)
        .offset_start(Offset::beginning())
        .build()?,
    )
    .await?;

  loop {
    tokio::select! {
        _ = handle_signals() => {
            break;
        }
        Some(data) = stream.next() => {
            let record = data.context("Failed to get record")?;
            _ = handle_message(record).await.inspect_err(|e| {
              debug!("Failed to handle message: {:?}", e);
            });
        }
    };
  }
  Ok(())
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
      debug!("Received None. Data: {:?}", data);
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
