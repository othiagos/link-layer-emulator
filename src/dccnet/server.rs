use std::fs::File;
use std::io::{BufReader, BufWriter, Error};

use tokio::net::{TcpListener, TcpStream};

use crate::dccnet::xfer;

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

    xfer::handle_connection(stream, input, output).await;

    println!("Connection closed with {}", peer_addr);
}
