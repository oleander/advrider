use llm_chain::{executor, options, parameters, prompt, Parameters};
use llm_chain::chains::map_reduce::Chain;
use llm_chain::options::ModelRef;
use llm_chain::step::Step;
use anyhow::Result;
use log::info;

const REDUCE_PROMPT: &str = include_str!("/Users/linus/.config/fabric/patterns/summarize/system.md");
const MAP_PROMPT: &str = include_str!("/Users/linus/.config/fabric/patterns/summarize/system.md");
const ARTICLE: &str = include_str!("../../examples/article.md");
const MODEL_NAME: &str = "gpt-4-turbo";

lazy_static::lazy_static! {
  static ref API_KEY: String = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not defined");
  static ref MODEL: ModelRef = ModelRef::from_model_name(MODEL_NAME);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  env_logger::init();

  let map_prompt = Step::for_prompt_template(prompt!(MAP_PROMPT, "\n{{text}}"));
  let reduce_prompt = Step::for_prompt_template(prompt!(REDUCE_PROMPT, "\n{{text}}"));
  let chain = Chain::new(map_prompt, reduce_prompt);

  let model = ModelRef::from_model_name(MODEL_NAME);
  let docs = vec![parameters!(ARTICLE)];
  let api_key = API_KEY.clone();

  let options = options!(
    MaxContextSize: 4000 as usize,
    Temperature: 0.01,
    Model: model,
    ApiKey: api_key
  );

  let exec = executor!(chatgpt, options)?;
  let result = chain.run(docs, Parameters::new(), &exec).await?;
  info!("Result: {}", result);

  Ok(())
}
