use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use std::vec;

use spider::configuration::Configuration;
use anyhow::{Context, Result};
use spider::website::Website;
use tokio::io::AsyncWriteExt;
use html2text::from_read;
use spider::tokio;

mod tor {
  use anyhow::{bail, Context, Result};
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use tokio::net::TcpStream;

  async fn send(command: &str, stream: &mut TcpStream) -> Result<()> {
    stream.write_all(command.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    stream.flush().await.context("Failed to flush")
  }

  pub async fn refresh(control_url: &str) -> Result<()> {
    let status = vec!["250 OK", "250 OK", "250 closing connection"];
    let mut stream = TcpStream::connect(control_url)
      .await
      .context("Failed to connect to Tor control port")?;

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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  env_logger::init();

  let url = "https://advrider.com/f/threads/the-toolkit-thread.262998/page-[1-480]";

  let proxy0 = "socks5://127.0.0.1:9050";
  let proxy1 = "socks5://127.0.0.1:8050";
  let proxy2 = "socks5://127.0.0.1:7050";

  let proxies = vec![proxy0.to_string(), proxy1.to_string(), proxy2.to_string()].into();
  let rotate_proxy_every = 30;

  let mut config = Configuration::new();
  let counter = AtomicUsize::new(0);
  let start = Instant::now();

  let config = config
    .with_depth(1)
    .with_proxies(proxies)
    .with_caching(true);

  let mut website = Website::new(url);
  let website = website.with_config(config.clone()).with_caching(true);
  let mut channel = website.subscribe(rotate_proxy_every).unwrap();

  refresh_all_proxies().await;

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

      log::info!("[{}] URL: {}", count, url);

      if markdown_bytes.len() == 0 {
        log::warn!("[{}] Skipping empty page #{}", count, page);
        log::warn!("[{}] Will rotate Tor proxy", count);
        refresh_all_proxies().await;
        continue;
      }

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

      let end_page = format!("Page {} of {} ", page, page);
      if markdown.contains(&end_page) {
        log::warn!("Reached the end of the thread: {} of {}", page, page);
        std::process::exit(0);
      } else if count % rotate_proxy_every == 0 && count > 0 {
        log::warn!("[{}] Resetting Tor proxy connection", count);

        refresh_all_proxies().await;
      }
    }
  });

  log::info!("Scraping website, hold on...");
  website.scrape().await;

  log::info!("Time passed: {:?}", start.elapsed());

  Ok(())
}

async fn refresh_all_proxies() {
  log::info!("Rotating ALL Tor proxies");
  tokio::select! {
    _ = tor::refresh("127.0.0.1:9051") => (),
    _ = tor::refresh("127.0.0.1:8051") => (),
    _ = tor::refresh("127.0.0.1:7051") => ()
  }
  log::info!("Successfully rotated ALL Tor proxies");
}
