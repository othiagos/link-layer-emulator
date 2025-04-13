use std::{
    io::{Error, Read, Write},
    net::TcpStream,
    thread,
    time::Duration,
};

use super::network;
use super::network::Payload;
use crate::dccnet::validation;

pub fn next_id(id: u16) -> u16 {
    (id + 1) % 2
}

pub fn send_frame(stream: &mut TcpStream, payload: &Payload) -> Result<usize, Error> {
    for curr_attempt in 0..network::MAX_SEND_ATTEMPTS {
        println!("SEND {payload}");
        stream.write_all(&payload.as_bytes())?;

        if wait_ack(stream, payload.id).is_ok() {
            return Ok(curr_attempt);
        }

        print!("({}) RETRANSMISSION ", curr_attempt);

        thread::sleep(Duration::from_millis(200));
    }

    Err(Error::new(
        std::io::ErrorKind::Other,
        "Failed to send frame after maximum attempts",
    ))
}

pub fn receive_frame(stream: &mut TcpStream) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = stream.read(&mut buf).unwrap();

    let payload = network::Payload::from_bytes(&buf[..bytes_read])?;
    println!("RECEIVED {}", payload);

    validation::check_received_rst(&payload);

    send_ack(stream, payload.id);
    Ok(payload)
}

fn wait_ack(stream: &mut TcpStream, id: u16) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = stream.read(&mut buf).unwrap();
    let payload = network::Payload::from_bytes(&buf[..bytes_read])?;
    println!("RECEIVED ACK {payload}");
    validation::check_received_rst(&payload);

    if payload.flag != network::FLAG_ACK {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "ACK not received",
        ));
    }

    if payload.id != id {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "ID is incorrect",
        ));
    }

    Ok(payload)
}

fn send_ack(stream: &mut TcpStream, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_ACK);
    println!("SEND ACK {payload}");

    stream.write_all(&payload.as_bytes()).unwrap();
}
