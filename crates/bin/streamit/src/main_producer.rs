use streamitlib::configure_tracing::configure_tracing;
use tracing::{error, info};

#[tokio::main]
async fn main() {
  configure_tracing();
  info!("Starting Producer");
  _ = streamitlib::producer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}
