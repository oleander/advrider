
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

  log::info!("Authenticating with Tor network ...");
  cmd.authenticate().await?;

  log::info!("Checking liveness of Tor network ...");
  cmd.liveness().await?;

  log::info!("Requesting new identity ...");
  cmd.newnym().await?;

  log::info!("Quitting Tor network ...");
  cmd.quit().await?;

  let ip = ip::get().await?;
  log::info!("Your new IP address is: {}", ip);
  Ok(())
}
