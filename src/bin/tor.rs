
use advrider::ip;
use advrider::tor;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  log::info!("Starting Tor control ...");
  let mut cmd1 = tor::Command::new("127.0.0.1:9051", "").await?;
  let mut cmd = cmd1.authenticate().await?;

  cmd.wait_for_ready().await?;

  let ip = ip::get().await?;
  log::info!("Your IP address is: {}", ip);


  log::info!("Refreshing Tor identity ...");
  cmd.refresh().await?;

  let ip = ip::get().await?;
  log::info!("Your new IP address is: {}", ip);

  cmd.quit().await?;
  Ok(())
}
