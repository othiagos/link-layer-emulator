use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::Mutex};

const WAIT_ACK_TIMEOUT: u64 = 1000;

use super::{
    communication::{NetworkError, NetworkErrorKind},
    network::{self, Payload},
};

static LAST_RECV_ACK: Lazy<Arc<Mutex<Option<Payload>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
static LAST_RECV_DATA: Lazy<Arc<Mutex<Option<Payload>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

async fn read_next_payload(stream_read: &Mutex<OwnedReadHalf>) -> Result<Payload, NetworkError> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    let read_result = timeout(Duration::from_secs(3), async {
        let mut stream = stream_read.lock().await;
        stream.read(&mut buf).await
    }).await;

    let bytes_read = match read_result {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                &format!("Read error: {}", e),
            ));
        }
        Err(_) => {
            return Err(NetworkError::new(
                NetworkErrorKind::ConnectionError,
                "Read operation timed out",
            ));
        }
    };

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
                handle_payload(payload).await;
            }
        }
    });
}

async fn handle_payload(payload: Payload) {
    match payload.flag {
        network::FLAG_ACK => store_ack_payload(payload).await,
        _ => store_data_payload(payload).await,
    }
}

async fn store_ack_payload(payload: Payload) {
    let mut last_ack = LAST_RECV_ACK.lock().await;
    if last_ack.is_none() {
        *last_ack = Some(payload);
    }
}

async fn store_data_payload(payload: Payload) {
    let mut last_data = LAST_RECV_DATA.lock().await;
    if last_data.is_none() {
        *last_data = Some(payload);
    }
}

pub async fn read_stream_ack() -> Result<Payload, NetworkError> {
    let mut elapsed = 0;
    let interval = Duration::from_millis(10);

    while elapsed < WAIT_ACK_TIMEOUT {
        if let Some(payload) = LAST_RECV_ACK.lock().await.take() {
            return Ok(payload);
        }
        sleep(interval).await;
        elapsed += interval.as_millis() as u64;
    }

    Err(NetworkError::new(
        NetworkErrorKind::Other,
        "No ACK payload available after waiting for 1 seconds",
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
