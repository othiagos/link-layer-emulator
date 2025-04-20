use std::{
    io::{Error, Read, Write},
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

use super::network;
use super::network::Payload;

#[inline(always)]
pub fn next_id(id: u16) -> u16 {
    (id + 1) % 2
}

fn check_received_rst(payload: &Payload) {
    if payload.flag == network::FLAG_RST && payload.id == u16::MAX {
        let payload_msg = String::from_utf8(payload.data.clone()).unwrap();
        panic!("Invalid Frame: {}", payload_msg);
    }
}

pub fn send_frame(stream: &mut TcpStream, payload: &Payload) -> Result<usize, Error> {
    const RETRANSMISSION_DELAY: u64 = 20;

    for curr_attempt in 0..network::MAX_SEND_ATTEMPTS {
        println!("SEND     {payload}");
        stream.write_all(&payload.as_bytes())?;
        let start = Instant::now();

        if wait_ack(stream, payload.id).is_ok() {
            if curr_attempt > 0 {
                println!("SUCCESS RETRANSMISSION");
                let time = start.elapsed().as_millis() as u64;

                if time < RETRANSMISSION_DELAY {
                    thread::sleep(Duration::from_millis(RETRANSMISSION_DELAY - time));
                }
            }
            return Ok(curr_attempt);
        }

        let time = start.elapsed().as_millis() as u64;

        println!("({curr_attempt}) RETRANSMISSION RECEIVE TIME {time}ms");

        if time < RETRANSMISSION_DELAY {
            println!("WAIT {}ms", RETRANSMISSION_DELAY - time);
            thread::sleep(Duration::from_millis(RETRANSMISSION_DELAY - time));
        }
    }

    Err(Error::new(
        std::io::ErrorKind::Other,
        "Failed to send frame after maximum attempts",
    ))
}

pub fn receive_frame(stream: &mut TcpStream) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    match stream.read(&mut buf) {
        Ok(bytes_read) => {
            let payload = Payload::from_bytes(&buf[..bytes_read])?;
            println!("RECV \t {}", payload);

            check_received_rst(&payload);

            if payload.flag == network::FLAG_END {
                return Ok(payload);
            }

            send_ack(stream, payload.id);
            Ok(payload)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            Err(Error::new(std::io::ErrorKind::TimedOut, "Timeout error"))
        }
        Err(e) => Err(e),
    }
}

fn wait_ack(stream: &mut TcpStream, id: u16) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    let bytes_read = stream.read(&mut buf).unwrap();
    let payload = network::Payload::from_bytes(&buf[..bytes_read])?;
    check_received_rst(&payload);

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

    println!("RECV ACK {}", payload);
    Ok(payload)
}

fn send_ack(stream: &mut TcpStream, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_ACK);
    println!("SEND ACK {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send ACK");
    });
}

pub fn send_rst(stream: &mut TcpStream, data: Option<Vec<u8>>) {
    let payload = Payload::new(data.unwrap_or_default(), u16::MAX, network::FLAG_RST);
    println!("SEND RST {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send RST");
    });
}

pub fn send_end(stream: &mut TcpStream, id: u16) {
    let payload = Payload::new(vec![], id, network::FLAG_END);
    println!("SEND END {payload}");

    stream.write_all(&payload.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Failed to send END");
    });
}

pub fn _send_frame_tcp(stream: &mut TcpStream, payload: &Payload) -> Result<usize, Error> {
    println!("SEND     {payload}");
    stream.write_all(&payload.as_bytes())?;

    Ok(0)
}

pub fn _receive_frame_tcp(stream: &mut TcpStream) -> Result<Payload, Error> {
    let mut buf = vec![0u8; network::MAX_PAYLOAD_SIZE];

    match stream.read(&mut buf) {
        Ok(bytes_read) => {
            let payload = Payload::from_bytes(&buf[..bytes_read])?;
            println!("RECV \t {}", payload);

            check_received_rst(&payload);

            if payload.flag == network::FLAG_END {
                return Ok(payload);
            }

            Ok(payload)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            Err(Error::new(std::io::ErrorKind::TimedOut, "Timeout error"))
        }
        Err(e) => Err(e),
    }
}
