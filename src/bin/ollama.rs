use std::io::{stdout, Write};

use async_openai::types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use llm_chain::output::StreamExt;
use async_openai::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
  dotenv::dotenv().ok();

  let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
  let api_host = std::env::var("OPENAI_API_HOST").expect("OPENAI_API_HOST must be set");
  let api_model = "llama3:latest";
  let max_tokens = 512u16;

  let messages = match ChatCompletionRequestUserMessageArgs::default()
    .content("Write a marketing blog praising and introducing Rust library async-openai")
    .build()
  {
    Ok(msg) => msg.into(),
    Err(e) => {
      println!("Error: {}", e);
      assert!(false);
      return;
    }
  };
  let client = Client::with_config(
    OpenAIConfig::new()
      .with_api_key(&api_key)
      .with_api_base(&api_host)
  );
  let request = match CreateChatCompletionRequestArgs::default()
    .model(api_model)
    .max_tokens(max_tokens)
    .messages([messages])
    .build()
  {
    Ok(req) => req,
    Err(e) => {
      println!("Error: {}", e);
      assert!(false);
      return;
    }
  };

  let stream_result = client.chat().create_stream(request).await;
  let mut stream = match stream_result {
    Ok(s) => s,
    Err(e) => {
      println!("Error: {}", e);
      assert!(false);
      return;
    }
  };

  let mut lock = stdout().lock();

  while let Some(result) = stream.next().await {
    match result {
      Ok(response) => {
        response.choices.iter().for_each(|chat_choice| {
          if let Some(ref content) = chat_choice.delta.content {
            write!(lock, "{}", content).unwrap();
          }
        });
      }
      Err(err) => {
        println!("Error: {}", err);
        // jsonify error
        let err = json!({
            "error": err.to_string()
        });
        println!("error: {}", err);
        writeln!(lock, "error: {err}").unwrap();
      }
    }
    match stdout().flush() {
      Ok(_) => (),
      Err(e) => {
        println!("Error: {}", e);
        assert!(false);
        return;
      }
    }
  }
}
