use crate_net::dccnet;
use tokio::net::TcpStream;

use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let server: Vec<&str> = args.get(1).unwrap().split(":").collect();
    let gas = args.get(2).unwrap().as_bytes().to_vec();

    if server.len() < 2 {
        eprintln!("Invalid server: use ':' to separate the server address from the server port");
        std::process::exit(1);
    }

    let server_address = *server.first().unwrap();
    let server_port = *server.get(1).unwrap();
    let server_port = match server_port.parse::<u16>() {
        Ok(port) => port,
        Err(e) => {
            eprintln!("Invalid port number: {:?}", e.to_string());
            std::process::exit(1);
        }
    };

    let communication_result = match TcpStream::connect((server_address, server_port)).await {
        Ok(stream) => dccnet::md5::handle_tcp_communication(stream, gas).await,
        Err(e) => {
            eprintln!("Failed to connect to the server: {:?}", e.to_string());
            std::process::exit(1);
        }
    };

    if let Err(e) = communication_result {
        eprintln!("Error during TCP communication: {:?}", e.to_string());
        std::process::exit(1);
    }
}
