use std::io::Error;

const SYNC: u32 = 0xDCC023C2;
pub const _FLAG_ACK: u8 = 0x80;
pub const _FLAG_END: u8 = 0x40;
pub const _FLAG_RST: u8 = 0x20;
pub const FLAG_ZER: u8 = 0x3F;

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
        let mut payload = Self {
            f_sync: SYNC,
            s_sync: SYNC,
            chksum: 0,
            length: data.len() as u16,
            id,
            flag,
            data,
        };

        payload.chksum = Payload::checksum(&payload.as_bytes());
        println!("{}", payload.chksum);
        payload
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 15 {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Insufficient data length",
            ));
        }

        let f_sync = u32::from_be_bytes(<[u8; 4]>::try_from(&data[..4]).unwrap());
        let s_sync = u32::from_be_bytes(<[u8; 4]>::try_from(&data[4..8]).unwrap());
        let chksum = u16::from_be_bytes(<[u8; 2]>::try_from(&data[8..10]).unwrap());
        let length = u16::from_be_bytes(<[u8; 2]>::try_from(&data[10..12]).unwrap());
        let id = u16::from_be_bytes(<[u8; 2]>::try_from(&data[12..14]).unwrap());
        let flag = u8::from_be_bytes(<[u8; 1]>::try_from(&data[14..15]).unwrap());

        if data.len() < 15 + length as usize {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Insufficient data length",
            ));
        }

        let data = data[15..15 + length as usize].to_vec();

        Ok(Self {
            f_sync,
            s_sync,
            chksum,
            length,
            id,
            flag,
            data,
        })
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
}
