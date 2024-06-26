use std::time::Duration;

use async_trait::async_trait;
use colored::*;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use thiserror::Error;
use anyhow::{bail, Result};
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum ControlError {
  #[error("IO error: {0}")]
  Io(#[from] io::Error),
  #[error("Expected '{0}' from '{1}' but got '{2}'")]
  CommandError(String, String, String)
}

const RETRIES: usize = 10;
const PAUSE: Duration = Duration::from_secs(1);

pub struct Control {
  stream: TcpStream
}

impl Control {
  pub async fn new(address: &str) -> Result<Self> {
    Self::new_and_retry(address, 0).await
  }

  pub async fn new_and_retry(address: &str, retries: usize) -> Result<Self> {
    if retries >= RETRIES {
      bail!("Failed to connect to Tor control at {} (retries: {})", address, retries);
    } else if retries > 0 {
      log::warn!("Retrying connection to Tor control at {} @ {}", address, retries);
      sleep(PAUSE).await;
    } else {
      log::info!("Connecting to Tor control at {} @ {}", address, retries);
    }

    match TcpStream::connect(address).await {
      Ok(stream) => {
        return Ok(Self {
          stream
        });
      }
      _ => return Box::pin(Self::new_and_retry(address, retries + 1)).await
    }
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    log::debug!("[{}] -> ({})", command.blue(), expected.green());

    self.stream.write_all(command.as_bytes()).await?;
    self.stream.write_all(b"\n").await?;
    self.stream.flush().await?;

    let actual = self.response().await?;

    if actual == expected {
      log::debug!("[{}] -> ({}) {}", command.blue(), expected.green(), actual.yellow());
      return Ok(());
    } else {
      log::error!("[{}] -> ({}) {}", command.blue(), expected.red(), actual.yellow());
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

#[async_trait]
pub trait Shared {
  async fn send(&mut self, command: &str, expected: &str) -> Result<()>;
  fn control(&mut self) -> &mut Control;
}

#[async_trait]
impl Shared for Authenticated {
  async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    self.control().send(command, expected).await
  }

  fn control(&mut self) -> &mut Control {
    &mut self.control
  }
}

#[async_trait]
impl Shared for Command {
  async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    self.control().send(command, expected).await
  }

  fn control(&mut self) -> &mut Control {
    &mut self.control
  }
}

pub struct Command {
  control: Control,
  auth:    String
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
    while self.liveness().await.is_err() {
      sleep(Duration::from_secs(1)).await;
    }

    Ok(())
  }

  pub async fn refresh(&mut self) -> Result<()> {
    self.newnym().await?;
    self.wait_for_ready().await
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    self.control.send(command, expected).await
  }
}

impl Command {
  pub async fn new(address: &str, password: &str) -> Result<Self> {
    Ok(Self {
      control: Control::new(address).await?,
      auth:    format!("AUTHENTICATE \"{}\"", password)
    })
  }

  pub async fn authenticate(mut self) -> Result<Authenticated> {
    self.send(&self.auth.clone(), "250 OK").await?;

    Ok(Authenticated {
      control: self.control
    })
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    self.control.send(command, expected).await
  }
}
