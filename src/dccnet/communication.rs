use std::fmt;

use crate::dccnet::sync_read;

use super::network;
use super::network::Payload;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex};

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum NetworkErrorKind {
    ConnectionError,
    ProtocolError,
    RSTError,
    UnexpectedFlagError,
    RetransmissionError,
    InvalidIdError,
    TimeoutError,
    ConnectionClosed,
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
        let payload_msg = String::from_utf8(payload.data.clone())
            .unwrap_or_else(|_| String::from("Invalid UTF-8 sequence"));

        return Err(NetworkError::new(
            NetworkErrorKind::RSTError,
            &format!("Received RST: {}", payload_msg),
        ));
    }

    Ok(())
}

pub async fn send_frame(
    stream_white: &Mutex<OwnedWriteHalf>,
    payload: &Payload,
) -> Result<usize, NetworkError> {
    for curr_attempt in 0..network::MAX_SEND_ATTEMPTS {
        if let Err(e) = stream_white
            .lock()
            .await
            .write_all(&payload.as_bytes())
            .await
        {
            return Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                &format!("Failed to send frame: {}", e),
            ));
        }
        println!("SEND     {payload}");

        match wait_ack(payload.id).await {
            Ok(_) => {
                if curr_attempt > 0 {
                    println!("SUCCESS RETRANSMISSION");
                }
                return Ok(curr_attempt);
            }
            Err(e) => {
                if e.kind == NetworkErrorKind::RSTError {
                    return Err(e);
                }

                if e.kind == NetworkErrorKind::ConnectionError {
                    return Err(e);
                }
            }
        }
        println!("({curr_attempt}) RETRANSMISSION");
    }

    Err(NetworkError::new(
        NetworkErrorKind::RetransmissionError,
        "Failed to send frame after maximum attempts",
    ))
}

pub async fn receive_frame(stream_white: &Mutex<OwnedWriteHalf>) -> Result<Payload, NetworkError> {
    let payload = sync_read::read_stream_data().await?;

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

    send_ack(stream_white, payload.id).await;
    Ok(payload)
}

async fn wait_ack(id: u16) -> Result<Payload, NetworkError> {
    let payload = sync_read::read_stream_ack().await?;

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

async fn send_ack(stream_white: &Mutex<OwnedWriteHalf>, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_ACK);
    println!("SEND ACK {payload}");

    if let Err(e) = stream_white
        .lock()
        .await
        .write_all(&payload.as_bytes())
        .await
    {
        eprintln!("Failed to send ACK: {}", e);
    }
}

pub async fn send_rst(stream_white: &Mutex<OwnedWriteHalf>, data: Option<Vec<u8>>) {
    let payload = Payload::new(data.unwrap_or_default(), u16::MAX, network::FLAG_RST);
    println!("SEND RST {payload}");

    if let Err(e) = stream_white
        .lock()
        .await
        .write_all(&payload.as_bytes())
        .await
    {
        eprintln!("Failed to send RST: {}", e);
    }
}

pub async fn send_end(stream_white: &Mutex<OwnedWriteHalf>, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_END);
    println!("SEND END {payload}");

    if let Err(e) = stream_white
        .lock()
        .await
        .write_all(&payload.as_bytes())
        .await
    {
        eprintln!("Failed to send END: {}", e);
    }
}
