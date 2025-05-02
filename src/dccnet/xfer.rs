use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
};

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use crate::dccnet::{
    communication::{self, NetworkErrorKind},
    network::{self, Payload},
};

pub async fn handle_client_send(
    stream_read: &Mutex<OwnedReadHalf>,
    stream_white: &Mutex<OwnedWriteHalf>,
    input: &mut BufReader<File>,
) -> std::io::Result<()> {
    let mut current_file_offset = 0;
    let mut id = network::START_ID;

    loop {
        let mut read_buf = vec![0u8; network::MAX_DATA_SIZE];

        let bytes_read = {
            input.seek(SeekFrom::Start(current_file_offset))?;
            input.read(&mut read_buf)?
        };
        current_file_offset += bytes_read as u64;

        if bytes_read == 0 {
            communication::send_end(stream_white, id).await;
            break;
        }

        let payload = Payload::new(read_buf[..bytes_read].to_vec(), id, network::FLAG_SED);
        if let Err(e) = communication::send_frame(stream_read, stream_white, &payload).await {
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
    stream_read: &Mutex<OwnedReadHalf>,
    stream_white: &Mutex<OwnedWriteHalf>,
    output: &mut BufWriter<File>,
) -> std::io::Result<()> {
    let mut id: u16 = network::START_ID;

    loop {
        let payload = match communication::receive_frame(stream_read, stream_white).await {
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
            output.write_all(&payload.data)?;
            output.flush()?;
        }

        id = communication::next_id(id);
    }

    println!("End receive data!");
    Ok(())
}
