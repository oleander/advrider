// extern crate spider;

// use spider::configuration::{GPTConfigs, WaitForIdleNetwork};
// use spider::moka::future::Cache;
// use spider::tokio;
// use spider::website::Website;
// use std::time::Duration;

// #[tokio::main]
// async fn main() {
//     let cache = Cache::builder()
//         .time_to_live(Duration::from_secs(30 * 60))
//         .time_to_idle(Duration::from_secs(5 * 60))
//         .max_capacity(10_000)
//         .build();

//     let mut gpt_config: GPTConfigs = GPTConfigs::new_multi_cache(
//         "gpt-4-1106-preview",
//         vec![
//             "Search for Movies",
//             "Click on the first result movie result",
//         ],
//         500,
//         Some(cache),
//     );
//     gpt_config.set_extra(true);

//     let mut website: Website = Website::new("https://www.google.com")
//         .with_chrome_intercept(true, true)
//         .with_wait_for_idle_network(Some(WaitForIdleNetwork::new(Some(Duration::from_secs(30)))))
//         .with_limit(1)
//         .with_openai(Some(gpt_config))
//         .build()
//         .unwrap();
//     let mut rx2 = website.subscribe(16).unwrap();

//     let handle = tokio::spawn(async move {
//         while let Ok(page) = rx2.recv().await {
//             println!("---\n{}\n{:?}\n{:?}\n---", page.get_url(), page.openai_credits_used, page.extra_ai_data);
//         }
//     });

//     let start = crate::tokio::time::Instant::now();
//     website.crawl().await;
//     let duration = start.elapsed();
//     let links = website.get_links();

//     println!(
//         "(0) Time elapsed in website.crawl() is: {:?} for total pages: {:?}",
//         duration,
//         links.len()
//     );

//     // crawl the page again to see if cache is re-used.
//     let start = crate::tokio::time::Instant::now();
//     website.crawl().await;
//     let duration = start.elapsed();

//     website.unsubscribe();

//     let _ = handle.await;

//     println!(
//         "(1) Time elapsed in website.crawl() is: {:?} for total pages: {:?}",
//         duration,
//         links.len()
//     );
// }

fn main() {
  println!("Hello, world!");
}
