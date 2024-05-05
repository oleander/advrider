use spider::configuration::{Configuration, GPTConfigs};
use reqwest::header::{self, HeaderMap};
use anyhow::{Context, Result};
use spider::website::Website;
use spider::tokio;

const URL: &str = "https://advrider.com/f/threads/husqvarna-701-super-moto-and-enduro.1086621/page-[1-2314]";
const OPENAI_MODEL: &str = "gpt-3.5-turbo";
const CRAWL_LIST: [&str; CAPACITY] = [URL];
const AGENT_NAME: &str = "Lisa Eriksson";
const ENABLE_WEB_CACHE: bool = false;
const OPENAI_MAX_TOKEN: u16 = 512;
const RESPECT_ROBOT: bool = true;
const REDIRECT_LIMIT: usize = 2;
const CAPACITY: usize = 1;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let mut handles = Vec::with_capacity(CAPACITY);

  for url in CRAWL_LIST {
    let handle = tokio::spawn(async move { fetch(url).await });

    handles.push(handle);
  }

  let mut acc = vec![];
  for handle in handles {
    let response = handle.await?.context("Handler could not be completed")?;
    let r1 = response.clone();
    let r2 = response.clone();

    acc.push(r1);
    log::info!("Removed {} bytes over handler", r2.len());
  }
  let total = acc.join("\n");
  log::info!("Received in total {} bytes", total.len());

  Ok(())
}

async fn fetch(url: &str) -> Result<String> {
  log::info!("Starting ...");

  let mut website = Website::new(url)
    .with_openai(openai().await.ok())
    .with_caching(true)
    .with_subdomains(false)
    .with_redirect_limit(2)
    .with_headers(header()?)
    .with_config(config())
    .build()
    .context("Could not build webpage")?;

  website.scrape().await;

  let body = website.get_pages().context("No web page received")?;

  let mut content = String::new();
  for page in (*body).iter() {
    let html = page.get_html();
    content.push_str(&html);
    log::info!("Adding {} bytes", html.len());
  }

  let len = content.len();
  log::info!("Done downloading {} bytes", len);

  Ok(content)
}

async fn openai() -> Result<GPTConfigs> {
  let system_path = "/Users/linus/.config/fabric/patterns/summarize/system.md";
  let system_prompt = tokio::fs::read_to_string(system_path).await?;
  let config = GPTConfigs::new(OPENAI_MODEL, &system_prompt, OPENAI_MAX_TOKEN);
  Ok(config)
}

fn config() -> Configuration {
  Configuration::new()
    .with_respect_robots_txt(RESPECT_ROBOT)
    .with_redirect_limit(REDIRECT_LIMIT)
    .with_user_agent(Some(AGENT_NAME))
    .build()
}

fn header() -> Result<Option<HeaderMap>> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  headers.insert(header::UPGRADE_INSECURE_REQUESTS, "1".parse().unwrap());
  headers.insert(header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 OPR/109.0.0.0".parse()?);
  Ok(Some(headers))
}
