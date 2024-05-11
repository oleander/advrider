use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use async_std::path::PathBuf;
use env_logger::Env;
use spider::configuration::Configuration;
use anyhow::{bail, Context, Result};
use spider::website::Website;
use tokio::io::AsyncWriteExt;
use spider::url::Url;
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

  pub async fn refresh(control_url: String) -> Result<()> {
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

use structopt::StructOpt;

/// Command-line options defined using StructOpt
#[derive(StructOpt, Debug)]
#[structopt(name = "scraper")]
struct Opt {
  #[structopt(short, long, help = "https://advrider.com/f/threads/the-toolkit-thread.262998/page-[1-2]")]
  url: String,

  #[structopt(long = "proxies", help = "socks5://127.0.0.1:9050")]
  proxies: Vec<String>,

  #[structopt(long = "controllers", help = "127.0.0.0.1:9051")]
  controllers: Vec<String>,

  /// Rotate proxy every N requests
  #[structopt(long, default_value = "30", help = "Rotate proxy every N requests")]
  rotate_proxy_every: usize,

  /// Enable verbose output
  #[structopt(short, long, help = "Enable verbose output")]
  verbose: bool,

  // cache
  #[structopt(long, help = "Enable caching")]
  cache: bool,

  // print only urls
  #[structopt(long, help = "Print only URLs, do not save")]
  only_print_urls: bool,

  // set output dir
  #[structopt(long, help = "Set output directory", default_value = ".")]
  output_dir: PathBuf,

  // limit number of pages
  #[structopt(long, help = "Limit number of pages", default_value = "50")]
  page_limit: u32
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
  let opt = Opt::from_args();

  let level = if opt.verbose {
    "debug"
  } else {
    "info"
  };

  let env = Env::default()
    .filter_or("RUST_LOG", level)
    .write_style_or("RUST_LOG_STYLE", "always");

  env_logger::init_from_env(env);

  let url = opt.url.clone();

  match (opt.proxies.len(), opt.controllers.len()) {
    (0, _) => {
      bail!("No proxies provided, using default");
    }
    (_, 0) => {
      bail!("No controllers provided, using default");
    }
    (a, b) if a != b => {
      bail!("Number of proxies and controllers must match ({} vs {})", a, b);
    }
    _ => ()
  }

  if !opt.output_dir.exists().await {
    bail!("Output directory does not exist: {:?}", opt.output_dir);
  } else if !opt.output_dir.is_dir().await {
    bail!("Output directory is not a directory: {:?}", opt.output_dir);
  }

  let proxies = opt.proxies.clone().into();
  let rotate_proxy_every = opt.rotate_proxy_every;

  let mut config = Configuration::new();
  let counter = AtomicUsize::new(0);
  let start = Instant::now();

  let config = config
    .with_depth(1)
    .with_proxies(proxies)
    .with_caching(opt.cache)
    .with_limit(opt.page_limit)
    .with_respect_robots_txt(true)
    .with_delay(100);

  let mut website = Website::new(&url);
  let website = website.with_config(config.clone()).with_caching(opt.cache);
  let mut channel = website.subscribe(16).unwrap();
  let mut guard = website.subscribe_guard().unwrap();
  let queue = website.queue(opt.proxies.len()).unwrap();

  if opt.only_print_urls {
    log::warn!("Will only print URLs, not save to disk");
  }

  refresh_all_proxies(opt.controllers.clone()).await;

  tokio::spawn(async move {
    while let Ok(res) = channel.recv().await {
      let count = counter.fetch_add(1, Ordering::SeqCst);
      let html = res.get_html();
      let html_bytes = html.as_bytes();
      let markdown = from_read(html_bytes, usize::MAX);
      let markdown_bytes = markdown.as_bytes();
      let url = res.get_url();
      let page = url.split("/").last().unwrap().split("-").last().unwrap();


      let parsed_url = Url::parse(url).unwrap();
      let _size = queue.send(parsed_url.into()).unwrap();

      guard.inc();

      log::info!("[{}] URL: {}", count, url);

      if opt.only_print_urls {
        continue;
      } else if markdown_bytes.len() == 0 {
        log::warn!("[{}] Skipping empty page #{}", count, page);
        log::warn!("[{}] Will rotate Tor proxy", count);
        refresh_all_proxies(opt.controllers.clone()).await;
        continue;
      }

      log::info!("[{}] Received {} bytes from page #{}", count, markdown_bytes.len(), page);

      let output_file = format!("page-{}.md", page);
      let output_path = opt.output_dir.join(output_file);

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

      log::info!("[{}] Wrote {} bytes to {}", count, markdown_bytes.len(), output_path.display());

      let end_page = format!("Page {} of {} ", page, page);
      if markdown.contains(&end_page) {
        log::warn!("Reached the end of the thread: {} of {}", page, page);
        std::process::exit(0);
      } else if count % rotate_proxy_every == 0 && count > 0 {
        log::warn!("[{}] Resetting Tor proxy connection", count);

        refresh_all_proxies(opt.controllers.clone()).await;
      }
    }
  });

  log::info!("Scraping website, hold on...");
  website.scrape().await;

  log::info!("URL: {}", website.get_links().len());

  log::info!("Time passed: {:?}", start.elapsed());

  Ok(())
}

use futures::future::join_all;

async fn refresh_all_proxies(controllers: Vec<String>) {
  let futures = controllers
    .into_iter()
    .map(|controller| tor::refresh(controller.clone()));
  join_all(futures).await;
}
