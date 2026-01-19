// DLT protocol implementation - pure data structures, no I/O
use crate::types::{AppId, ContextId, EcuId};
use std::time::{SystemTime, UNIX_EPOCH};

// DLT Storage Header (16 bytes)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DltStorageHeader {
    pub pattern: [u8; 4],      // "DLT\x01"
    pub seconds: u32,
    pub microseconds: u32,
    pub ecu: EcuId,
}

impl DltStorageHeader {
    pub fn new(ecu: EcuId) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        DltStorageHeader {
            pattern: *b"DLT\x01",
            seconds: now.as_secs() as u32,
            microseconds: now.subsec_micros(),
            ecu,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.pattern);
        bytes.extend_from_slice(&self.seconds.to_le_bytes());
        bytes.extend_from_slice(&self.microseconds.to_le_bytes());
        bytes.extend_from_slice(&self.ecu.0);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        let mut pattern = [0u8; 4];
        let mut ecu_bytes = [0u8; 4];
        pattern.copy_from_slice(&bytes[0..4]);
        ecu_bytes.copy_from_slice(&bytes[12..16]);

        Some(DltStorageHeader {
            pattern,
            seconds: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            microseconds: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            ecu: EcuId(ecu_bytes),
        })
    }
}

// DLT Standard Header (4 bytes minimum)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DltStandardHeader {
    pub htyp: u8,              // Header type
    pub mcnt: u8,              // Message counter
    pub len: u16,              // Length (excluding storage header)
}

impl DltStandardHeader {
    pub fn new(has_extended: bool, mcnt: u8, len: u16) -> Self {
        let htyp = if has_extended { 0x35 } else { 0x21 };
        DltStandardHeader { htyp, mcnt, len }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);
        bytes.push(self.htyp);
        bytes.push(self.mcnt);
        bytes.extend_from_slice(&self.len.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        Some(DltStandardHeader {
            htyp: bytes[0],
            mcnt: bytes[1],
            len: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }
}

// DLT Extended Header (10 bytes)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DltExtendedHeader {
    pub msin: u8,              // Message info
    pub noar: u8,              // Number of arguments
    pub apid: AppId,
    pub ctid: ContextId,
}

impl DltExtendedHeader {
    pub fn new(apid: AppId, ctid: ContextId, noar: u8) -> Self {
        DltExtendedHeader {
            msin: 0x01, // Verbose, Log, Info
            noar,
            apid,
            ctid,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10);
        bytes.push(self.msin);
        bytes.push(self.noar);
        bytes.extend_from_slice(&self.apid.0);
        bytes.extend_from_slice(&self.ctid.0);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 10 {
            return None;
        }
        let mut apid = [0u8; 4];
        let mut ctid = [0u8; 4];
        apid.copy_from_slice(&bytes[2..6]);
        ctid.copy_from_slice(&bytes[6..10]);

        Some(DltExtendedHeader {
            msin: bytes[0],
            noar: bytes[1],
            apid: AppId(apid),
            ctid: ContextId(ctid),
        })
    }
}

// Complete DLT Message
#[derive(Debug, Clone)]
pub struct DltMessage {
    pub storage_header: DltStorageHeader,
    pub standard_header: DltStandardHeader,
    pub extended_header: Option<DltExtendedHeader>,
    pub payload: Vec<u8>,
}

impl DltMessage {
    pub fn new_verbose(ecu: EcuId, apid: AppId, ctid: ContextId, message: &str) -> Self {
        // Create verbose payload: type info + string length + string
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x21]); // String type
        let str_len = (message.len() + 1) as u16;
        payload.extend_from_slice(&str_len.to_le_bytes());
        payload.extend_from_slice(message.as_bytes());
        payload.push(0); // null terminator

        let extended_header = Some(DltExtendedHeader::new(apid, ctid, 1));
        let total_len = 4 + 10 + payload.len(); // std + ext + payload

        DltMessage {
            storage_header: DltStorageHeader::new(ecu),
            standard_header: DltStandardHeader::new(true, 0, total_len as u16),
            extended_header,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.storage_header.to_bytes());
        bytes.extend_from_slice(&self.standard_header.to_bytes());
        if let Some(ref ext) = self.extended_header {
            bytes.extend_from_slice(&ext.to_bytes());
        }
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        let storage_header = DltStorageHeader::from_bytes(&bytes[0..16])?;
        let standard_header = DltStandardHeader::from_bytes(&bytes[16..20])?;

        // DLT_HTYP_UEH = 0x01 (use extended header)
        let has_extended = (standard_header.htyp & 0x01) != 0;
        let (extended_header, payload_start) = if has_extended {
            if bytes.len() < 30 {
                return None;
            }
            (DltExtendedHeader::from_bytes(&bytes[20..30]), 30)
        } else {
            (None, 20)
        };

        let payload = bytes[payload_start..].to_vec();

        Some(DltMessage {
            storage_header,
            standard_header,
            extended_header,
            payload,
        })
    }

    pub fn extract_string_payload(&self) -> Option<String> {
        if self.payload.len() > 6 {
            Some(String::from_utf8_lossy(&self.payload[6..])
                .trim_end_matches('\0')
                .to_string())
        } else {
            None
        }
    }
}
