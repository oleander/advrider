#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;

use reqwest::Client;
use llm_chain::options::{ModelRef, Options};
use llm_chain::traits::Executor;
// use llm_chain_openai::chatgpt::Executor::for_client as chatgpt;
use llm_chain::{executor, parameters, prompt, Parameters};
use llm_chain::chains::map_reduce::Chain;
use serde::{Deserialize, Serialize};
use llm_chain::step::Step;
use anyhow::Result;
use log::info;

const REDUCE_PROMPT: &str = include_str!("../../prompts/reduce.md");
const ARTICLE: &str = include_str!("../../examples/article.md");
const MAP_PROMPT: &str = include_str!("../../prompts/map.md");
const MAX_CONTEXT_SIZE: usize = 2048;
const MODEL_NAME: &str = "llama3";
const MAX_INPUT_SIZE: usize = 1;
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

  // std::env::set_var("OPENAI_API_BASE_URL", "http://127.0.0.1:11434/v1");

  let map_prompt = Step::for_prompt_template(prompt!(MAP_PROMPT, "\n{{text}}"));
  let reduce_prompt = Step::for_prompt_template(prompt!(REDUCE_PROMPT, "\n{{text}}"));
  let chain = Chain::new(map_prompt, reduce_prompt);
  let body = posts.body();
  let body = body
    .lines()
    .take(MAX_INPUT_SIZE)
    .collect::<Vec<&str>>()
    .join("\n");

  use llm_chain::options;
  // ... existing code ...

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

    let options = llm_chain::options::Option::options!(
      MaxContextSize: MAX_CONTEXT_SIZE,
      Temperature: TEMP,
      Model: ModelRef::from_model_name(MODEL_NAME)
    );

    let client = async_openai::Client::new();
    let exec = llm_chain_openai::chatgpt::Executor::for_client(client, options);

    let docs = vec![parameters!(body)];
    let result = chain.run(docs, Parameters::new(), &exec.clone()).await?;
    Ok(())
  }
  Ok(())
}
