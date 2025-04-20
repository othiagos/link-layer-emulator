use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process;

use crate_net::dccnet::client;
use crate_net::dccnet::server;

fn parse_server_args(args: &[String]) {
    let port = args[2].parse::<u16>().unwrap_or_else(|_| {
        eprintln!("Invalid port: {}", args[2]);
        process::exit(1);
    });

    let input_path = PathBuf::from(&args[3]);
    let output_path = PathBuf::from(&args[4]);

    let input_file = File::open(&input_path).unwrap_or_else(|_| {
        eprintln!("Failed to open input file: {:?}", input_path);
        process::exit(1);
    });
    let output_file = File::create(&output_path).unwrap_or_else(|_| {
        eprintln!("Failed to create output file: {:?}", output_path);
        process::exit(1);
    });

    let input = BufReader::new(input_file);
    let output = BufWriter::new(output_file);

    server::run_server(port, input, output);
}

fn parse_client_args(args: &[String]) {
    let (server_ip, server_port) = args[2].split_once(':').unwrap_or_else(|| {
        eprintln!("Invalid IP:PORT format: {}", args[2]);
        process::exit(1);
    });

    let server_port = server_port.parse::<u16>().unwrap_or_else(|_| {
        eprintln!("Invalid port: {}", server_port);
        process::exit(1);
    });

    let input_path = PathBuf::from(&args[3]);
    let output_path = PathBuf::from(&args[4]);

    let input_file = File::open(&input_path).unwrap_or_else(|_| {
        eprintln!("Failed to open input file: {:?}", input_path);
        process::exit(1);
    });
    let output_file = File::create(&output_path).unwrap_or_else(|_| {
        eprintln!("Failed to create output file: {:?}", output_path);
        process::exit(1);
    });

    let input = BufReader::new(input_file);
    let output = BufWriter::new(output_file);

    let _ = client::run_client((server_ip, server_port), input, output);
}

fn print_usage_and_exit(program_name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} -s <PORT> <INPUT> <OUTPUT>", program_name);
    eprintln!("  {} -c <IP>:<PORT> <INPUT> <OUTPUT>", program_name);
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        print_usage_and_exit(&args[0]);
    }

    match args[1].as_str() {
        "-s" => parse_server_args(&args),
        "-c" => parse_client_args(&args),
        _ => {
            eprintln!("Invalid mode: {}", args[1]);
            print_usage_and_exit(&args[0]);
        }
    }
}
