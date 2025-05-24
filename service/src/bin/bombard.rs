use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use rand::Rng;


use bincode::{config::standard, Decode, Encode, decode_from_slice, encode_to_vec};

#[derive(Encode, Decode, Debug)]
struct Request {
    number: u64,
}

#[derive(Encode, Decode, Debug)]
struct Response {
    response: u64,
}

fn send_request(id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    let req = Request { number: id };
    let config = standard();

    let data = encode_to_vec(&req, config)?;
    let len = (data.len() as u32).to_be_bytes();

    stream.write_all(&len)?;
    stream.write_all(&data)?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0u8; resp_len];
    stream.read_exact(&mut buf)?;

    let (_resp, _): (Response, usize) = decode_from_slice(&buf, config)?;

    Ok(())
}

fn main() {
    const THREADS: usize = 16;
    const DURATION: Duration = Duration::from_secs(10);

    let counter = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let mut handles = vec![];
    for _ in 0..THREADS {

        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            while Instant::now() - start < DURATION {
                let mut rng = rand::thread_rng();
                let random_number = rng.gen_range(0..=99);
                if send_request(random_number).is_ok() {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let total = counter.load(Ordering::Relaxed);
    println!("Requests handled in {}s with {} threads: {}", DURATION.as_secs(), THREADS, total);
    println!("Throughput: {:.2} req/s", total as f64 / DURATION.as_secs_f64());
}
