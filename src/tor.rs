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
    let stream = TcpStream::connect(address).await?;
    Ok(Self {
      stream
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
  control:  Control,
  password: String
}

impl Command {
  pub async fn new(address: &str, password: &str) -> Result<Self> {
    let control = Control::new(address).await?;
    let mut cmd = Self {
      control,
      password: password.to_string()
    };
    cmd.authenticate().await?;
    Ok(cmd)
  }

  pub async fn authenticate(&mut self) -> Result<()> {
    self.send(&format!("AUTHENTICATE \"{}\"", self.password), "250 OK").await
  }

  pub async fn quit(&mut self) -> Result<()> {
    self.send("QUIT", "250 closing connection").await
  }

  pub async fn newnym(&mut self) -> Result<()> {
    self.send("SIGNAL NEWNYM", "XX").await
  }

  pub async fn liveness(&mut self) -> Result<()> {
    self.send("GETINFO network-liveness", "250-network-liveness=up").await
  }

  pub async fn wait_for_ready(&mut self) -> Result<()> {
    self.authenticate().await?;

    log::info!("Waiting for Tor to be ready ...");
    while let Err(err) = self.liveness().await {
      log::warn!("Tor is not ready yet: {}, wait ...", err);
      sleep(Duration::from_secs(1)).await;
      log::info!("Checking Tor status again ...");
    }

    log::info!("Tor is ready!");
    self.quit().await
  }

  pub async fn refresh(&mut self) -> Result<()> {
    self.authenticate().await?;
    self.newnym().await?;
    self.liveness().await?;
    self.quit().await
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    log::debug!("Sending command '{}' and expecting '{}'", command, expected);
    self.control.send(command, expected).await
  }
}
