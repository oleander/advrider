
use advrider::ip;
use advrider::tor;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let ip = ip::get().await?;
  log::info!("Your IP address is: {}", ip);

  log::info!("Starting Tor control ...");
  let mut cmd = tor::Command::new("127.0.0.1:9051", "").await?;

  log::info!("Refreshing Tor identity ...");
  cmd.refresh().await?;

  let ip = ip::get().await?;
  log::info!("Your new IP address is: {}", ip);
  Ok(())
}
