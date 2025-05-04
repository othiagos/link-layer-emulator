use std::sync::Arc;

use tokio::{
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::Mutex,
};

use super::{
    communication::{self, NetworkError, NetworkErrorKind},
    network::{self, Payload},
    sync_read,
};

async fn validate_gas(
    stream_white: &Mutex<OwnedWriteHalf>,
    mut gas: Vec<u8>,
) -> Result<u16, NetworkError> {
    gas.push(b'\n');

    let gas_payload = Payload::new(gas, network::START_ID, network::FLAG_SED);

    communication::send_frame(stream_white, &gas_payload).await?;

    let mut id = communication::next_id(gas_payload.id);

    let payload = loop {
        match communication::receive_frame(stream_white).await {
            Ok(payload) => break payload,
            Err(_) => continue,
        }
    };

    if gas_payload.id != payload.id {
        return Err(NetworkError::new(
            NetworkErrorKind::InvalidIdError,
            "Received ID is incorrect",
        ));
    }

    let payload_data = trim_data_payload(payload.data.clone())?;
    let md5_hash = md5::compute(payload_data);
    let rash_string = format!("{:x}\n", md5_hash);

    let send_payload = Payload::new(rash_string.as_bytes().to_vec(), id, network::FLAG_SED);
    communication::send_frame(stream_white, &send_payload).await?;

    id = communication::next_id(payload.id);

    Ok(id)
}

#[inline(always)]
fn trim_data_payload(data: Vec<u8>) -> Result<Vec<u8>, NetworkError> {
    match String::from_utf8(data) {
        Ok(data) => Ok(data.trim().as_bytes().to_vec()),
        Err(e) => Err(NetworkError::new(
            NetworkErrorKind::Other,
            e.to_string().as_ref(),
        )),
    }
}

async fn read_date_from_server(stream: TcpStream, gas: Vec<u8>) -> Result<(), NetworkError> {
    let mut payload_data = vec![];

    let (read_half, write_half) = stream.into_split();
    let stream_read = Arc::new(Mutex::new(read_half));
    let stream_white = Arc::new(Mutex::new(write_half));

    sync_read::read_stream_data_loop(stream_read).await;
    let mut id = validate_gas(&stream_white, gas).await?;

    loop {
        let payload = match communication::receive_frame(&stream_white).await {
            Ok(payload) => payload,
            Err(_) => continue,
        };

        if payload.flag == network::FLAG_END {
            break;
        }

        if payload.flag == network::FLAG_ACK {
            continue;
        }

        if payload_data.is_empty() {
            id = communication::next_id(id);
        }

        if !payload.data.ends_with(b"\n") {
            payload_data.extend(payload.data);
            continue;
        }

        payload_data.extend(trim_data_payload(payload.data)?);
        for data in payload_data.split(|&x| x == b'\n') {
            let md5_hash = md5::compute(data);
            let rash_string = format!("{:x}\n", md5_hash);

            let send_payload = Payload::new(rash_string.as_bytes().to_vec(), id, network::FLAG_SED);

            communication::send_frame(&stream_white, &send_payload).await?;

            id = communication::next_id(id);
        }
        id = communication::next_id(id);

        payload_data.clear();
    }

    Ok(())
}

pub async fn handle_tcp_communication(stream: TcpStream, gas: Vec<u8>) -> Result<(), NetworkError> {
    read_date_from_server(stream, gas).await?;
    Ok(())
}
