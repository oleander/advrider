#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::time::Duration;

use async_openai::config::OpenAIConfig;
use llm_chain::traits::Executor as ExecutorTrait;
use llm_chain::{executor, options, parameters, prompt, Parameters};
use llm_chain_openai::chatgpt::Executor;
use reqwest::Client;
use llm_chain::options::{ModelRef, Options};
use llm_chain::chains::map_reduce::Chain;
use serde::{Deserialize, Serialize};
use llm_chain::step::Step;
use anyhow::{bail, Context, Result};
use log::info;

const REDUCE_PROMPT: &str = include_str!("../../prompts/reduce.md");
const ARTICLE: &str = include_str!("../../examples/article.md");
const MAP_PROMPT: &str = include_str!("../../prompts/map.md");
// const MAX_CONTEXT_SIZE: usize = 3048;
const MODEL_NAME: &str = "gpt-3.5-turbo";
const MAX_INPUT_SIZE: usize = 30000_usize;
// const TEMP: f32 = 0.1;

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
  // static ref MODEL: ModelRef = ModelRef::from_model_name(MODEL_NAME);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  env_logger::init();

  let posts = Posts::new()?;
  info!("Found posts: {:?}", posts.len());

  let map_prompt = Step::for_prompt_template(prompt!(MAP_PROMPT, "\n{{text}}"));
  let reduce_prompt = Step::for_prompt_template(prompt!(REDUCE_PROMPT, "\n{{text}}"));
  let chain = Chain::new(map_prompt, reduce_prompt);

  let body = posts
    .body()
    .chars()
    .take(MAX_INPUT_SIZE)
    .collect::<String>();

  let model = ModelRef::from_model_name(MODEL_NAME);

  let options = options!(
    // ThrottleDelay: Duration::from_millis(400),
    RepeatPenaltyLastN: 1_usize,
    MaxContextSize: 3048_usize,
    // MaxBatchSize: 3000_usize,
    Temperature: 0.01_f32,
    NThreads: 3_usize,
    Model: model,
    Stream: false
  );

  let exec = executor!(chatgpt, options)?;
  let docs = vec![parameters!(body)];

  use llm_chain::output::Output::{Immediate, Stream};

  let result = chain.run(docs, Parameters::new(), &exec).await;
  let raw = match result {
    Ok(Immediate(out)) => out.primary_textual_output().context("NOPE"),
    Ok(Stream(_)) => bail!("Invalid stream returned"),
    Err(reason) => bail!("Failed running chain: {}", reason)
  };

  // Write raw to results.md
  let output = html2text::from_read(raw.context("Something failed")?.as_bytes(), usize::MAX);
  let result_path = "data/results.md";
  info!("Writing calculated result to {} @ {} bytes", result_path, output.len());
  fs::write(result_path, output).context("Unable to write file")
}
