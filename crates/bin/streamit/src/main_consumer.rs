use streamitlib::{configure_tracing::configure_tracing, consumer::consumer};
use tracing::{error, info};

#[tokio::main]
async fn main() {
  configure_tracing();
  info!("Starting Consumer");
  _ = consumer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}
