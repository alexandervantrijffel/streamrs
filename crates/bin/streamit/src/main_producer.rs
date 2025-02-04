use streamitlib::configure_tracing::configure_tracing;
use tracing::error;

#[tokio::main]
async fn main() {
  configure_tracing();
  _ = streamitlib::producer().await.inspect_err(|e| {
    error!("Unexpected error: {:?}", e);
  });
}
