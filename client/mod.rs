// client: DLT client library for building receivers, control tools, etc.
use dlt_core::DltMessage;
use dlt_transport::{Transport, TcpTransport};

pub struct DltClient {
    transport: Box<dyn Transport>,
    buffer: Vec<u8>,
    pending_data: Vec<u8>,
}

impl DltClient {
    pub fn connect(host: &str, port: u16) -> std::io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut transport = TcpTransport::new(&addr);
        transport.connect()?;

        Ok(DltClient {
            transport: Box::new(transport),
            buffer: vec![0u8; 65536],
            pending_data: Vec::new(),
        })
    }

    pub fn receive_messages(&mut self) -> std::io::Result<Vec<DltMessage>> {
        let mut messages = Vec::new();
        
        match self.transport.receive(&mut self.buffer) {
            Ok(0) => return Ok(messages), // Connection closed
            Ok(n) => {
                // Append new data to pending data
                self.pending_data.extend_from_slice(&self.buffer[..n]);
                
                // Try to parse all complete messages from pending data
                let mut offset = 0;
                while offset < self.pending_data.len() {
                    // Check if we have enough data for at least storage + standard header
                    if self.pending_data.len() - offset < 20 {
                        break;
                    }
                    
                    // Get message length from standard header
                    let std_len = u16::from_le_bytes([
                        self.pending_data[offset + 18],
                        self.pending_data[offset + 19]
                    ]) as usize;
                    
                    // Total message size = storage header (16) + standard header length
                    let total_len = 16 + std_len;
                    
                    // Check if we have the complete message
                    if offset + total_len > self.pending_data.len() {
                        break;
                    }
                    
                    // Parse the message
                    if let Some(msg) = DltMessage::from_bytes(&self.pending_data[offset..offset + total_len]) {
                        messages.push(msg);
                    }
                    
                    offset += total_len;
                }
                
                // Remove processed data from pending buffer
                if offset > 0 {
                    self.pending_data.drain(0..offset);
                }
            }
            Err(e) => return Err(e),
        }
        
        Ok(messages)
    }

    pub fn send_control_message(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.transport.send(data)?;
        Ok(())
    }
}

pub fn parse_message_text(msg: &DltMessage) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    
    if let Some(ref ext) = msg.extended_header {
        let apid = ext.apid.as_str();
        let ctid = ext.ctid.as_str();
        let text = msg.extract_string_payload().unwrap_or_default();

        // Format date/time from storage header
        let secs = msg.storage_header.seconds;
        let usecs = msg.storage_header.microseconds;
        
        let datetime = UNIX_EPOCH + Duration::from_secs(secs as u64);
        let datetime = chrono::DateTime::<chrono::Local>::from(datetime);
        let date_str = datetime.format("%Y/%m/%d %H:%M:%S").to_string();

        // ECU ID
        let ecu = String::from_utf8_lossy(&msg.storage_header.ecu.0)
            .trim_end_matches('\0')
            .to_string();

        // Message counter
        let mcnt = msg.standard_header.mcnt;
        
        // Number of arguments from extended header
        let noar = ext.noar;

        // Format: "YYYY/MM/DD HH:MM:SS.uuuuuu   timestamp mcnt ECU APID CTID log level V/N noar [payload]"
        format!("{}.{:06}   {:10} {:03} {} {:<4} {:<4} log warn V {} [{}]",
            date_str, usecs, secs, mcnt, ecu, apid, ctid, noar, text)
    } else {
        format!("[{}] <no extended header>", msg.storage_header.seconds)
    }
}
