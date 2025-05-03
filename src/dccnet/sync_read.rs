use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::Mutex};

use super::{
    communication::{NetworkError, NetworkErrorKind},
    network::{self, Payload},
};

pub static LAST_RECV_ACK: Lazy<Arc<Mutex<Option<Payload>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
pub static LAST_RECV_DATA: Lazy<Arc<Mutex<Option<Payload>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

async fn read_stream(stream_read: &Mutex<OwnedReadHalf>) -> Result<(), NetworkError> {

    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = match stream_read.lock().await.read(&mut buf).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Timeout error: {}", e),
        )),
    }?;

    let payload = match Payload::from_bytes(&buf[..bytes_read]) {
        Ok(p) => p,
        Err(e) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ProtocolError,
                &format!("Failed to parse payload: {}", e),
            ));
        }
    };

    if payload.flag == network::FLAG_ACK {
        let mut ack_guard = LAST_RECV_ACK.lock().await;
        if ack_guard.is_none() {
            *ack_guard = Some(payload);
        }
    } else if payload.flag != network::FLAG_ACK {
        let mut ack_guard = LAST_RECV_DATA.lock().await;
        if ack_guard.is_none() {
            *ack_guard = Some(payload);
        }
    }

    Ok(())
}

pub async fn read_stream_ack(stream_read: &Mutex<OwnedReadHalf>) -> Result<Payload, NetworkError> {
    read_stream(stream_read).await?;

    let payload = match LAST_RECV_ACK.lock().await.take() {
        Some(p) => p,
        None => {
            return Err(NetworkError::new(
                NetworkErrorKind::UnexpectedFlagError,
                "No ACK payload available",
            ));
        }
    };

    Ok(payload)
}

pub async fn read_stream_data(stream_read: &Mutex<OwnedReadHalf>) -> Result<Payload, NetworkError> {
    read_stream(stream_read).await?;

    let payload = match LAST_RECV_DATA.lock().await.take() {
        Some(p) => p,
        None => {
            return Err(NetworkError::new(
                NetworkErrorKind::UnexpectedFlagError,
                "No DATA payload available",
            ));
        }
    };

    Ok(payload)
}
