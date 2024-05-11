//! `cargo run --example url_glob --features glob`
extern crate spider;

use spider::configuration::Configuration;
use spider::tokio;
use spider::website::Website;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let mut b = Configuration::new();
    let config = b.with_depth(1);
    let mut website: Website = Website::new(
      "https://spider.cloud",
    );

    website
      .configuration
      .blacklist_url
      .insert(Default::default())
      .push("https://github.com/oleander".into());

    let mut website = website.with_config(config.clone());

    let start = Instant::now();
    website.scrape().await;
    let duration = start.elapsed();

    let links = website.get_links();
    let Some(pages) = website.get_pages() else {
        println!("No pages found");
        return;
    };

    for page in pages.iter() {
        println!("- {:?}", page);
    }

    for link in links {
        println!("- {:?}", link.as_ref());
    }

    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {:?}",
        duration,
        links.len()
    )
}
