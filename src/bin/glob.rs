//! `cargo run --example url_glob --features glob`
extern crate spider;

use std::time::Instant;
use std::vec;

use spider::configuration::Configuration;
use spider::tokio;
use spider::website::Website;

use crate::spider::http_cache_reqwest::CacheManager;

#[tokio::main]
async fn main() {
  let mut b = Configuration::new();
  let config = b
    .with_depth(1)
    .with_proxies(vec!["socks5://127.0.0.1:9050".to_string()].into())
    .with_caching(true);

  let mut website: Website = Website::new("https://advrider.com/f/threads/thinwater-escapades.1502022/page-[1-5]");

  website
    .configuration
    .blacklist_url
    .insert(Default::default())
    .push("https://github.com/oleander".into());

  let website = website.with_config(config.clone()).with_caching(true);

  let start = Instant::now();
  website.scrape().await;
  let duration = start.elapsed();

  let links = website.get_links();
  let Some(pages) = website.get_pages() else {
    println!("No pages found");
    return;
  };

  for page in pages.iter() {
    if let Some(bytes) =  page.get_bytes() {
      println!("Page size is: {:?}", bytes.len());
    } else {
      println!("Nothing returned");
    }
  }

  for link in links {
    println!("- {:?}", link.as_ref());
  }

  println!("Time elapsed in website.crawl() is: {:?} for total pages: {:?}", duration, links.len())
}
