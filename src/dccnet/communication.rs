use std::{
    fmt, io::{Read, Write}, net::TcpStream, thread, time::{Duration, Instant}
};

use super::network;
use super::network::Payload;

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum NetworkErrorKind {
    ConnectionError,
    ProtocolError,
    RSTError,
    UnexpectedFlagError,
    RetransmissionError,
    InvalidIdError,
    Other,
}

impl fmt::Display for NetworkErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct NetworkError {
    pub kind: NetworkErrorKind,
    pub message: String,
}

impl NetworkError {
    pub fn new(kind: NetworkErrorKind, message: &str) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

#[inline(always)]
pub fn next_id(id: u16) -> u16 {
    (id + 1) % 2
}

fn check_received_rst(payload: &Payload) -> Result<(), NetworkError> {
    if payload.flag == network::FLAG_RST && payload.id == u16::MAX {
        let payload_msg = String::from_utf8(payload.data.clone()).unwrap_or_else(|_| {
            String::from("Invalid UTF-8 sequence")
        });

        return Err(NetworkError::new(
            NetworkErrorKind::RSTError,
            &format!("Received RST: {}", payload_msg),
        ));
    }

    Ok(())
}

pub fn send_frame(stream: &mut TcpStream, payload: &Payload) -> Result<usize, NetworkError> {
    const RETRANSMISSION_DELAY: u64 = 200;

    for curr_attempt in 0..network::MAX_SEND_ATTEMPTS {
        if let Err(e) = stream.write_all(&payload.as_bytes()) {
            return Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                &format!("Failed to send frame: {}", e),
            ));
        }
        println!("SEND     {payload}");
        
        let start = Instant::now();

        match wait_ack(stream, payload.id) {
            Ok(_) => {
                if curr_attempt > 0 {
                    println!("SUCCESS RETRANSMISSION");
                    let time = start.elapsed().as_millis() as u64;
    
                    if time < RETRANSMISSION_DELAY {
                        thread::sleep(Duration::from_millis(RETRANSMISSION_DELAY - time));
                    }
                }
                return Ok(curr_attempt);
            }
            Err(e) => {
                if e.kind == NetworkErrorKind::RSTError {
                    return Err(e);
                }
            }
        }

        let time = start.elapsed().as_millis() as u64;

        println!("({curr_attempt}) RETRANSMISSION RECEIVE TIME {time}ms");

        if time < RETRANSMISSION_DELAY {
            println!("WAIT {}ms", RETRANSMISSION_DELAY - time);
            thread::sleep(Duration::from_millis(RETRANSMISSION_DELAY - time));
        }
    }

    Err(NetworkError::new(
        NetworkErrorKind::RetransmissionError,
        "Failed to send frame after maximum attempts",
    ))
}

pub fn receive_frame(stream: &mut TcpStream) -> Result<Payload, NetworkError> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    let bytes_read = stream.read(&mut buf).map_err(|e| {
        NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Timeout error: {}", e),
        )
    })?;

    let payload = match Payload::from_bytes(&buf[..bytes_read]) {
        Ok(p) => p,
        Err(e) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ProtocolError,
                &format!("Failed to parse payload: {}", e),
            ));
        }
    };
    
    check_received_rst(&payload)?;

    if payload.flag == network::FLAG_ACK {
        return Err(NetworkError::new(
            NetworkErrorKind::UnexpectedFlagError,
            "Received ACK instead of data",
        ));
    }

    println!("RECV \t {}", payload);
    if payload.flag == network::FLAG_END {
        return Ok(payload);
    }

    send_ack(stream, payload.id);
    Ok(payload)
}

fn wait_ack(stream: &mut TcpStream, id: u16) -> Result<Payload, NetworkError> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    let bytes_read = stream.read(&mut buf).map_err(|e| {
        NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Timeout error: {}", e),
        )
    })?;
    
    let payload = match Payload::from_bytes(&buf[..bytes_read]) {
        Ok(p) => p,
        Err(e) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ProtocolError,
                &format!("Failed to parse payload: {}", e),
            ));
        }
    };

    check_received_rst(&payload)?;

    if payload.flag != network::FLAG_ACK {
        return Err(NetworkError::new(
            NetworkErrorKind::UnexpectedFlagError,
            "Received unexpected flag",
        ));
    }

    if payload.id != id {
        return Err(NetworkError::new(
            NetworkErrorKind::InvalidIdError,
            &format!("Received ACK with invalid ID: {}", payload.id),
        ));
    }

    println!("RECV ACK {}", payload);
    Ok(payload)
}

fn send_ack(stream: &mut TcpStream, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_ACK);
    println!("SEND ACK {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send ACK");
    });
}

pub fn send_rst(stream: &mut TcpStream, data: Option<Vec<u8>>) {
    let payload = Payload::new(data.unwrap_or_default(), u16::MAX, network::FLAG_RST);
    println!("SEND RST {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send RST");
    });
}

pub fn send_end(stream: &mut TcpStream, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_END);
    println!("SEND END {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send END");
    });
}


pub fn send_frame_without_ack(stream: &mut TcpStream, payload: &Payload) -> Result<usize, NetworkError> {
    if let Err(e) = stream.write_all(&payload.as_bytes()) {
        return Err(NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Failed to send frame: {}", e),
        ));
    }

    println!("SEND     {payload}");
    Ok(0)
}

pub fn receive_frame_without_ack(stream: &mut TcpStream) -> Result<Payload, NetworkError> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    let bytes_read = stream.read(&mut buf).map_err(|e| {
        NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Timeout error: {}", e),
        )
    })?;

    let payload = match Payload::from_bytes(&buf[..bytes_read]) {
        Ok(p) => p,
        Err(e) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ProtocolError,
                &format!("Failed to parse payload: {}", e),
            ));
        }
    };
    
    check_received_rst(&payload)?;

    if payload.flag == network::FLAG_ACK {
        return Err(NetworkError::new(
            NetworkErrorKind::UnexpectedFlagError,
            "Received ACK instead of data",
        ));
    }
    
    println!("RECV \t {}", payload);
    if payload.flag == network::FLAG_END {
        return Ok(payload);
    }

    Ok(payload)
}
