use std::fs::File;
use std::io::{BufReader, BufWriter, Error, ErrorKind};
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::dccnet::sync_read;
use crate::dccnet::xfer::{handle_client_receive, handle_client_send};

pub async fn run_client<A: tokio::net::ToSocketAddrs>(
    addr: A,
    mut input: BufReader<File>,
    mut output: BufWriter<File>,
) -> std::io::Result<()> {
    println!("Connecting to server");

    use tokio::time::{Duration, timeout};

    let stream = match timeout(Duration::from_secs(3), TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            return Err(e);
        }
        Err(e) => {
            return Err(Error::new(ErrorKind::TimedOut, e));
        }
    };

    let (reader_half, writer_half) = stream.into_split();

    let reader_half_mutex =  Arc::new(Mutex::new(reader_half));
    let writer_half_mutex = Arc::new(Mutex::new(writer_half));

    let future_send = handle_client_send( &writer_half_mutex, &mut input);
    let future_receive = handle_client_receive(&writer_half_mutex, &mut output);
    
    sync_read::read_stream_data_loop(reader_half_mutex).await;
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
