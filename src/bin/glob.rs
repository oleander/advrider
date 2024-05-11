use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use std::vec;

use advrider::ip;
use anyhow::{Context, Result};
use html2text::from_read;
use reqwest::header::HeaderMap;
use reqwest::header;
use spider::configuration::Configuration;
use spider::tokio;
use spider::website::Website;
use tokio::io::AsyncWriteExt;

mod tor {
  use lazy_static::lazy_static;
  use anyhow::{bail, Context, Result};
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use tokio::net::TcpStream;

  lazy_static! {}

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
        bail!("Unexpected response: {} vs {:?}", response, status);
      }
    }

    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let proxy = "socks5://127.0.0.1:9050";
  let url = "https://advrider.com/f/threads/thinwater-escapades.1502022/page-[1-40]";
  let dump_path = "data/dump.txt";
  let rotate_proxy_every = 10;
  let counter = AtomicUsize::new(0);
  let start = Instant::now();
  let mut config = Configuration::new();

  let config = config
    .with_depth(1)
    .with_headers(header()?)
    .with_proxies(vec![proxy.to_string()].into())
    .with_caching(true);

  let mut website = Website::new(url);
  let website = website.with_config(config.clone()).with_caching(true);
  let mut channel = website.subscribe(16).unwrap();

  log::info!("Reset {}", dump_path);
  tokio::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .open(dump_path)
    .await
    .context("Failed to open file")
    .unwrap()
    .set_len(0)
    .await
    .context("Failed to truncate file")
    .unwrap();

  tokio::spawn(async move {
    while let Ok(res) = channel.recv().await {
      let count = counter.fetch_add(1, Ordering::SeqCst);
      let html = res.get_html();
      let html_bytes = html.as_bytes();
      let markdown = from_read(html_bytes, usize::MAX);
      let markdown_bytes = markdown.as_bytes();
      let url = res.get_url();

      log::info!("[{}] Received {} bytes from {}", count, markdown_bytes.len(), url);

      tokio::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(dump_path)
        .await
        .context("Failed to open file")
        .unwrap()
        .write_all(markdown_bytes)
        .await
        .context("Failed to write to file")
        .unwrap();

      if count % rotate_proxy_every == 0 {
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

fn header() -> Result<Option<HeaderMap>> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  Ok(Some(headers))
}
