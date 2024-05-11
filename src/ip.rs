use reqwest::header::USER_AGENT;
use reqwest::{Client, Proxy};
use anyhow::Result;

pub async fn get() -> Result<String> {
  let proxy = Proxy::all("socks5://127.0.0.1:9050")?;
  let client = Client::builder()
    .proxy(proxy)
    .danger_accept_invalid_certs(true)
    .build()?;

  let res = client
    .get("https://ifconfig.io")
    .header(USER_AGENT, "curl/7.64.1")
    .send()
    .await?;

  let body = res.text().await?;
  Ok(body.trim().to_string())
}
