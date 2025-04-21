use std::fs::File;
use std::io::{BufReader, BufWriter, Error};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::dccnet::communication::send_rst;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub fn run_server(port: u16, input: BufReader<File>, output: BufWriter<File>) {
    let listener =
        TcpListener::bind(format!("[::]:{}", port)).expect("Failed to bind to address");
    println!("Server listening on [::]:{}", port);

    let input = Arc::new(Mutex::new(input));
    let output = Arc::new(Mutex::new(output));

    for stream in listener.incoming() {
        handle_incoming_stream(stream, Arc::clone(&input), Arc::clone(&output));
    }
}

fn handle_incoming_stream(
    stream: Result<TcpStream, Error>,
    input: Arc<Mutex<BufReader<File>>>,
    output: Arc<Mutex<BufWriter<File>>>,
) {
    match stream {
        Ok(stream) => handle_incoming_connection(stream, input, output),
        Err(e) => eprintln!("Connection failed: {}", e),
    }
}

fn handle_incoming_connection(
    mut stream: TcpStream,
    input: Arc<Mutex<BufReader<File>>>,
    output: Arc<Mutex<BufWriter<File>>>,
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap_or_else(|e| {
            eprintln!("Failed to set read timeout: {}", e);
        });

    let peer_addr = stream.peer_addr().unwrap();
    println!("New connection from {}", peer_addr);

    let mut stream_clone = match stream.try_clone() {
        Ok(clone) => clone,
        Err(e) => {
            eprintln!("Failed to clone stream: {}", e);
            return;
        }
    };

    let receive_thread = thread::spawn(move || {
        if let Err(e) = handle_client_receive(&mut stream_clone, output) {
            eprintln!("Error handling client: {}", e);
            send_rst(&mut stream_clone, None);
        }
    });

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

    println!("Connection closed with {}", peer_addr);
}
