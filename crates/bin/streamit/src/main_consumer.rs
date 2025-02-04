use streamitlib::{configure_tracing::configure_tracing, consumer::consumer};
use tracing::error;

#[tokio::main]
async fn main() {
  configure_tracing();
  _ = consumer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}
