use std::{fmt, io::Error};

const SYNC: u32 = 0xDCC023C2;
pub const START_ID: u16 = 0;
pub const FLAG_ACK: u8 = 0x80;
pub const FLAG_END: u8 = 0x40;
pub const FLAG_RST: u8 = 0x20;
pub const _FLAG_ZER: u8 = 0x3F;
pub const FLAG_SED: u8 = 0x0;
pub const MAX_DATA_SIZE: usize = 0x1000;
pub const PAYLOAD_HEADER_SIZE: usize = 15;
pub const MAX_PAYLOAD_SIZE: usize = MAX_DATA_SIZE + PAYLOAD_HEADER_SIZE;
pub const MAX_SEND_ATTEMPTS: usize = 16;

#[derive(Debug, Clone)]
pub struct Payload {
    pub f_sync: u32,
    pub s_sync: u32,
    pub chksum: u16,
    pub length: u16,
    pub id: u16,
    pub flag: u8,
    pub data: Vec<u8>,
}

impl Payload {
    fn checksum(frame: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // Iterate over the bytes two at a time (16 bits)
        let mut i = 0;
        while i < frame.len() {
            let word = if i + 1 < frame.len() {
                u16::from_be_bytes([frame[i], frame[i + 1]])
            } else {
                u16::from_be_bytes([frame[i], 0]) // Padding if odd number of bytes
            };
            sum += word as u32;

            // Carry-around addition
            if sum > 0xFFFF {
                sum = (sum & 0xFFFF) + (sum >> 16);
            }
            i += 2;
        }

        // One's complement of the result
        !(sum as u16)
    }

    pub fn new(data: Vec<u8>, id: u16, flag: u8) -> Self {
        let mut payload = if data.is_empty() {
            Self {
                f_sync: SYNC,
                s_sync: SYNC,
                chksum: 0,
                length: 0,
                id,
                flag,
                data,
            }
        } else {
            Self {
                f_sync: SYNC,
                s_sync: SYNC,
                chksum: 0,
                length: data.len() as u16,
                id,
                flag,
                data,
            }
        };
            
        payload.chksum = Payload::checksum(&payload.as_bytes_without_checksum());
        payload
    }

    pub fn from_bytes(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() < PAYLOAD_HEADER_SIZE {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Insufficient length",
            ));
        }

        let f_sync = u32::from_be_bytes(<[u8; 4]>::try_from(&payload[..4]).unwrap());
        let s_sync = u32::from_be_bytes(<[u8; 4]>::try_from(&payload[4..8]).unwrap());
        let chksum = u16::from_be_bytes(<[u8; 2]>::try_from(&payload[8..10]).unwrap());
        let length = u16::from_be_bytes(<[u8; 2]>::try_from(&payload[10..12]).unwrap());
        let id = u16::from_be_bytes(<[u8; 2]>::try_from(&payload[12..14]).unwrap());
        let flag = u8::from_be_bytes(<[u8; 1]>::try_from(&payload[14..15]).unwrap());

        if f_sync != SYNC || s_sync != SYNC || f_sync != s_sync {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Received SYNC is invalid",
            ));
        }

        if payload.len() - PAYLOAD_HEADER_SIZE != length as usize {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Insufficient data length",
            ));
        }

        let data = payload[PAYLOAD_HEADER_SIZE..PAYLOAD_HEADER_SIZE + length as usize].to_vec();
        let payload = Self {
            f_sync,
            s_sync,
            chksum,
            length,
            id,
            flag,
            data,
        };

        if !Payload::is_valid_payload_checksum(&payload) {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Passed checksum is incorrect",
            ));
        }

        Ok(payload)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.f_sync.to_be_bytes());
        bytes.extend_from_slice(&self.s_sync.to_be_bytes());
        bytes.extend_from_slice(&self.chksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.flag.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn as_bytes_without_checksum(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.f_sync.to_be_bytes());
        bytes.extend_from_slice(&self.s_sync.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.flag.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }

    fn is_valid_payload_checksum(payload: &Payload) -> bool {
        payload.chksum == Payload::checksum(&payload.as_bytes_without_checksum())
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Payload {{")?;
        write!(f, " f_sync: 0x{:08X},", self.f_sync)?;
        write!(f, " s_sync: 0x{:08X},", self.s_sync)?;
        write!(f, " chksum: 0x{:04X},", self.chksum)?;
        write!(f, " length: {:0>4},", self.length)?;
        write!(f, " id: {},", self.id)?;
        write!(f, " flag: 0x{:02X},", self.flag)?;
        write!(f, " data: [frame data]")?;
        
        // write!(f, " data: ")?;
        // match String::from_utf8(self.data.clone()) {
        //     Ok(data) => write!(f, "{}", data.replace("\n", "\\n"))?,
        //     Err(_) => write!(f, "<Invalid UFT-8 String>")?,
        // };
        write!(f, " }}")
    }
}
