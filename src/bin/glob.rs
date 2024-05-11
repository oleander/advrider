use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use std::vec;

use anyhow::{Context, Result};
use tokio::io::BufReader;
use html2text::from_read;
use reqwest::header::HeaderMap;
use reqwest::header;
use spider::configuration::Configuration;
use spider::tokio;
use spider::website::Website;

mod tor {
  use anyhow::bail;
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use anyhow::{Context, Result};
  use tokio::net::TcpStream;

  async fn send(command: &str, stream: &mut TcpStream) -> Result<()> {
    stream.write(command.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    stream.flush().await.context("Failed to flush")
  }

  pub async fn refresh() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:9051").await?;
    send("AUTHENTICATE \"\"", &mut stream).await?;
    send("SIGNAL NEWNYM", &mut stream).await?;
    send("QUIT", &mut stream).await?;

    let expected = vec!["250 OK", "250 OK", "250 closing connection"];

    let mut reader = BufReader::new(&mut stream);
    let mut response = String::new();

    while reader.read_line(&mut response).await? > 0 {
      if !expected.iter().any(|e| response.contains(e)) {
        bail!("Unexpected response: {}", response);
      }
    }

    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let mut b = Configuration::new();
  let h = header()?;
  let config = b
    .with_depth(1)
    .with_proxies(vec!["socks5://127.0.0.1:9050".to_string()].into())
    .with_caching(true);

  let mut website: Website = Website::new("https://advrider.com/f/threads/thinwater-escapades.1502022/page-[1-40]");

  website
    .configuration
    .blacklist_url
    .insert(Default::default())
    .push("https://github.com/oleander".into());

  let website = website.with_config(config.clone()).with_caching(true);
  let mut rx2 = website.subscribe(16).unwrap();
  let counter = AtomicUsize::new(0);

  tokio::spawn(async move {
    while let Ok(res) = rx2.recv().await {
      let count = counter.load(Ordering::SeqCst);
      counter.fetch_add(1, Ordering::SeqCst);

      if count % 10 != 0 || count == 0 {
        log::info!("Count: {}", count);
        continue;
      }

      tor::refresh().await;
    }
  });

  let start = Instant::now();
  website.scrape().await;
  let duration = start.elapsed();

  let links = website.get_links();
  let Some(pages) = website.get_pages() else {
    println!("No pages found");
    return Ok(());
  };

  let body = website
    .get_pages()
    .context("No web pages received")?
    .iter()
    .map(|page| from_read(page.get_html().as_bytes(), usize::MAX))
    .collect::<Vec<String>>()
    .join("\n");

  log::info!("Writing to file data/dump.txt");
  std::fs::write("data/dump.txt", body).context("Failed to write to file")?;
  for link in links {
    println!("- {:?}", link.as_ref());
  }

  println!("Time elapsed in website.crawl() is: {:?} for total pages: {:?}", duration, links.len());

  Ok(())
}

fn header() -> Result<Option<HeaderMap>> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  Ok(Some(headers))
}
