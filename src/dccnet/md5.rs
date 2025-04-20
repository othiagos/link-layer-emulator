use std::{io::Error, net::TcpStream};

use super::{communication, network, network::Payload};

fn validate_gas(
    stream_connection: &mut TcpStream,
    mut gas: Vec<u8>,
) -> Result<u16, std::io::Error> {
    gas.push(b'\n');

    let gas_payload = Payload::new(gas, network::START_ID, network::FLAG_SED);

    communication::send_frame(stream_connection, &gas_payload)?;
    let mut id = communication::next_id(gas_payload.id);

    let payload = communication::receive_frame(stream_connection)?;

    if gas_payload.id != payload.id {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Received ID is incorrect",
        ));
    }

    let payload_data = trim_data_payload(payload.data.clone())?;
    let md5_hash = md5::compute(payload_data);
    let rash_string = format!("{:x}\n", md5_hash);

    let send_payload = Payload::new(rash_string.as_bytes().to_vec(), id, network::FLAG_SED);
    communication::send_frame(stream_connection, &send_payload)?;
    id = communication::next_id(payload.id);

    Ok(id)
}

#[inline(always)]
fn trim_data_payload(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    match String::from_utf8(data) {
        Ok(data) => Ok(data.trim().as_bytes().to_vec()),
        Err(e) => Err(Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

fn read_date_from_server(stream_connection: &mut TcpStream, gas: Vec<u8>) -> Result<(), Error> {
    let mut payload_data = vec![];
    let mut id = validate_gas(stream_connection, gas)?;

    loop {
        let payload = match communication::receive_frame(stream_connection) {
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

            communication::send_frame(stream_connection, &send_payload)?;

            id = communication::next_id(id);
        }
        id = communication::next_id(id);

        payload_data.clear();
    }

    Ok(())
}

pub fn handle_tcp_communication(
    stream_connection: &mut TcpStream,
    gas: Vec<u8>,
) -> Result<(), Error> {
    read_date_from_server(stream_connection, gas)?;
    Ok(())
}
