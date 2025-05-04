use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    sync::Arc,
};

use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::Mutex,
};

use super::{
    communication::{self, NetworkErrorKind},
    network::{self, Payload},
    sync_read
};

pub async fn handle_connection(stream: TcpStream, input: &mut BufReader<File>, output: &mut BufWriter<File>) {
    let (reader_half, writer_half) = stream.into_split();
    let reader_half_mutex =  Arc::new(Mutex::new(reader_half));
    let writer_half_mutex = Arc::new(Mutex::new(writer_half));
    
    let future_send = handle_client_send( &writer_half_mutex, input);
    let future_receive = handle_client_receive(&writer_half_mutex, output);
    
    sync_read::read_stream_data_loop(reader_half_mutex).await;
    let (result_send, result_receive) = tokio::join!(future_send, future_receive);
    
    result_send.unwrap_or_else(|e| {
        let error_message = e.to_string();
        let writer_half_mutex_clone = Arc::clone(&writer_half_mutex);
        tokio::spawn(async move {
            communication::send_rst(&writer_half_mutex_clone, Some(error_message.as_bytes().to_vec())).await;
        });
        eprintln!("Error receiving data: {}", e);
    });
    
    result_receive.unwrap_or_else(|e| {
        eprintln!("Error sending data: {}", e);
    });

    writer_half_mutex.lock().await.shutdown().await.unwrap_or_else(|e| {
        eprintln!("Error shutting connection: {}", e);
    });
    
}

pub async fn handle_client_send(
    stream_white: &Mutex<OwnedWriteHalf>,
    input: &mut BufReader<File>,
) -> std::io::Result<()> {
    let mut id = network::START_ID;

    loop {
        let mut read_buf = vec![0u8; network::MAX_DATA_SIZE];

        let bytes_read = input.read(&mut read_buf)?;
        if bytes_read == 0 {
            communication::send_end(stream_white, id).await;
            break;
        }

        let payload = Payload::new(read_buf[..bytes_read].to_vec(), id, network::FLAG_SED);
        if let Err(e) = communication::send_frame(stream_white, &payload).await {
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
    stream_white: &Mutex<OwnedWriteHalf>,
    output: &mut BufWriter<File>,
) -> std::io::Result<()> {
    let mut id: u16 = network::START_ID;

    loop {
        let payload = match communication::receive_frame(stream_white).await {
            Ok(payload) => payload,
            Err(e) => {
                if e.kind == NetworkErrorKind::UnexpectedFlagError {
                    continue;
                }

                break;
            }
        };

        if payload.flag == network::FLAG_END {
            if !payload.data.is_empty() {
                output.write_all(&payload.data)?;
                output.flush()?;
            }

            break;
        }

        if payload.id != id {
            continue;
        }

        output.write_all(&payload.data)?;
        output.flush()?;

        id = communication::next_id(id);
    }

    println!("End receive data!");
    Ok(())
}
