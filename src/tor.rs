use std::time::Duration;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use thiserror::Error;
use anyhow::Result;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum ControlError {
  #[error("IO error: {0}")]
  Io(#[from] io::Error),
  #[error("Expected '{0}' from '{1}' but got '{2}'")]
  CommandError(String, String, String)
}

pub struct Control {
  stream: TcpStream
}

impl Control {
  pub async fn new(address: &str) -> Result<Self> {
    Ok(Self {
      stream: TcpStream::connect(address).await?
    })
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    self.stream.write_all(command.as_bytes()).await?;
    self.stream.write_all(b"\n").await?;
    self.stream.flush().await?;

    let actual = self.response().await?;
    if actual == expected {
      return Ok(());
    }

    Err(ControlError::CommandError(expected.to_string(), command.to_string(), actual).into())
  }

  async fn response(&mut self) -> Result<String> {
    let mut reader = BufReader::new(&mut self.stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    Ok(response.trim().to_string())
  }
}

pub struct Command {
  control: Control,
  auth:    String,
  open:    bool
}

pub struct Authenticated {
  control: Control
}

impl Authenticated {
  pub async fn quit(mut self) -> Result<()> {
    self.send("QUIT", "250 closing connection").await?;
    Ok(())
  }

  pub async fn newnym(&mut self) -> Result<()> {
    self.send("SIGNAL NEWNYM", "250 OK").await
  }

  pub async fn liveness(&mut self) -> Result<()> {
    self
      .send("GETINFO network-liveness", "250-network-liveness=up")
      .await
  }

  pub async fn wait_for_ready(&mut self) -> Result<()> {
    log::info!("Waiting for Tor to be ready ...");
    while let Err(err) = self.liveness().await {
      log::warn!("Tor is not ready yet: {}, wait ...", err);
      sleep(Duration::from_secs(1)).await;
      log::info!("Checking Tor status again ...");
    }

    log::info!("Tor is ready!");
    Ok(())
  }

  pub async fn refresh(&mut self) -> Result<()> {
    self.newnym().await?;
    self.wait_for_ready().await
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    log::debug!("Sending command '{}' and expecting '{}'", command, expected);
    self.control.send(command, expected).await
  }
}

impl Command {
  pub async fn new(address: &str, password: &str) -> Result<Self> {
    Ok(Self {
      control: Control::new(address).await?,
      auth:    format!("AUTHENTICATE \"{}\"", password),
      open:    false
    })
  }

  pub async fn authenticate(mut self) -> Result<Authenticated> {
    self.send(&self.auth.clone(), "250 OK").await?;

    Ok(Authenticated {
      control: self.control
    })
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    log::debug!("Sending command '{}' and expecting '{}'", command, expected);
    self.control.send(command, expected).await
  }
}
