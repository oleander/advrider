use async_openai::config::OpenAIConfig;
use spider::configuration::{Configuration, GPTConfigs};
use reqwest::header::{self, HeaderMap};
use spider::website::Website;
use anyhow::Result;
use spider::tokio;

const CAPACITY: usize = 1;
const OPENAI_MAX_TOKEN: u16 = 512;
const CRAWL_LIST: [&str; CAPACITY] = ["https://advrider.com/f/threads/husqvarna-701-super-moto-and-enduro.1086621"];

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let mut handles = Vec::with_capacity(CAPACITY);

  for url in CRAWL_LIST {
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

fn header() -> Result<Optional<HeaderMap>> {
  let mut headers = HeaderMap::new();
  headers.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert(header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert(header::COOKIE, "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  headers.insert(header::UPGRADE_INSECURE_REQUESTS, "1".parse().unwrap());
  headers.insert(header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 OPR/109.0.0.0".parse()?);
  Ok(Some(headers))
}

async fn openai() -> Result<GPTConfigs> {
  let system_path = "/Users/linus/.config/fabric/patterns/summarize/system.md";
  let system_prompt = tokio::fs::read_to_string(system_path).await?;
  let config = GPTConfigs::new("gpt-4-turbo-preview", &system_prompt, OPENAI_MAX_TOKEN);
  Ok(config)
}

fn config() -> Configuration {
  Configuration::new()
    .with_user_agent(Some("SpiderBot"))
    .with_blacklist_url(Some(Vec::from(["https://spider.cloud/resume".into()])))
    .with_subdomains(false)
    .with_tld(false)
    .with_redirect_limit(3)
    .with_respect_robots_txt(true)
    .with_external_domains(Some(Vec::from(["http://loto.rsseau.fr/"].map(|d| d.to_string())).into_iter()))
    .build()
}

fn webpage() -> Website {
  Website::new(url)
    .with_openai(openai.clone())
    .with_config(config())
    .with_headers(header()?)
    .with_caching(true)
    .build()
    .unwrap();
}
