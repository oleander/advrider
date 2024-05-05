use std::time::Instant;
use std::io::Error;

use reqwest::header::{self, HeaderMap, HeaderValue};
use serde_json::json;
use spider::configuration::{Configuration, GPTConfigs};
use spider::website::Website;
use spider::tokio;
use anyhow::{Context, Result};
use tokio::io::stdout;

const CAPACITY: usize = 1;
const CRAWL_LIST: [&str; CAPACITY] = ["https://advrider.com/f/threads/husqvarna-701-super-moto-and-enduro.1086621"];

fn header() -> Result<HeaderMap> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  headers.insert(header::UPGRADE_INSECURE_REQUESTS, "1".parse().unwrap());
  headers.insert(header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 OPR/109.0.0.0".parse()?);
  Ok(headers)
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let config = Configuration::new()
    .with_user_agent(Some("SpiderBot"))
    .with_blacklist_url(Some(Vec::from(["https://spider.cloud/resume".into()])))
    .with_subdomains(false)
    .with_tld(false)
    .with_redirect_limit(3)
    .with_respect_robots_txt(true)
    .with_external_domains(Some(Vec::from(["http://loto.rsseau.fr/"].map(|d| d.to_string())).into_iter()))
    .build();

  let mut handles = Vec::with_capacity(CAPACITY);

  let system = std::fs::read_to_string("/Users/linus/.config/fabric/patterns/summarize/system.md").unwrap();
  let openai = Some(GPTConfigs::new("gpt-4-turbo-preview", &system, 512));
  for url in CRAWL_LIST {
    let mut website = Website::new(url)
      .with_openai(openai.clone())
      .with_config(config.to_owned())
      .with_headers(Some(header()?.clone()))
      .with_caching(true)
      .build()
      .unwrap();

    website.crawl().await;

    let handle = tokio::spawn(async move {
      log::info!("Starting ...");

      website.crawl().await;

      let separator = "-".repeat(url.len());

      let Some(body) = website.get_pages() else {
        log::error!("Could not get page");
        return;
      };

      for page in (*body).iter() {
        let html = page.get_html();
        log::info!("{}{}{}", separator, html.len(), separator);
       }

      log::info!("Done downloading");
    });

    handles.push(handle);
  }

  for handle in handles {
    let _ = handle.await;
  }

  Ok(())
}
