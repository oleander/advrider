use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::sync::Arc;

use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use select::node::Node;
use serde::{Deserialize, Serialize};
use select::document::Document;
use reqwest::{header, Client};
use select::predicate::Class;
use tokio::sync::Semaphore;

#[derive(Serialize, Deserialize, Debug)]
struct Post {
  post: String,
  page: usize
}

const BASE_URL: &str = "https://advrider.com/f/threads/husqvarna-701-super-moto-and-enduro.1086621/page-";
const TOTAL_PAGES: usize = 2314;

async fn setup_client() -> Client {
  let mut headers = header::HeaderMap::new();
  headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap());
  headers.insert("Accept-Language", "en-GB,en-US;q=0.9,en;q=0.8,sv;q=0.7".parse().unwrap());
  headers.insert("Cookie", "_gcl_au=1.1.82089729.1713720688; _gid=GA1.2.1985556420.1713720688; xf_logged_in=1; xf_session=867c856b44b55341ea9c2e9b34fe6808; _ga_BCFR910NDY=GS1.1.1713720688.1.1.1713722217.0.0.0; _ga=GA1.2.469143051.1713720688".parse().unwrap());
  headers.insert("DNT", "1".parse().unwrap());
  headers.insert(
    "Sec-CH-UA",
    "\"Opera\";v=\"109\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\""
      .parse()
      .unwrap()
  );
  headers.insert("Sec-CH-UA-Mobile", "?0".parse().unwrap());
  headers.insert("Sec-CH-UA-Platform", "\"macOS\"".parse().unwrap());
  headers.insert("Sec-Fetch-Dest", "document".parse().unwrap());
  headers.insert("Sec-Fetch-Mode", "navigate".parse().unwrap());
  headers.insert("Sec-Fetch-Site", "none".parse().unwrap());
  headers.insert("Sec-Fetch-User", "?1".parse().unwrap());
  headers.insert("Upgrade-Insecure-Requests", "1".parse().unwrap());
  headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 OPR/109.0.0.0".parse().unwrap());

  Client::builder().default_headers(headers).build().unwrap()
}

fn clean_text(input: &str) -> String {
  let mut text = input.trim().to_string(); // Trim leading and trailing whitespace
  let re = Regex::new(r"\s+").unwrap(); // Regex to match sequences of whitespace
  text = re.replace_all(&text, " ").to_string(); // Replace all sequences of whitespace with a single space
  text
}

fn extract_text(node: &Node) -> String {
    node.children()
        .filter(|child| !child.is(select::predicate::Name("aside")))
        .filter(|child| !child.is(select::predicate::Class("bbCodeBlock")))
        .filter(|child| !child.is(select::predicate::Class("bbCodeQuote")))
        .map(|n| n.text())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

async fn fetch_and_save(page: usize, client: Client, semaphore: Arc<Semaphore>, progress_bar: Arc<ProgressBar>) {
  let _permit = semaphore.acquire().await.unwrap();
  let url = format!("{}{}", BASE_URL, page);
  match client.get(&url).send().await {
    Ok(resp) => {
      if let Ok(text) = resp.text().await {
        let document = Document::from(text.as_str());
        let posts: Vec<Post> = document
          .find(Class("baseHtml"))
          .map(|node| {
            Post {
              post: clean_text(&extract_text(&node)), // Extracting text with exclusion of <aside>
              page
            }
          })
          .collect();

        let mut file = OpenOptions::new()
          .create(true)
          .append(true)
          .open("output.json")
          .unwrap();
        let writer = BufWriter::new(&file);
        serde_json::to_writer(writer, &posts).unwrap();
        writeln!(file).unwrap(); // Ensures JSON objects are written line by line
      }
    }
    Err(_) => println!("Failed to fetch page {}", page)
  }
  progress_bar.inc(1);
}

#[tokio::main]
async fn main() {
  let client = setup_client().await;
  let semaphore = Arc::new(Semaphore::new(3)); // Limit to 3 concurrent requests
  let progress_bar = Arc::new(ProgressBar::new(TOTAL_PAGES as u64));
  progress_bar.set_style(
    ProgressStyle::default_bar()
      .template("{wide_bar} {pos}/{len}")
      .expect("Failed to set progress bar style")
  );

  let mut handles = vec![];
  for page in 1..=TOTAL_PAGES {
    let client = client.clone();
    let semaphore = semaphore.clone();
    let progress_bar = progress_bar.clone();
    handles.push(tokio::spawn(async move {
      fetch_and_save(page, client, semaphore, progress_bar).await;
    }));
  }

  for handle in handles {
    handle.await.unwrap();
  }

  progress_bar.finish_with_message("Download complete.");
}
