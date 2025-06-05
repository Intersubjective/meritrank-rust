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

    let counter_r = Arc::new(AtomicUsize::new(0));
    let counter_w = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let mut handles = vec![];
    for _ in 0..THREADS {

        let counter_r = Arc::clone(&counter_r);
        let counter_w = Arc::clone(&counter_w);
        handles.push(thread::spawn(move || {
            while Instant::now() - start < DURATION {
                //thread::sleep(Duration::from_millis(1));
                let mut rng = rand::thread_rng();
                let random_number = rng.gen_range(0..=99);
                if send_request(random_number).is_ok() {
                    if random_number <1{
                        counter_w.fetch_add(1, Ordering::Relaxed);
                    }else {
                        counter_r.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let total_r = counter_r.load(Ordering::Relaxed);
    let total_w = counter_w.load(Ordering::Relaxed);
    println!("Read requests handled in {}s with {} threads: {}", DURATION.as_secs(), THREADS, total_r);
    println!("Write requests handled in {}s with {} threads: {}", DURATION.as_secs(), THREADS, total_w);
    println!("Throughput R: {:.2} req/s", total_r as f64 / DURATION.as_secs_f64());
    println!("Throughput W: {:.2} req/s", total_w as f64 / DURATION.as_secs_f64());
}
