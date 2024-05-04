#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;

use llm_chain::{executor, options, parameters, prompt, Parameters};
use llm_chain::chains::map_reduce::Chain;
use llm_chain::options::ModelRef;
use llm_chain::step::Step;
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};

const REDUCE_PROMPT: &str = include_str!("../../prompts/reduce.md");
const MAP_PROMPT: &str = include_str!("../../prompts/map.md");
const ARTICLE: &str = include_str!("../../examples/article.md");
const MODEL_NAME: &str = "gpt-4";
const MAX_INPUT_SIZE: usize = 200;
const MAX_CONTEXT_SIZE: usize = 2048;
const TEMP: f32 = 0.1;

type ID = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
  quotes:   Vec<ID>,
  body:     String,
  is_liked: bool,
  id:       ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Posts {
  posts: BTreeMap<ID, Post>
}

impl Posts {
  fn new() -> Result<Self> {
    let data = fs::read_to_string("posts.json").expect("Unable to read file");
    Ok(Self {
      posts: serde_json::from_str(&data)?
    })
  }

  fn len(&self) -> usize {
    self.posts.len()
  }

  fn body(&self) -> String {
    let posts = self.posts.values();
    let bodies = posts.map(|post| post.body.clone()).collect::<Vec<String>>();
    let body = bodies.join("\n");
    body
  }
}

lazy_static::lazy_static! {
  static ref API_KEY: String = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not defined");
  static ref MODEL: ModelRef = ModelRef::from_model_name(MODEL_NAME);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  env_logger::init();

  let posts = Posts::new()?;
  info!("Found posts: {:?}", posts.len());

  let map_prompt = Step::for_prompt_template(prompt!(MAP_PROMPT, "\n{{text}}"));
  let reduce_prompt = Step::for_prompt_template(prompt!(REDUCE_PROMPT, "\n{{text}}"));
  let chain = Chain::new(map_prompt, reduce_prompt);
  let body = posts.body();
  let body = body
    .lines()
    .take(MAX_INPUT_SIZE)
    .collect::<Vec<&str>>()
    .join("\n");

  let model = ModelRef::from_model_name(MODEL_NAME);
  let docs = vec![parameters!(body)];
  let api_key = API_KEY.clone();

  let options = options!(
    MaxContextSize: MAX_CONTEXT_SIZE,
    Temperature: TEMP,
    ApiKey: api_key,
    Model: model
  );

  let exec = executor!(chatgpt, options)?;
  let result = chain.run(docs, Parameters::new(), &exec).await?;
  info!("Result: {}", result);

  Ok(())
}