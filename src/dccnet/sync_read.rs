use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::time::{Duration, timeout};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::Mutex};

use super::{
    communication::{NetworkError, NetworkErrorKind},
    network::{self, Payload},
};

pub static LAST_RECV_ACK: Lazy<Arc<Mutex<Option<Payload>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));
pub static LAST_RECV_DATA: Lazy<Arc<Mutex<Option<Payload>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

async fn read_next_payload(stream_read: &Mutex<OwnedReadHalf>) -> Result<Payload, NetworkError> {
    // if LAST_RECV_ACK.lock().await.is_some() {
    //     println!("HAS ACK");
    // }

    // if LAST_RECV_DATA.lock().await.is_some() {
    //     println!("HAS DATA");
    // }

    // println!("READ RUNNING ...");
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = match timeout(
        Duration::from_secs(3),
        stream_read.lock().await.read(&mut buf),
    )
    .await
    {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(e)) => Err(NetworkError::new(
            NetworkErrorKind::ConnectionError,
            &format!("Read error: {}", e),
        )),
        Err(_) => Err(NetworkError::new(
            NetworkErrorKind::ConnectionError,
            "Read operation timed out",
        )),
    }?;
    // println!("END READ RUNNING ...");

    let payload = match Payload::from_bytes(&buf[..bytes_read]) {
        Ok(p) => p,
        Err(e) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ProtocolError,
                &format!("Failed to parse payload: {}", e),
            ));
        }
    };

    Ok(payload)
}

pub async fn read_stream_data_loop(stream_read: Arc<Mutex<OwnedReadHalf>>) {
    tokio::spawn(async move {
        loop {
            if let Ok(payload) = read_next_payload(&stream_read).await {
                match payload.flag {
                    network::FLAG_ACK => {
                        let mut last_ack = LAST_RECV_ACK.lock().await;
                        if last_ack.is_none() {
                            *last_ack = Some(payload);
                        }
                    }
                    _ => {
                        let mut last_data = LAST_RECV_DATA.lock().await;
                        if last_data.is_none() {
                            *last_data = Some(payload);
                        }
                    }
                }
            }
        }
    });
}

pub async fn read_stream_ack() -> Result<Payload, NetworkError> {
    let mut elapsed = 0;
    let interval = Duration::from_millis(100); // Check every 100ms
    while elapsed < 3000 {
        if let Some(payload) = LAST_RECV_ACK.lock().await.take() {
            return Ok(payload);
        }
        tokio::time::sleep(interval).await;
        elapsed += interval.as_millis() as u64;
    }

    Err(NetworkError::new(
        NetworkErrorKind::UnexpectedFlagError,
        "No ACK payload available after waiting for 3 seconds",
    ))
}

pub async fn read_stream_data() -> Result<Payload, NetworkError> {
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
