use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;

use crate::dccnet::communication::send_rst;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub fn run_client<A: ToSocketAddrs>(
    addr: A,
    input: BufReader<File>,
    output: BufWriter<File>,
) -> std::io::Result<()> {
    println!("Connecting to server");
    let mut stream = TcpStream::connect(addr).expect("Could not connect to server");

    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap_or_else(|e| {
            eprintln!("Failed to set read timeout: {}", e);
        });

    let mut stream_clone = match stream.try_clone() {
        Ok(clone) => clone,
        Err(e) => {
            eprintln!("Failed to clone stream: {}", e);
            return Err(e);
        }
    };

    let output = std::sync::Arc::new(std::sync::Mutex::new(output));
    let receive_thread = thread::spawn(move || {
        if let Err(e) = handle_client_receive(&mut stream_clone, output) {
            eprintln!("Error handling client: {}", e);
            send_rst(&mut stream_clone, None);
        }
    });

    let input = std::sync::Arc::new(std::sync::Mutex::new(input));
    let send_thread = thread::spawn(move || {
        if let Err(e) = handle_client_send(&mut stream, input) {
            eprintln!("Error handling client: {}", e);
            send_rst(&mut stream, None);
        }
    });

    receive_thread.join().unwrap_or_else(|e| {
        eprintln!("Receive thread panicked: {:?}", e);
    });

    send_thread.join().unwrap_or_else(|e| {
        eprintln!("Send thread panicked: {:?}", e);
    });

    Ok(())
}
