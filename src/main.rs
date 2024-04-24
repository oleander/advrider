#![allow(dead_code)]
#![allow(unused_imports)]

use std::borrow::Cow;
use std::io::{BufWriter, Write};
use std::fs::OpenOptions;
use std::sync::Arc;

use select::predicate::{Class, Name, Predicate};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use select::document::Document;
use anyhow::{Context, Result};
use reqwest::{header, Client, Proxy};
use tokio::sync::Semaphore;
use select::node::Node;
use regex::Regex;

const BASE_URL: &str = "https://advrider.com/f/threads/husqvarna-701-super-moto-and-enduro.1086621/page-";
const TOTAL_PAGES: usize = 2314;

pub trait StringExtensions {
  fn cleaned(&self) -> String;
}

impl StringExtensions for String {
  fn cleaned(&self) -> String {
    let re = Regex::new(r"\s+").unwrap();
    let temp: Cow<str> = re.replace_all(self, " ");
    temp.trim().to_string()
  }
}

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
  let proxy = Proxy::http("http://127.0.0.1:5566").unwrap();
  Client::builder()
    .default_headers(headers)
    .proxy(proxy)
    .build()
    .unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
struct Post {
  id:       i32,
  is_liked: bool,
  quotes:   Vec<i32>,
  body:     String
}

fn extract_id(node: &Node) -> i32 {
  node
    .attr("id")
    .and_then(|id| id.strip_prefix("post-"))
    .and_then(|id| id.parse::<i32>().ok())
    .unwrap_or_default()
}

fn extract_is_liked(node: &Node) -> bool {
  node
    .find(Class("LikeText").descendant(Name("a")))
    .next()
    .is_some()
}

fn extract_quotes(node: &Node) -> Vec<i32> {
  node
    .find(Class("bbCodeQuote").descendant(Name("a")))
    .filter_map(|a| a.attr("href"))
    .filter_map(|href| href.strip_prefix("goto/post?id="))
    .filter_map(|id| id.split('#').next())
    .filter_map(|id| id.parse::<i32>().ok())
    .collect()
}

fn extract_body(node: &Node) -> Result<String> {
  let result = node
    .find(Class("messageText"))
    .next()
    .context("Failed to find messageText")?
    .children()
    .filter(|child| !child.is(Name("div")))
    .map(|n| n.text())
    .collect::<Vec<_>>()
    .join(" ")
    .replace("\n", " ")
    .trim()
    .to_string()
    .cleaned();
  Ok(result)
}

fn clean_text(raw_input: String) -> String {
  let re = Regex::new(r"\s+").unwrap();
  re.replace_all(&raw_input, " ").trim().to_string()
}

async fn fetch_and_save(document: Document) -> Result<Vec<Post>> {
  let mut posts = Vec::new();
  for node in document.find(Class("message")) {
    let post = Post {
      id:       extract_id(&node),
      is_liked: extract_is_liked(&node),
      quotes:   extract_quotes(&node),
      body:     extract_body(&node).context("Failed to extract body")?
    };

    posts.push(post);
  }

  Ok(posts)
}

async fn download(client: &Client, page: usize) -> Result<String> {
  let url = format!("{}{}", BASE_URL, page);
  let resp = client.get(&url).send().await?;
  let body = resp.text().await?;
  Ok(body)
}

async fn save(page: String) -> Result<()> {
  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open("page.html")
    .context("Failed to open file")?;
  let mut writer = BufWriter::new(file);
  writer
    .write_all(page.as_bytes())
    .context("Failed to write to file")?;
  Ok(())
}

#[tokio::main]
async fn main() {
  let raw_html = std::fs::read_to_string("page.html").unwrap();
  let document = Document::from(raw_html.as_str());
  let result = fetch_and_save(document).await.unwrap();
  println!("{:#?}", result);
}
