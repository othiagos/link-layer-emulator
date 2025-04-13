use std::{io::Error, net::TcpStream};

use super::{communication, network, network::Payload};

fn validate_gas(
    stream_connection: &mut TcpStream,
    mut gas: Vec<u8>,
) -> Result<Payload, std::io::Error> {
    gas.push(b'\n');

    let gas_payload = Payload::new(gas, network::START_ID, network::FLAG_SED);

    let mut res_data = vec![];
    communication::send_frame(stream_connection, &gas_payload, &mut res_data)?;
    let payload = communication::receive_frame(stream_connection)?;

    if gas_payload.id != payload.id {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Received ID is incorrect",
        ));
    }

    Ok(payload)
}

fn trim_data_payload(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    match String::from_utf8(data) {
        Ok(data) => Ok(data.trim().as_bytes().to_vec()),
        Err(e) => Err(Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

fn read_date_from_server(stream_connection: &mut TcpStream, payload: Payload) -> Result<(), Error> {
    let mut end_communication = false;
    let mut id = payload.id;

    let mut payload_data = payload.data;

    while !end_communication {
        id = communication::next_id(id);
        payload_data = trim_data_payload(payload_data)?;

        println!("MSG: {:?}", String::from_utf8(payload_data.clone()).unwrap());
        let md5_hash = md5::compute(payload_data);
        let rash_string = format!("{:x}\n", md5_hash);

        let send_payload = Payload::new(rash_string.as_bytes().to_vec(), id, network::FLAG_SED);

        let mut res_data = vec![];
        communication::send_frame(stream_connection, &send_payload, &mut res_data)?;

        if res_data.is_empty() {
            let payload = communication::receive_frame(stream_connection)?;
            if send_payload.id != payload.id {
                return Err(Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Received ID is incorrect",
                ));
            }
            payload_data = payload.data;
        } else {
            payload_data = res_data;
        }

        if payload.flag == network::FLAG_END {
            end_communication = true;
        }
    }

    Ok(())
}

pub fn handle_tcp_communication(stream_connection: &mut TcpStream, gas: Vec<u8>) -> Result<(), Error> {
    let payload = validate_gas(stream_connection, gas)?;
    read_date_from_server(stream_connection, payload)?;
    Ok(())
}
