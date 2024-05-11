use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use thiserror::Error;
use anyhow::Result;

#[derive(Error, Debug)]
pub enum ControlError {
  #[error("IO error: {0}")]
  Io(#[from] io::Error),
  #[error("Command failed with response: {0}")]
  CommandError(String)
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

  pub async fn send(&mut self, command: &str) -> Result<String> {
    self.stream.write_all(command.as_bytes()).await?;
    self.stream.write_all(b"\n").await?;
    self.stream.flush().await?;
    self.response().await
  }

  async fn response(&mut self) -> Result<String> {
    let mut reader = BufReader::new(&mut self.stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let trimmed = response.trim().to_string();
    if !trimmed.starts_with("250") {
      Err(ControlError::CommandError(trimmed).into())
    } else {
      Ok(trimmed)
    }
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
    let command = format!("AUTHENTICATE \"{}\"", self.password);
    self.send(&command).await?;
    Ok(())
  }

  pub async fn quit(&mut self) -> Result<()> {
    self.send("QUIT").await?;
    Ok(())
  }

  pub async fn newnym(&mut self) -> Result<()> {
    self.send("SIGNAL NEWNYM").await?;
    Ok(())
  }

  pub async fn liveness(&mut self) -> Result<()> {
    self.send("GETINFO network-liveness").await?;
    Ok(())
  }

  pub async fn send(&mut self, command: &str) -> Result<()> {
    self.control.send(command).await.map(|_| ())
  }
}
