use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};

const SYSTEM: &str = "
You are an AI bot tasked with processing and summarizing posts from the ADVRider forum, specifically those concerning the Husqvarna 701 Enduro motorcycle. Your goal is to distill unique tips, tricks, and insights beneficial for Husqvarna 701 owners, integrating this new information into an existing aggregated knowledge base without duplicating content. Exclude common knowledge or basic maintenance tips.

OUTPUT:

Only the extracted knowlage in list form and do not wrap the answer in anything. Just plain raw output. No thank you. Just the summery as it will be used as input for the next post. Skip info about the motorcycle such as suspension, engine, etc. as it is already known. Do not include the prompt in the output.

The output should only consists of gadgets or accessories that are useful for the Husqvarna 701 Enduro and that's likly to enhance the user experience, like a a new saddle, cruise control or a new windscreen. Do not include the prompt in the output. Be precise and concise and use a markdown list format and include the brand of the product if possible

Shorten the phrasing as much as possible. Only include the name of the gadget and the brand. No justification or explanation is needed
";

// #[derive(Debug, serde::Deserialize)]
// struct Response {
//   context:  Option<Vec<String>>,
//   response: String
// }

fn main() -> Result<()> {
  env_logger::init();

  let posts = fs::read_to_string("posts.json").expect("Failed to read posts.json");
  let posts: BTreeMap<i64, Value> = serde_json::from_str(&posts).expect("Failed to parse JSON");

  let client = Client::new();
  let mut context = None;
  let mut post_ids = posts.keys().cloned().collect::<Vec<i64>>();

  post_ids.sort();
  post_ids.reverse();

  for post_id in post_ids {
    let other = Vec::new();

    let post = posts.get(&post_id).unwrap();
    // println!("Processing post: {}", post_id);
    let post_body = post["body"].as_str().unwrap_or_default();
    // println!("Post body: {}", post_body);
    let post_quotes = post["quotes"].as_array().unwrap_or(&other);
    let mut content = vec![post_body.to_string()];

    for quote_id in post_quotes {
      let id = quote_id.as_i64().unwrap();
      if let Some(quote_post) = posts.get(&id) {
        let quote_body = quote_post["body"].as_str().unwrap_or_default();
        content.push(format!("{}", quote_body));
      } else {
        eprintln!("Failed to find quote with ID {}", id);
      }
    }

    let mut request = json!({
      "system": SYSTEM,
      "prompt": content.join("\n"),
      "model": "mistral:latest",
      "stream": false,
      "options": {
        "num_ctx": 4048
      }
    });

    if let Some(c) = context {
      request["context"] = json!(c);
    }

    log::info!("Sending request: {:?}", request);
    let response = client
      .post("http://localhost:11434/api/generate")
      .json(&request)
      .send()
      .context("Failed to send request")?
      .json::<Value>()?;

    // println!("Response: {:?}", response);

    context = Some(response["context"].as_array().unwrap().clone());
    let response = response["response"].as_str().unwrap();

    println!("Response: {}", response);
    println!();
    // println!("Context: {:?}", context);
  }

  Ok(())
}

// fn send_to_api(client: &Client, messages: &[Value]) -> Option<String> {
//   let request_body = json!({
//       "model": "llama2:13b",
//       "stream": false,
//       "messages": messages,
//   });

//   let response = client
//     .post("http://localhost:11434/api/chat")
//     .json(&request_body)
//     .send()
//     .ok()?;

//   let result: Value = response.json().ok()?;
//   result["message"]["content"].as_str().map(String::from)
// }
