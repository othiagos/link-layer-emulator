use super::network::Payload;
use super::network;

pub fn check_received_rst(payload: &Payload) {
    if payload.flag == network::FLAG_RST && payload.id == u16::MAX {
        let payload_msg = String::from_utf8(payload.data.clone()).unwrap();
        eprintln!("Invalid Frame: {}", payload_msg);
        std::process::exit(1);
    }
}