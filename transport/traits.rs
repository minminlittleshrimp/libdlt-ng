// Transport abstraction - allows pluggable communication backends
use std::io::Result;

pub trait Transport: Send {
    fn send(&mut self, data: &[u8]) -> Result<usize>;
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn connect(&mut self) -> Result<()>;
    fn disconnect(&mut self) -> Result<()>;
}

pub trait TransportListener: Send {
    type Connection: Transport;

    fn bind(&mut self) -> Result<()>;
    fn accept(&mut self) -> Result<Self::Connection>;
}
