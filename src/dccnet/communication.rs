use std::{
    io::{Error, Read, Write},
    net::TcpStream,
    thread,
    time::Duration,
};

use super::network::Payload;
use super::network;

pub fn next_id(id: u16) -> u16 {
    if id > 0 { 0 } else { 1 }
}

pub fn send_frame(stream: &mut TcpStream, payload: Payload) -> Result<(), Error> {
    println!("SEND {payload}");

    for i in 0..network::MAX_SEND_ATTEMPTS {
        let _ = stream.write(&payload.as_bytes())?;
        if wait_ack(stream, payload.id).is_ok() {
            return Ok(());
        }

        println!("({}, 16)", i);

        thread::sleep(Duration::new(1, 0));
    }

    Err(Error::new(
        std::io::ErrorKind::Other,
        "Failed to send frame after maximum attempts",
    ))
}

pub fn receive_frame(stream: &mut TcpStream) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let _res_read = stream.read(&mut buf).unwrap();

    let payload = network::Payload::from_bytes(&buf).unwrap();
    println!("RECEIVED {}", payload);

    send_ack(stream, payload.id);
    Ok(payload)
}

fn wait_ack(stream: &mut TcpStream, id: u16) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];
    let bytes_read = stream.read(&mut buf)?;

    let payload = network::Payload::from_bytes(&buf[..bytes_read]).unwrap();
    println!("RECEIVED {payload}");

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
    println!("SEND {payload}");

    let _ = stream.write(&payload.as_bytes());
}
