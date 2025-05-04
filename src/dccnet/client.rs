use std::fs::File;
use std::io::{BufReader, BufWriter, Error, ErrorKind};

use tokio::net::TcpStream;

use crate::dccnet::xfer;

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

    xfer::handle_connection(stream, &mut input, &mut output).await;

    println!("End connection with server!");
    Ok(())
}
