use std::fs::File;
use std::io::{BufReader, BufWriter, Error};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::dccnet::communication;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_server(port: u16, mut input: BufReader<File>, mut output: BufWriter<File>) {
    let listener = TcpListener::bind(format!("[::]:{}", port))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port {}: {}", port, e);
            std::process::exit(1);
        });
    println!("Server listening on [::]:{}", port);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => handle_incoming_stream(Ok(stream), &mut input, &mut output).await,
            Err(e) => eprintln!("Failed to accept connection: {}", e),
        }
    }
}

async fn handle_incoming_stream(
    stream: Result<TcpStream, Error>,
    input: &mut BufReader<File>,
    output: &mut BufWriter<File>,
) {
    match stream {
        Ok(stream) => handle_incoming_connection(stream, input, output).await,
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

async fn handle_incoming_connection(
    stream: TcpStream,
    input: &mut BufReader<File>,
    output: &mut BufWriter<File>,
) {
    let peer_addr = stream.peer_addr().unwrap();
    println!("New connection from {}", peer_addr);

    let (reader_half, writer_half) = stream.into_split();

    let reader_half_mutex = Mutex::new(reader_half);
    let writer_half_mutex = std::sync::Arc::new(Mutex::new(writer_half));

    let future_receive = handle_client_receive(&reader_half_mutex, &writer_half_mutex, output);
    let future_send = handle_client_send(&reader_half_mutex, &writer_half_mutex, input);

    let (result_receive, result_send) = tokio::join!(future_receive, future_send);

    let writer_half_mutex_clone = writer_half_mutex.clone();
    result_receive.unwrap_or_else(|e| {
        let error_message = e.to_string();
        let writer_half_mutex_clone = writer_half_mutex_clone.clone();
        tokio::spawn(async move {
            communication::send_rst(
                &writer_half_mutex_clone,
                Some(error_message.as_bytes().to_vec()),
            )
            .await;
        });
        eprintln!("Error sending data: {}", e);
    });

    result_send.unwrap_or_else(|e| {
        let error_message = e.to_string();
        let writer_half_mutex_clone = writer_half_mutex.clone();
        tokio::spawn(async move {
            communication::send_rst(
                &writer_half_mutex_clone,
                Some(error_message.as_bytes().to_vec()),
            )
            .await;
        });
        eprintln!("Error receiving data: {}", e);
    });

    println!("Connection closed with {}", peer_addr);
}
