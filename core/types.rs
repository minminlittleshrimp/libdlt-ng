// Core types used across all DLT components
use std::fmt;

// Application ID (4 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppId(pub [u8; 4]);

impl AppId {
    pub fn new(s: &str) -> Self {
        let mut id = [0u8; 4];
        let bytes = s.as_bytes();
        let len = bytes.len().min(4);
        id[..len].copy_from_slice(&bytes[..len]);
        AppId(id)
    }

    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.0).trim_end_matches('\0').to_string()
    }
}

// Context ID (4 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextId(pub [u8; 4]);

impl ContextId {
    pub fn new(s: &str) -> Self {
        let mut id = [0u8; 4];
        let bytes = s.as_bytes();
        let len = bytes.len().min(4);
        id[..len].copy_from_slice(&bytes[..len]);
        ContextId(id)
    }

    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.0).trim_end_matches('\0').to_string()
    }
}

// ECU ID (4 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcuId(pub [u8; 4]);

impl EcuId {
    pub fn new(s: &str) -> Self {
        let mut id = [0u8; 4];
        let bytes = s.as_bytes();
        let len = bytes.len().min(4);
        id[..len].copy_from_slice(&bytes[..len]);
        EcuId(id)
    }
}

// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Fatal = 1,
    Error = 2,
    Warn = 3,
    Info = 4,
    Debug = 5,
    Verbose = 6,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogLevel::Fatal => write!(f, "FATAL"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Verbose => write!(f, "VERBOSE"),
        }
    }
}

// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Log = 0,
    AppTrace = 1,
    NwTrace = 2,
    Control = 3,
}
