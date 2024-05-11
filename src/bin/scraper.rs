use std::time::Duration;

use spider::configuration::{Configuration, GPTConfigs, WaitForIdleNetwork};
use reqwest::header::{self, HeaderMap};
use spider::moka::future::Cache;
use anyhow::{Context, Result};
use spider::website::Website;
use html2text::from_read;
use log::{error, info};
use spider::tokio;

// const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 OPR/109.0.0.0";
const URL: &str = "https://www.advrider.com/f/threads/offroad-riding-in-germany.1349208/";
const OPENAI_MODEL: &str = "gpt-3.5-turbo";
const OPENAI_MAX_TOKEN: u16 = 512;
const RESPECT_ROBOT: bool = true;
const REDIRECT_LIMIT: usize = 2;

use warp::Filter;

lazy_static::lazy_static! {
  static ref PROXY_URL: String = env!("PROXY_URL").into();
}

async fn run_health_check_server() {
  let health_route = warp::path!("health").map(|| warp::reply::json(&"OK"));
  warp::serve(health_route).run(([127, 0, 0, 1], 4040)).await;
}

async fn perform_main_tasks() -> Result<()> {
  info!("Fetching URL, hold on...");
  let body = fetch(URL).await?;

  info!("Saving output to data/dump.txt");
  match tokio::fs::write("data/dump.txt", body).await {
    Ok(_) => info!("Scraped output saved to data/dump.txt"),
    Err(e) => error!("Failed to save scraped output: {}", e)
  }

  Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  env_logger::init();

  tokio::select! {
    _ = run_health_check_server() => {},
    _ = perform_main_tasks() => {},
  }

  Ok(())
}

async fn fetch(url: &str) -> Result<String> {
  log::info!("Starting ...");

  let cache = Cache::builder()
    .time_to_live(Duration::from_secs(30 * 60))
    .time_to_idle(Duration::from_secs(5 * 60))
    .max_capacity(100_000)
    .build();

  let network_config = Some(WaitForIdleNetwork::new(Some(Duration::from_micros(100))));
  let system_path = "prompts/summarize.md";
  let system_prompt = tokio::fs::read_to_string(system_path).await?;

  let openai_config =
    GPTConfigs::new_multi_cache(OPENAI_MODEL, vec![&system_prompt], OPENAI_MAX_TOKEN, Some(cache)).into();

  let proxies = vec![PROXY_URL.clone()];

  info!("Building request object to fetch website ...");
  let mut website = Website::new(url)
    .with_wait_for_idle_network(network_config)
    // .with_proxies(proxies.into())
    .with_openai(openai_config)
    .with_headers(header()?)
    .with_redirect_limit(2)
    .with_config(config())
    .with_caching(true)
    .with_tld(false)
    .with_limit(5)
    .build()
    .context("Could not build webpage")?;

  info!("Starting the crawler ...");
  website.crawl().await;

  info!("Starting processing the raw data ...");
  Ok(
    website
      .get_pages()
      .context("No web pages received")?
      .iter()
      .map(|page| from_read(page.get_html().as_bytes(), usize::MAX))
      .collect::<Vec<String>>()
      .join("\n")
  )
}

fn config() -> Configuration {
  Configuration::new()
    .with_respect_robots_txt(RESPECT_ROBOT)
    .with_redirect_limit(REDIRECT_LIMIT)
    // .with_user_agent(USER_AGENT.into())
    .build()
}

fn header() -> Result<Option<HeaderMap>> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  Ok(Some(headers))
}
