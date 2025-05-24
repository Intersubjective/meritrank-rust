use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}};
use serde::{Serialize, Deserialize};
use std::error::Error;
use bincode::{config::standard, decode_from_slice, encode_to_vec, Decode, Encode};

#[derive(Debug, Encode, Decode)]
struct Request {
    number: u64,
}

#[derive(Debug, Encode, Decode)]
struct Response {
    square: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut len_buf = [0u8; 4];
            if socket.read_exact(&mut len_buf).await.is_err() { return; }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            if socket.read_exact(&mut buf).await.is_err() { return; }

            let config = standard();
            let (req, _): (Request, _) = match decode_from_slice(&buf, config) {
                Ok(r) => r,
                Err(_) => return,
            };

            let resp = Response { square: req.number * req.number };
            let out = match encode_to_vec(&resp, config) {
                Ok(data) => data,
                Err(_) => return,
            };
            let len_bytes = (out.len() as u32).to_be_bytes();

            if socket.write_all(&len_bytes).await.is_err() { return; }
            let _ = socket.write_all(&out).await;
        });
    }
}
