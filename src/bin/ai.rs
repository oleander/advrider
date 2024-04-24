use std::collections::BTreeMap;
use std::fs;

use rand::Rng;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use indicatif::ProgressBar;
use anyhow::Result;

const SYSTEM: &str = "
TASK:
You are tasked with processing and summarizing posts from the ADVRider forum, focusing specifically on motorcycle gadgets or accessories discussed within the posts.

STEPS:
1. Read each post and extract information about any gadgets or accessories mentioned.
2. Create a list summarizing these items, ensuring each entry includes both the type (e.g., Windscreen) and the brand (e.g., Something).
3. Review the list to merge new entries with the existing list named LIST.
4. If no new gadgets or accessories are mentioned, return the list from the previous context unchanged.

OUTPUT:
* Return a list of gadgets or accessories.
* Ensure the list is clear and concise, with no explanatory text or wrapping.
* Handle posts with no relevant mentions by returning the existing list as is.

EXAMPLE OUTPUT:
* Cruise control (MC Cruise)
* Lower foot pegs (Rade Garage)
* Race air filter (Rade Garage)
* Phone charger (Quad Lock)
* Tire pressure monitoring system (FoBo 2)
* Taller saddle (Seat Concepts)
* Exhaust (Akrapovic)
* Rally Tower (Nomad ADV)
* Brake Module (SP1)

Note: Adjust the task based on the specifics of the post content and required format.

LIST:
";

fn main() -> Result<()> {
  env_logger::init();

  let posts = fs::read_to_string("posts.json").expect("Failed to read posts.json");
  let posts: BTreeMap<i64, Value> = serde_json::from_str(&posts).expect("Failed to parse JSON");

  let client = Client::new();
  let mut context = json!([]);
  let mut post_ids = posts.keys().cloned().collect::<Vec<i64>>();
  let progress_bar = ProgressBar::new(post_ids.len() as u64);
  let mut str_context: Option<String> = None;

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
    let mut content = vec![format!("POST: {}", post_body)];

    for quote_id in post_quotes {
      let id = quote_id.as_i64().unwrap();
      if let Some(quote_post) = posts.get(&id) {
        let quote_body = quote_post["body"].as_str().unwrap_or_default();
        content.push(format!("QUOTE: {}", quote_body));
      } else {
        eprintln!("Failed to find quote with ID {}", id);
      }
    }

    let f_content = format!("Content: {}", content.join("\n"));
    progress_bar.println(f_content);
    progress_bar.println("".to_string());

    let mut system = SYSTEM.trim().to_string();
    if let Some(ref c) = str_context {
      system.push_str(c.as_str());
    }

    let request = json!({
      "prompt": content.join(" "),
      "model": "mistral:latest",
      "context": context,
      "system": system,
      "stream": false,
      "options": {
        "num_ctx": 8000
      }
    });

    let msg = format!("\nProcessing post #{}", post_id);
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

    context = response["context"].clone();

    let f_context = format!("Context: {}\n", context);
    progress_bar.println(f_context);

    let response = response["response"].as_str().unwrap().to_string();

    str_context = Some(response.clone());

    progress_bar.println("------".to_string());
    let f_response = format!("Response:\n{}", response.trim());
    progress_bar.println(f_response);
    progress_bar.println("------".to_string());
  }

  Ok(())
}
