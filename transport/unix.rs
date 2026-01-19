// Unix socket transport implementation with writev() and non-blocking I/O support
use crate::traits::Transport;
use std::io::{Result, Read, Write, IoSlice};
use std::os::unix::net::UnixStream;
use std::os::unix::io::{AsRawFd, RawFd};
use nix::sys::socket::{setsockopt, sockopt};

pub struct UnixSocketTransport {
    socket_path: String,
    stream: Option<UnixStream>,
}

impl UnixSocketTransport {
    pub fn new(socket_path: &str) -> Self {
        UnixSocketTransport {
            socket_path: socket_path.to_string(),
            stream: None,
        }
    }
    
    /// Get raw file descriptor for low-level operations (writev, fcntl, etc.)
    pub fn as_raw_fd(&self) -> Option<RawFd> {
        self.stream.as_ref().map(|s| s.as_raw_fd())
    }
    
    /// Enable non-blocking mode on the socket
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> Result<()> {
        if let Some(ref stream) = self.stream {
            stream.set_nonblocking(nonblocking)?;
        }
        Ok(())
    }
    
    /// Set socket send buffer size (SO_SNDBUF)
    pub fn set_send_buffer_size(&self, size: usize) -> Result<()> {
        if let Some(ref stream) = self.stream {
            setsockopt(stream, sockopt::SndBuf, &size)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok(())
    }
    
    /// Perform writev() syscall for atomic multi-buffer writes
    /// This is the real vectored I/O, much more efficient than multiple send() calls
    pub fn writev(&mut self, buffers: &[&[u8]]) -> Result<usize> {
        if buffers.is_empty() {
            return Ok(0);
        }
        
        if let Some(ref mut stream) = self.stream {
            // Use std's write_vectored which calls writev() on Unix
            let io_slices: Vec<IoSlice> = buffers.iter()
                .map(|buf| IoSlice::new(buf))
                .collect();
            
            stream.write_vectored(&io_slices)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            ))
        }
    }
}

impl Transport for UnixSocketTransport {
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
        let stream = UnixStream::connect(&self.socket_path)?;
        
        // Enable non-blocking I/O for better performance
        stream.set_nonblocking(true)?;
        
        self.stream = Some(stream);
        
        // Optimize socket buffer size (64KB for high-throughput logging)
        let _ = self.set_send_buffer_size(65536);
        
        Ok(())
    }

    fn disconnect(&mut self) -> Result<()> {
        self.stream = None;
        Ok(())
    }
}
