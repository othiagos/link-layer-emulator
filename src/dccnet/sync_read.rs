use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::time::{Duration, sleep, timeout};
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
    })
    .await;

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
                NetworkErrorKind::TimeoutError,
                "Read operation timed out",
            ));
        }
    };

    if bytes_read == 0 {
        return Err(NetworkError::new(
            NetworkErrorKind::ConnectionClosed,
            "Connection closed by peer",
        ));
    }

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
            if let Err(should_break) = process_next_payload(&stream_read).await {
                if should_break {
                    break;
                }
            }
        }
    });
}

async fn process_next_payload(stream_read: &Arc<Mutex<OwnedReadHalf>>) -> Result<(), bool> {
    match read_next_payload(stream_read).await {
        Ok(payload) => {
            handle_payload(payload).await;
            Ok(())
        }
        Err(e) => handle_read_error(e).await,
    }
}

async fn handle_read_error(error: NetworkError) -> Result<(), bool> {
    match error.kind {
        NetworkErrorKind::TimeoutError => {
            println!("Timeout error: {}", error);
            end_connection().await;
            Err(true)
        }
        NetworkErrorKind::ConnectionClosed => {
            println!("Connection closed: {}", error);
            end_connection().await;
            Err(true)
        }
        _ => {
            println!("Unhandled error: {}", error);
            Err(false)
        }
    }
}

async fn end_connection() {
    let payload = Payload::new(vec![], 0, network::FLAG_END);

    store_ack_payload(payload.clone()).await;
    store_data_payload(payload).await;
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
