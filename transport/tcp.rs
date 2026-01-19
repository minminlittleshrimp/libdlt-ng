// TCP transport implementation
use crate::traits::Transport;
use std::io::{Result, Read, Write};
use std::net::TcpStream;

pub struct TcpTransport {
    address: String,
    stream: Option<TcpStream>,
}

impl TcpTransport {
    pub fn new(address: &str) -> Self {
        TcpTransport {
            address: address.to_string(),
            stream: None,
        }
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, data: &[u8]) -> Result<usize> {
        if let Some(ref mut stream) = self.stream {
            stream.write(data)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            ))
        }
    }

    fn receive(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let Some(ref mut stream) = self.stream {
            stream.read(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            ))
        }
    }

    fn connect(&mut self) -> Result<()> {
        let stream = TcpStream::connect(&self.address)?;
        self.stream = Some(stream);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<()> {
        self.stream = None;
        Ok(())
    }
}
