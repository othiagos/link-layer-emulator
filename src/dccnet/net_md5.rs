use std::{io::Error, net::TcpStream};

use super::{communication, network, network::Payload};

fn validate_gas(
    stream_connection: &mut TcpStream,
    mut gas: Vec<u8>,
) -> Result<u16, std::io::Error> {
    gas.push(b'\n');

    let gas_payload = Payload::new(gas, network::START_ID, network::FLAG_SED);

    let mut res_data = vec![];
    communication::send_frame(stream_connection, &gas_payload, &mut res_data)?;
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
    let mut res_data = vec![];
    communication::send_frame(stream_connection, &send_payload, &mut res_data)?;
    id = communication::next_id(payload.id);

    Ok(id)
}

fn trim_data_payload(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    match String::from_utf8(data) {
        Ok(data) => Ok(data.trim().as_bytes().to_vec()),
        Err(e) => Err(Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

fn read_date_from_server(stream_connection: &mut TcpStream, mut id: u16) -> Result<(), Error> {
    let mut end_communication = false;
    let mut payload_data = vec![];
    let mut payload = Payload::new(vec![], id, network::FLAG_SED);

    while !end_communication {

        // if payload_data.is_empty() {
        //     payload = communication::receive_frame(stream_connection)?;
        //     payload_data = payload.data;
        // }
        payload = communication::receive_frame(stream_connection)?;
        payload_data = payload.data;
        
        id = communication::next_id(id);
        payload_data = trim_data_payload(payload_data)?;

        println!("MSG: {:?}", String::from_utf8(payload_data.clone()).unwrap());
        let md5_hash = md5::compute(payload_data);
        let rash_string = format!("{:x}\n", md5_hash);

        let send_payload = Payload::new(rash_string.as_bytes().to_vec(), id, network::FLAG_SED);

        let mut res_data = vec![];
        communication::send_frame(stream_connection, &send_payload, &mut res_data)?;
        // payload_data = res_data;
        

        if payload.flag == network::FLAG_END {
            end_communication = true;
        }
    }

    Ok(())
}

pub fn handle_tcp_communication(stream_connection: &mut TcpStream, gas: Vec<u8>) -> Result<(), Error> {
    let id = validate_gas(stream_connection, gas)?;
    read_date_from_server(stream_connection, id)?;
    
    Ok(())
}
