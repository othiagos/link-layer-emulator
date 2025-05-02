use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::Mutex};
use once_cell::sync::Lazy;

use super::{communication::{NetworkError, NetworkErrorKind}, network::{self, Payload}};

pub static LAST_RECV_ACK: Lazy<Mutex<Option<Payload>>> = Lazy::new(|| Mutex::new(None));
pub static LAST_RECV_DATA: Lazy<Mutex<Option<Payload>>> = Lazy::new(|| Mutex::new(None));

pub async fn read_stream_ack(stream_read: &Mutex<OwnedReadHalf>,) -> Result<Payload, NetworkError> {
    let ack = {
        let mut last_ack = LAST_RECV_ACK.lock().await;
        last_ack.take()
    };

    if let Some(ack) = ack {
        println!("USED LAST ACK");
        *LAST_RECV_ACK.lock().await = None;
        return Ok(ack);
    }

    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read =  match stream_read.lock().await.read(&mut buf).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                &format!("Timeout error: {}", e),
            ))
        }
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

    if payload.flag != network::FLAG_ACK && payload.flag != network::FLAG_RST {
        *LAST_RECV_DATA.lock().await = Some(payload);

        return Err(NetworkError::new(
            NetworkErrorKind::UnexpectedFlagError,
            "Received unexpected flag",
        ));
    }

    Ok(payload)
}

pub async fn read_stream_data(stream_read: &Mutex<OwnedReadHalf>) -> Result<Payload, NetworkError> {
    let data = {
        let mut last_data = LAST_RECV_DATA.lock().await;
        last_data.take()
    };

    if let Some(data) = data {
        println!("USED FRAME DATA");
        *LAST_RECV_DATA.lock().await = None;
        return Ok(data);
    }

    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = match stream_read.lock().await.read(&mut buf).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                &format!("Timeout error: {}", e),
            ))
        }
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
        *LAST_RECV_ACK.lock().await = Some(payload);

        return Err(NetworkError::new(
            NetworkErrorKind::UnexpectedFlagError,
            "Received unexpected flag",
        ));
    }

    Ok(payload)
}