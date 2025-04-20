use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::dccnet::{
    communication::{self, receive_frame, send_end, send_frame, send_rst},
    network::{self, Payload},
};

pub fn handle_client_send(
    stream: &mut TcpStream,
    input: Arc<Mutex<BufReader<File>>>,
) -> std::io::Result<()> {
    let mut current_file_offset = 0;
    let mut id = network::START_ID;

    loop {
        let mut read_buf = vec![0u8; network::MAX_DATA_SIZE];

        let mut input_file = input.lock().unwrap();
        input_file.seek(SeekFrom::Start(current_file_offset))?;

        let bytes_read = input_file.read(&mut read_buf)?;
        current_file_offset += bytes_read as u64;

        if bytes_read == 0 {
            send_end(stream, id);
            break;
        }

        let payload = Payload::new(read_buf[..bytes_read].to_vec(), id, network::FLAG_SED);

        if let Err(e) = send_frame(stream, &payload) {
            if e.kind() == std::io::ErrorKind::Other {
                send_rst(stream, Some(e.to_string().as_bytes().to_vec()));
                break;
            }

            return Err(e);
        }

        id = communication::next_id(id);
    }

    println!("End send data!");
    Ok(())
}

pub fn handle_client_receive(
    stream: &mut TcpStream,
    output: Arc<Mutex<BufWriter<File>>>,
) -> std::io::Result<()> {
    let mut id: u16 = network::START_ID;

    loop {
        let payload = match receive_frame(stream) {
            Ok(payload) => payload,
            Err(e) => {
                println!("Error receiving frame: {}", e);
                let error_name = e.to_string();
                send_rst(stream, Some(error_name.as_bytes().to_vec()));
                break;
            }
        };

        if payload.flag == network::FLAG_END {
            break;
        }

        if payload.flag == network::FLAG_ACK {
            continue;
        }

        if payload.id != id {
            continue;
        }

        let mut output_file = output.lock().unwrap();
        output_file.write_all(&payload.data)?;
        output_file.flush()?;

        id = communication::next_id(id);
    }

    println!("End receive data!");
    Ok(())
}
