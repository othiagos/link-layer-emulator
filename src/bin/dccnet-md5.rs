use crate_net::dccnet;
use std::net::TcpStream;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let server: Vec<&str> = args.get(1).unwrap().split(":").collect();
    let mut gas = args.get(2).unwrap().as_bytes().to_vec();
    gas.push(10);

    let server_address = *server.first().unwrap();
    let port = *server.get(1).unwrap();
    let port = match port.parse::<u16>() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Invalid port number: {:?}", e.to_string());
            std::process::exit(1);
        }
    };

    match TcpStream::connect((server_address, port)) {
        Ok(stream) => dccnet::md5::handle_tpc_communication(stream),
        Err(e) => {
            eprintln!("Failed to connect to the server: {:?}", e.to_string());
            std::process::exit(1);
        }
    };
}
