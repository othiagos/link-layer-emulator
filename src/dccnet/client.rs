use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_client<A: ToSocketAddrs>(
    addr: A,
    input: BufReader<File>,
    output: BufWriter<File>,
) -> std::io::Result<()> {
    println!("Connecting to server");
    let stream = TcpStream::connect(addr).expect("Could not connect to server");

    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap_or_else(|e| {
            eprintln!("Failed to set read timeout: {}", e);
        });

    let output = Arc::new(Mutex::new(output));
    let input = Arc::new(Mutex::new(input));

    let mut reader = stream;
    let mut writer = reader.try_clone().unwrap();

    let future_send = handle_client_send(&mut writer, input);
    let future_receive = handle_client_receive(&mut reader, output);

    let (result_send, result_receive) = tokio::join!(future_send, future_receive);

    result_send.unwrap_or_else(|e| {
        eprintln!("Error receiving data: {}", e);
    });

    result_receive.unwrap_or_else(|e| {
        eprintln!("Error sending data: {}", e);
    });

    println!("End connection with server!");
    Ok(())
}
