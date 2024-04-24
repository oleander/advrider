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

OUTPUT:
* Return a list of gadgets or accessories.
* Ensure the list is clear and concise, with no explanatory text or wrapping.
* Merge the list from POST and QUOTE and KEEP THESE ACCESSORIES sections.
* If an item is mentioned often, place it higher on the list
* Add a counter how many times the item is mentioned [x]

EXAMPLE OUTPUT:
* Cruise control (MC Cruise) [5]
* Lower foot pegs (Rade Garage) [2]
* Race air filter (Rade Garage)
* Phone charger (Quad Lock)
* Tire pressure monitoring system (FoBo 2)
* Taller saddle (Seat Concepts)
* Exhaust (Akrapovic)
* Rally Tower (Nomad ADV)
* Brake Module (SP1)
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
    // if let Some(ref c) = str_context {
    //   system.push_str("\n");
    //   system.push_str(c.as_str());
    // }

    let prompt = "Extract the gadgets or accessories from the posts and quotes.";
    let prompt = format!("PROMPT:{}\n{}\nKEEP THESE ACCESSORIES:{}", prompt, content.join(" "), str_context.clone().unwrap_or_default());

    let request = json!({
      "prompt": prompt,
      "model": "mistral:latest",
      "system": system,
      "stream": false,
      "options": {
        "num_ctx": 4000,
        "temperature": 0.5,
        // "mirostat": 0.5,
        // "mirostat_eta": 0.5,
        // "repeat_last_n": 1,
        // "repeat_penalty": 1.5,
        // "tfs_z": 0.5,
        // "num_predict": 1,
        "top_k": 50,
        // "top_p": 0.9,
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
    let response = response["response"].as_str().unwrap().to_string();
    str_context = Some(response.clone());

    progress_bar.println("------".to_string());
    let f_response = format!("Response:\n{}", response.trim());
    progress_bar.println(f_response);
    progress_bar.println("------".to_string());
  }

  Ok(())
}
