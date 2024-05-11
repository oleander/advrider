use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Error, Proxy};

pub async fn get() -> Result<String, Error> {
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
