use std::collections::BTreeMap;
use std::fs;

use rand::Rng;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use indicatif::ProgressBar;
use anyhow::Result;

const SYSTEM: &str = "
You are an AI bot tasked with processing and summarizing posts from the ADVRider forum, specifically those concerning the Husqvarna 701 Enduro motorcycle. Your goal is to distill unique tips, tricks, and insights beneficial for Husqvarna 701 owners, integrating this new information into an existing aggregated knowledge base without duplicating content. Exclude common knowledge or basic maintenance tips.

OUTPUT:

Only the extracted knowlage in list form and do not wrap the answer in anything. Just plain raw output. No thank you. Just the summery as it will be used as input for the next post. Skip info about the motorcycle such as suspension, engine, etc. as it is already known. Do not include the prompt in the output.

The output should only consists of gadgets or accessories that are useful for the Husqvarna 701 Enduro and that's likly to enhance the user experience, like a a new saddle, cruise control or a new windscreen. Do not include the prompt in the output. Be precise and concise and use a markdown list format and include the brand of the product if possible

Shorten the phrasing as much as possible. Only include the name of the gadget and the brand. No justification or explanation is needed

Only use information provided in the prompt. Do not include any additional information or make assumptions or by the user.
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
  let mut context: Option<Vec<u64>> = None;
  let mut post_ids = posts.keys().cloned().collect::<Vec<i64>>();
  let progress_bar = ProgressBar::new(post_ids.len() as u64);

  post_ids.sort();
  post_ids.reverse();

  let mut post_ids = post_ids.clone().into_iter().collect::<Vec<i64>>();
  post_ids.sort_by(|_, _| {
    let mut rng = rand::thread_rng();
    rng.gen::<i64>().cmp(&rng.gen::<i64>())
  });

  for post_id in post_ids {
    progress_bar.inc(1);

    let other = Vec::new();
    let post = posts.get(&post_id).unwrap();
    let post_body = post["body"].as_str().unwrap_or_default();
    let post_quotes = post["quotes"].as_array().unwrap_or(&other);
    let mut content = vec![post_body.to_string()];

    // sort post_quotes by random

    for quote_id in post_quotes {
      let id = quote_id.as_i64().unwrap();
      if let Some(quote_post) = posts.get(&id) {
        let quote_body = quote_post["body"].as_str().unwrap_or_default();
        content.push(format!("{}", quote_body));
      } else {
        eprintln!("Failed to find quote with ID {}", id);
      }
    }

    let f_content = format!("Content: {}", content.join("\n"));
    progress_bar.println(f_content);
    progress_bar.println("".to_string());

    let mut request = json!({
      "prompt": content.join("\n"),
      "model": "mistral:latest",
      "system": SYSTEM,
      "stream": false,
      "options": {
        "num_ctx": 4048
      }
    });

    if let Some(c) = context.clone() {
      request["context"] = json!(c);
    }

    let msg = format!("\nProcessing post #{}\n", post_id);
    progress_bar.println(msg);
    progress_bar.println("".to_string());

    let response = client
      .post("http://localhost:11434/api/generate")
      .json(&request)
      .send();

    let Ok(response) = response else {
      progress_bar.println("Failed to parse response");
      progress_bar.println("".to_string());
      continue;
    };

    let response = response.json::<Value>()?;

    let inner_context = response["context"]
      .as_array()
      .unwrap()
      .clone()
      .into_iter()
      .collect::<Vec<Value>>()
      .iter()
      .map(|v| v.as_u64().unwrap())
      .collect::<Vec<u64>>();

    context = Some(inner_context);

    let response = response["response"].as_str().unwrap();

    let f_response = format!("Response: {}", response);
    progress_bar.println(f_response);
    progress_bar.println("".to_string());
  }

  Ok(())
}
