use std::fs::File;
use std::io::{BufReader, BufWriter, Error};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::dccnet::communication;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_server(port: u16, input: BufReader<File>, output: BufWriter<File>) {
    let listener = TcpListener::bind(format!("[::]:{}", port)).expect("Failed to bind to address");
    println!("Server listening on [::]:{}", port);

    let input = Arc::new(Mutex::new(input));
    let output = Arc::new(Mutex::new(output));

    for stream in listener.incoming() {
        handle_incoming_stream(stream, Arc::clone(&input), Arc::clone(&output)).await;
    }
}

async fn handle_incoming_stream(
    stream: Result<TcpStream, Error>,
    input: Arc<Mutex<BufReader<File>>>,
    output: Arc<Mutex<BufWriter<File>>>,
) {
    match stream {
        Ok(stream) => {
            let input = Arc::clone(&input);
            let output = Arc::clone(&output);
            handle_incoming_connection(stream, input, output).await;
        }
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

async fn handle_incoming_connection(
    stream: TcpStream,
    input: Arc<Mutex<BufReader<File>>>,
    output: Arc<Mutex<BufWriter<File>>>,
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap_or_else(|e| {
            eprintln!("Failed to set read timeout: {}", e);
        });

    let peer_addr = stream.peer_addr().unwrap();
    println!("New connection from {}", peer_addr);

    let mut reader = stream;
    let mut writer = reader.try_clone().unwrap();

    let future_receive = handle_client_receive(&mut reader, output);
    let future_send = handle_client_send(&mut writer, input);

    let (result_receive, result_send) = tokio::join!(future_receive, future_send);

    result_receive.unwrap_or_else(|e| {
        let error_message = e.to_string();
        tokio::spawn(async move {
            communication::send_rst(&mut reader, Some(error_message.as_bytes().to_vec())).await;
        });
        eprintln!("Error sending data: {}", e);
    });

    result_send.unwrap_or_else(|e| {
        let error_message = e.to_string();
        tokio::spawn(async move {
            communication::send_rst(&mut writer, Some(error_message.as_bytes().to_vec())).await;
        });
        eprintln!("Error receiving data: {}", e);
    });

    println!("Connection closed with {}", peer_addr);
}
