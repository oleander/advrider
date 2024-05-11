use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use async_std::path::PathBuf;
use env_logger::Env;
use spider::compact_str::CompactString;
use spider::hashbrown::HashMap;
use structopt::StructOpt;
use spider::configuration::Configuration;
use futures::future::join_all;
use anyhow::{bail, Context, Result};
use spider::website::Website;
use tokio::io::AsyncWriteExt;
use spider::url::Url;
use html2text::from_read;
use spider::tokio;

mod tor {
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use anyhow::{bail, Context, Result};
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

async fn refresh_all_proxies(controllers: Vec<String>) {
  let futures = controllers
    .into_iter()
    .map(|controller| tor::refresh(controller.clone()));
  join_all(futures).await;
}

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
  #[structopt(long, help = "Set output directory", default_value = "/tmp")]
  output_dir: PathBuf,

  // limit number of pages
  #[structopt(long, help = "Limit number of pages", default_value = "50")]
  page_limit: u32
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
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

  // pub fn get_blacklist(&self) -> Box<regex::RegexSet> {
  //     match &self.blacklist_url {
  //         Some(blacklist) => match regex::RegexSet::new(&**blacklist) {
  //             Ok(s) => Box::new(s),
  //             _ => Default::default(),
  //         },
  //         _ => Default::default(),
  //     }
  // }
  //   https://advrider.com/f/{forums/racing.25,threads/*}{/,/page-[0-9]*, }

  // https://advrider.com/f/forums/racing.25/
  // https://advrider.com/f/forums/racing.25/page-4
  // https://advrider.com/f/forums/racing.25/page-5/
  // https://advrider.com/f/threads/motogp-francais-spoileurs.1733155/
  // https://advrider.com/f/threads/motogp-francais-spoileurs.1733155/page-281
  // https://advrider.com/f/threads/motogp-francais-spoileurs.1733155/page-2/

  // let set = regex::RegexSet::new(&[
  //   r"https://advrider\.com/f/forums/racing\.25",
  //   r"https://advrider\.com/f/forums/racing\.25/page-\d+",
  //   r"https://advrider\.com/f/threads/[^/]+",
  //   r"https://advrider\.com/f/threads/[^/]+/page-\d+"
  // ]).unwrap();

  // let one_pattern = "^(?!https:\/\/advrider\.com\/f\/(forums\/racing\.25|threads\/[^\/]+)\/(page-\d+\/?)?$).*
  // "

  let patterns = vec![
    "^/f/forums/racing\\.25/$".into(),
    "^/f/forums/racing\\.25/page-\\d+/?$".into(),
    "^/f/threads/[^/]+/$".into(),
    "^/f/threads/[^/]+/page-\\d+/?$".into(),
  ]
  .into();

  let budget = HashMap::from([("/f/forums/racing.25*", 5), ("/f/threads/*/page-*", 10), ("/f/threads/*", 2)]).into();

  let config = config
    .with_respect_robots_txt(true)
    .with_caching(opt.cache)
    .with_proxies(proxies)
    .with_budget(budget)
    .with_blacklist_url(patterns);

  // .with_delay(50)
  // .with_depth(2);

  let mut website = Website::new(&url);
  let website = website.with_config(config.clone());
  let mut channel = website.subscribe(16).unwrap();
  // let mut guard = website.subscribe_guard().unwrap();
  // let queue = website.queue(opt.proxies.len()).unwrap();

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

      // let parsed_url = Url::parse(url).unwrap();
      // let _size = queue.send(parsed_url.into()).unwrap();

      // guard.inc();

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
        return log::warn!("Reached the end of the thread: {} of {}", page, page);
      } else if count % rotate_proxy_every == 0 && count > 0 {
        log::warn!("[{}] Resetting Tor proxy connection", count);
        refresh_all_proxies(opt.controllers.clone()).await;
      }

      if count >= opt.page_limit as usize {
        return log::warn!("Reached page limit: {}", opt.page_limit);
      }
    }
  });

  log::info!("Scraping website, hold on...");
  website.crawl().await;

  log::info!("URL: {}", website.get_links().len());

  log::info!("Time passed: {:?}", start.elapsed());

  Ok(())
}
