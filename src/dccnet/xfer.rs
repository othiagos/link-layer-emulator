use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::dccnet::{
    communication::{self, NetworkErrorKind},
    network::{self, Payload},
};

pub async fn handle_client_send(
    stream: &mut TcpStream,
    input: Arc<Mutex<BufReader<File>>>,
) -> std::io::Result<()> {
    let mut current_file_offset = 0;
    let mut id = network::START_ID;

    loop {
        let mut read_buf = vec![0u8; network::MAX_DATA_SIZE];

        let bytes_read = {
            let mut input_file = input.lock().unwrap();
            input_file.seek(SeekFrom::Start(current_file_offset))?;
            input_file.read(&mut read_buf)?
        };
        current_file_offset += bytes_read as u64;

        if bytes_read == 0 {
            communication::send_end(stream, id).await;
            break;
        }

        let payload = Payload::new(read_buf[..bytes_read].to_vec(), id, network::FLAG_SED);
        if let Err(e) = communication::send_frame(stream, &payload).await {
            if e.kind == NetworkErrorKind::ConnectionError {
                println!("Connection error: {}", e);
                break;
            }

            if e.kind == NetworkErrorKind::RetransmissionError {
                break;
            }
            continue;
        }

        id = communication::next_id(id);
    }

    println!("End send data!");
    Ok(())
}

pub async fn handle_client_receive(
    stream: &mut TcpStream,
    output: Arc<Mutex<BufWriter<File>>>,
) -> std::io::Result<()> {
    let mut id: u16 = network::START_ID;

    loop {
        let payload = match communication::receive_frame(stream).await {
            Ok(payload) => payload,
            Err(e) => {
                if e.kind == NetworkErrorKind::UnexpectedFlagError {
                    continue;
                }

                break;
            }
        };

        if payload.flag == network::FLAG_END {
            break;
        }

        if payload.id != id {
            continue;
        }

        {
            let mut output_file = output.lock().unwrap();
            output_file.write_all(&payload.data)?;
            output_file.flush()?;
        }

        id = communication::next_id(id);
    }

    println!("End receive data!");
    Ok(())
}
