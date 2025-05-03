use std::fs::File;
use std::io::{BufReader, BufWriter, Error};
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::dccnet::sync_read;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_server(port: u16, mut input: BufReader<File>, mut output: BufWriter<File>) {
    let listener = TcpListener::bind(format!("[::]:{}", port))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port {}: {}", port, e);
            std::process::exit(1);
        });
    println!("Server listening on [::]:{}", port);

    
    match listener.accept().await {
        Ok((stream, _)) => handle_incoming_stream(Ok(stream), &mut input, &mut output).await,
        Err(e) => eprintln!("Failed to accept connection: {}", e),
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
    let peer_addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to get peer address: {}", e);
            return;
        }
    };
    println!("New connection from {}", peer_addr);

    let (reader_half, writer_half) = stream.into_split();
    let reader_half_mutex =  Arc::new(Mutex::new(reader_half));
    let writer_half_mutex =  Arc::new(Mutex::new(writer_half));
    
    let future_send = handle_client_send(&writer_half_mutex, input);
    let future_receive = handle_client_receive(&writer_half_mutex, output);
    
    sync_read::read_stream_data_loop(reader_half_mutex).await;
    let (result_send, result_receive) = tokio::join!(future_send, future_receive);
    
    result_send.unwrap_or_else(|e| {
        eprintln!("Error receiving data: {}", e);
    });

    result_receive.unwrap_or_else(|e| {
        eprintln!("Error sending data: {}", e);
    });

    println!("Connection closed with {}", peer_addr);
}
