use std::fs::File;
use std::io::{BufReader, BufWriter};

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_client<A: tokio::net::ToSocketAddrs>(
    addr: A,
    mut input: BufReader<File>,
    mut output: BufWriter<File>,
) -> std::io::Result<()> {
    println!("Connecting to server");

    use tokio::time::{Duration, timeout};

    let stream = timeout(Duration::from_secs(3), TcpStream::connect(addr))
        .await
        .expect("Timeout while connecting to server")
        .expect("Could not connect to server");

    let (reader_half, writer_half) = stream.into_split();

    let reader_half_mutex = Mutex::new(reader_half);
    let writer_half_mutex = std::sync::Arc::new(Mutex::new(writer_half));

    let future_send = handle_client_send(&reader_half_mutex, &writer_half_mutex, &mut input);
    let future_receive = handle_client_receive(&reader_half_mutex, &writer_half_mutex, &mut output);

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
