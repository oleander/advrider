use std::sync::Arc;
use std::time::Duration;
use core::future::AsyncDrop;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use thiserror::Error;
use anyhow::Result;
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum ControlError {
  #[error("IO error: {0}")]
  Io(#[from] io::Error),
  #[error("Expected '{0}' from '{1}' but got '{2}'")]
  CommandError(String, String, String)
}

#[derive(Debug, Clone)]
pub struct Control {
  stream: Arc<Mutex<TcpStream>>
}

impl Control {
  pub async fn new(address: &str) -> Result<Self> {
    let stream = TcpStream::connect(address).await?;
    Ok(Self {
      stream: Arc::new(Mutex::new(stream))
    })
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    let mut stream = self.stream.lock().await;

    let mut reader = BufReader::new(&mut *stream);
    let mut response = String::new();

    reader.read_line(&mut response).await?;
    log::debug!("Received response: '{}'", response);
    let actual = response.trim().to_string();

    if actual == expected {
      return Ok(());
    }

    Err(ControlError::CommandError(expected.to_string(), command.to_string(), actual).into())
  }
}

#[derive(Debug, Clone)]
pub struct Command {
  control:  Arc<Mutex<Control>>,
  password: String,
  open:     bool
}

impl Command {
  pub async fn new(address: &str, password: &str) -> Result<Self> {
    let control = Control::new(address).await?;
    let mut cmd = Self {
      control:  Arc::new(Mutex::new(control)),
      password: password.to_string(),
      open:     false
    };
    cmd.wait_for_ready().await?;
    Ok(cmd)
  }

  pub async fn authenticate(&mut self) -> Result<()> {
    if self.open {
      return Ok(());
    }

    self
      .send(&format!("AUTHENTICATE \"{}\"", self.password), "250 OK")
      .await?;
    self.open = true;
    Ok(())
  }

  pub async fn quit(&mut self) -> Result<()> {
    if !self.open {
      return Ok(());
    }

    self.send("QUIT", "250 closing connection").await?;
    self.open = false;
    Ok(())
  }

  pub async fn newnym(&mut self) -> Result<()> {
    self.send("SIGNAL NEWNYM", "XX").await
  }

  pub async fn liveness(&mut self) -> Result<()> {
    self
      .send("GETINFO network-liveness", "250-network-liveness=up")
      .await
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
    // self.quit().await
    Ok(())
  }

  pub async fn refresh(&mut self) -> Result<()> {
    self.authenticate().await?;
    self.newnym().await?;
    self.wait_for_ready().await
    // self.quit().await
  }

  pub async fn send(&mut self, command: &str, expected: &str) -> Result<()> {
    log::debug!("Sending command '{}' and expecting '{}'", command, expected);
    self.control.lock().await.send(command, expected).await
  }
}

impl Drop for Command {
  fn drop(&mut self) {
      if Arc::strong_count(&self.control) == 1 {
          let control = self.control.clone();
          tokio::spawn(async move {
              let mut control = control.lock().await;
              control.quit().await.expect("Failed to quit Tor control");
          });
      }
  }
}
