use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use std::vec;

use advrider::ip;
use anyhow::{Context, Result};
use html2text::from_read;
use spider::configuration::Configuration;
use spider::tokio;
use spider::website::Website;
use tokio::io::AsyncWriteExt;

mod tor {
  use anyhow::{bail, Context, Result};
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use tokio::net::TcpStream;

  const CONTROL_URL: &str = "127.0.0.1:9051";

  async fn send(command: &str, stream: &mut TcpStream) -> Result<()> {
    stream.write_all(command.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    stream.flush().await.context("Failed to flush")
  }

  pub async fn refresh() -> Result<()> {
    let status = vec!["250 OK", "250 OK", "250 closing connection"];
    let mut stream = TcpStream::connect(CONTROL_URL).await?;

    send("AUTHENTICATE \"\"", &mut stream).await?;
    send("SIGNAL NEWNYM", &mut stream).await?;
    send("QUIT", &mut stream).await?;

    let mut reader = BufReader::new(&mut stream);
    let mut response = String::new();

    while reader.read_line(&mut response).await? > 0 {
      if !status.iter().any(|status| response.contains(status)) {
        bail!("Unexpected Tor response: {} vs {:?}", response, status);
      }
    }

    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let url = "https://advrider.com/f/threads/thinwater-escapades.1502022/page-[1-40]";
  let proxy = "socks5://127.0.0.1:9050";
  let rotate_proxy_every = 5;

  let counter = AtomicUsize::new(0);
  let start = Instant::now();
  let mut config = Configuration::new();

  let config = config
    .with_depth(1)
    .with_proxies(vec![proxy.to_string()].into())
    .with_caching(true);

  let mut website = Website::new(url);
  let website = website.with_config(config.clone()).with_caching(true);
  let mut channel = website.subscribe(rotate_proxy_every).unwrap();

  log::info!("Rotating Tor proxy");
  tor::refresh().await?;
  log::info!("Resetting IP address to {}", ip::get().await.unwrap());

  tokio::spawn(async move {
    while let Ok(res) = channel.recv().await {
      let count = counter.fetch_add(1, Ordering::SeqCst);
      let html = res.get_html();
      let html_bytes = html.as_bytes();
      let markdown = from_read(html_bytes, usize::MAX);
      let markdown_bytes = markdown.as_bytes();
      let url = res.get_url();
      let page = url.split("/").last().unwrap().split("-").last().unwrap();
      let output_path = format!("data/pages/{}.md", page);

      log::info!("[{}] Received {} bytes from page #{}", count, markdown_bytes.len(), page);

      tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_path.clone())
        .await
        .context("Failed to open file")
        .unwrap()
        .write_all(markdown_bytes)
        .await
        .context("Failed to write to file")
        .unwrap();

      log::info!("[{}] Wrote {} bytes to {}", count, markdown_bytes.len(), output_path);

      if count % rotate_proxy_every == 0 && count > 0 {
        log::warn!("[{}] Resetting Tor proxy connection", count);

        match tor::refresh().await {
          Ok(_) => log::info!("[{}] Successfully refreshed Tor", count),
          Err(e) => log::error!("[{}] Failed to refresh Tor: {}", count, e)
        }

        log::info!("[{}] Resetting IP address to {}", count, ip::get().await.unwrap());
      }
    }
  });

  website.scrape().await;
  let duration = start.elapsed();

  log::info!("Time passed: {:?}", duration);

  Ok(())
}
