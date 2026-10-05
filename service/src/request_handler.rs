use crate::data::*;
use crate::settings::*;
use crate::state_manager::MultiGraphProcessor;
use crate::utils::log::*;

use bincode::{
  config::standard,
  decode_from_slice,
  encode_to_vec,
  Decode,
  Encode,
};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

use std::{error::Error, sync::Arc};

/// Writes a length-prefixed (4-byte big-endian) bincode message.
async fn write_message<T: Encode>(
  stream: &mut TcpStream,
  value: &T,
) -> Result<(), Box<dyn Error>> {
  log_trace!();
  let out = encode_to_vec(value, standard())?;
  //  One write per frame: a separate 4-byte length write followed by the
  //  payload write trips Nagle + the peer's delayed ACK (~40 ms per frame).
  let mut frame = Vec::with_capacity(4 + out.len());
  frame.extend_from_slice(&(out.len() as u32).to_be_bytes());
  frame.extend_from_slice(&out);
  stream.write_all(&frame).await?;
  Ok(())
}

/// Largest accepted frame (MERITRANK_MAX_FRAME_BYTES, default 256 MiB): the length prefix comes
/// from the client and must not size an allocation by itself.
pub fn max_frame_bytes() -> usize {
  static MAX: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    std::env::var("MERITRANK_MAX_FRAME_BYTES")
      .ok()
      .and_then(|s| s.parse().ok())
      .unwrap_or(256 * 1024 * 1024)
  });
  *MAX
}

/// Reads a length-prefixed (4-byte big-endian) bincode message. The body is read as it arrives,
/// so memory grows with the bytes actually received, never with the claimed length alone.
async fn read_message<T: Decode<()>>(stream: &mut TcpStream) -> Result<T, Box<dyn Error>> {
  log_trace!();
  let mut len_buf = [0u8; 4];
  stream.read_exact(&mut len_buf).await?;
  let len = u32::from_be_bytes(len_buf) as usize;
  if len > max_frame_bytes() {
    return Err(format!("frame of {} bytes exceeds the limit of {}", len, max_frame_bytes()).into());
  }
  let mut buf = Vec::with_capacity(len.min(64 * 1024));
  (&mut *stream).take(len as u64).read_to_end(&mut buf).await?;
  if buf.len() != len {
    return Err("truncated frame".into());
  }
  Ok(decode_from_slice(&buf, standard())?.0)
}

#[allow(unused)]
pub async fn write_request(
  stream: &mut TcpStream,
  request: Request,
) -> Result<(), Box<dyn Error>> {
  write_message(stream, &request).await
}

#[allow(unused)]
pub async fn read_request(
  stream: &mut TcpStream,
) -> Result<Request, Box<dyn Error>> {
  read_message(stream).await
}

#[allow(unused)]
pub async fn write_response(
  stream: &mut TcpStream,
  response: Response,
) -> Result<(), Box<dyn Error>> {
  write_message(stream, &response).await
}

#[allow(unused)]
pub async fn read_response(
  stream: &mut TcpStream,
) -> Result<Response, Box<dyn Error>> {
  read_message(stream).await
}

pub async fn run_server(
  settings: Settings,
  processor: Arc<MultiGraphProcessor>,
  running: CancellationToken,
) -> Result<(), Box<dyn Error>> {
  log_trace!();

  let url = format!("{}:{}", settings.server_address, settings.server_port);

  let listener = TcpListener::bind(&url).await?;

  log_verbose!("Server running on {}", url);

  loop {
    let mut stream;

    tokio::select! {
      _ = running.cancelled() => {
        log_verbose!("Server stopped.");
        break;
      }
      accept_result = listener.accept() => {
        match accept_result {
          Ok((s, _)) => {
            //  Small request/response frames: never wait for Nagle.
            if let Err(e) = s.set_nodelay(true) {
              log_warning!("set_nodelay failed: {}", e);
            }
            stream = s;
          },
          Err(e) => {
            log_error!("Socket accept failed: {}", e);
            break;
          },
        };
      }
    };

    let processor_cloned = Arc::clone(&processor);

    tokio::spawn(async move {
      loop {
        let req = match read_request(&mut stream).await {
          Ok(x) => x,
          Err(_) => break,
        };

        let response = processor_cloned.process_request(&req).await;

        if write_response(&mut stream, response).await.is_err() {
          break;
        }
      }
    });
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  use tokio::{
    net::TcpSocket,
    time::{timeout, Duration},
  };

  fn test_settings(port: u16) -> Settings {
    Settings {
      server_port: port,
      min_ops_before_swap: 1,
      ..Settings::default()
    }
  }

  /// Spawns the server on the given port; returns the task handle and cancellation token.
  fn spawn_server(port: u16) -> (tokio::task::JoinHandle<()>, CancellationToken) {
    let running = CancellationToken::new();
    let running_cloned = running.clone();
    let settings = test_settings(port);
    let server_task = tokio::spawn(async move {
      run_server(
        settings.clone(),
        Arc::new(MultiGraphProcessor::new(settings)),
        running_cloned,
      )
      .await
      .unwrap();
    });
    (server_task, running)
  }

  async fn connect_to(port: u16) -> TcpStream {
    TcpSocket::new_v4()
      .unwrap()
      .connect(format!("127.0.0.1:{}", port).parse().unwrap())
      .await
      .unwrap()
  }

  /// Waits for the server to accept connections (retries until timeout), then returns.
  async fn wait_for_server(port: u16) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
      if let Ok(socket) = TcpSocket::new_v4() {
        if socket
          .connect(format!("127.0.0.1:{}", port).parse().unwrap())
          .await
          .is_ok()
        {
          return;
        }
      }
      tokio::task::yield_now().await;
    }
    panic!("server did not become ready within 2s");
  }

  /// Sends a request and returns the response (convenience for tests).
  async fn roundtrip(stream: &mut TcpStream, request: Request) -> Response {
    write_request(stream, request).await.unwrap();
    read_response(stream).await.unwrap()
  }

  /// Roundtrip for a request, then sync so the server has applied it before the next call.
  async fn roundtrip_then_sync(stream: &mut TcpStream, request: Request) -> Response {
    let r = roundtrip(stream, request).await;
    let _ = roundtrip(
      stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::Sync(1),
      },
    )
    .await;
    r
  }

  fn test_score_options() -> FilterOptions {
    FilterOptions {
      node_kind:     None,
      hide_personal: true,
      score_lt:      100.0,
      score_lte:     false,
      score_gt:      -100.0,
      score_gte:     false,
      index:         0,
      count:         100,
    }
  }

  #[tokio::test]
  async fn cancel() {
    let (mut server_task, running) = spawn_server(8081);
    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn request_response() {
    let (mut server_task, running) = spawn_server(8082);
    wait_for_server(8082).await;

    let mut stream = connect_to(8082).await;
    let _ = roundtrip_then_sync(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::WriteEdge(OpWriteEdge {
          src:       "U1".into(),
          dst:       "U2".into(),
          amount:    1.0,
          magnitude: 1,
        }),
      },
    )
    .await;
    // Lazy calculation on read: poll until scores appear (no sleep; same pattern as state_manager tests).
    let mut scores_resp = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U1".into(),
          score_options: test_score_options(),
        }),
      },
    )
    .await;
    for _ in 0..100 {
      if let Response::Scores(ref s) = scores_resp {
        if !s.scores.is_empty() {
          break;
        }
      }
      tokio::task::yield_now().await;
      scores_resp = roundtrip(
        &mut stream,
        Request {
          subgraph: "".into(),
          data:     ReqData::ReadScores(OpReadScores {
            ego:           "U1".into(),
            score_options: test_score_options(),
          }),
        },
      )
      .await;
    }
    match scores_resp {
      Response::Scores(scores) => {
        // U2 is not "owned by U1" so hide_personal=true does not filter it out.
        assert!(scores.scores.len() > 0);
      },
      _ => assert!(false),
    };

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn calculate_and_fetch_score() {
    let (mut server_task, running) = spawn_server(8083);
    wait_for_server(8083).await;

    let mut stream = connect_to(8083).await;
    let _ = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::WriteEdge(OpWriteEdge {
          src:       "U1".into(),
          dst:       "U2".into(),
          amount:    1.0,
          magnitude: 1,
        }),
      },
    )
    .await;
    let _ = roundtrip_then_sync(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::WriteCalculate(OpWriteCalculate { ego: "U1".into() }),
      },
    )
    .await;
    let scores = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::ReadScores(OpReadScores {
          ego:           "U1".into(),
          score_options: test_score_options(),
        }),
      },
    )
    .await;

    match scores {
      Response::Scores(scores) => {
        // Score = visit probability (denominator W), blended with zero opinion (k = 0.2):
        // the ego 1.0 · 0.8, U2 alpha · 0.8 = 0.68.
        assert!(scores.scores.len() == 2);
        assert!(scores.scores[0].score > 0.75);
        assert!(scores.scores[0].score < 0.85);
        assert!(scores.scores[1].score > 0.62);
        assert!(scores.scores[1].score < 0.74);
      },
      _ => assert!(false),
    };

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }

  /// Regression: frames were written as a 4-byte length write plus a payload
  /// write, so with Nagle + delayed ACK every roundtrip on a kept-alive
  /// connection stalled ~40 ms per direction (~88 ms per call in production).
  /// Uses the blocking client the Postgres connector uses, without touching
  /// its socket options, so both the client and the server framing are covered.
  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn sequential_roundtrips_do_not_stall_on_delayed_ack() {
    let (mut server_task, running) = spawn_server(8090);
    wait_for_server(8090).await;

    let elapsed = tokio::task::spawn_blocking(|| {
      let mut stream = std::net::TcpStream::connect("127.0.0.1:8090").unwrap();
      let request = Request {
        subgraph: "".into(),
        data:     ReqData::Sync(1),
      };
      // Warm up: the first exchanges on a fresh connection run in quick-ACK mode.
      for _ in 0..3 {
        crate::rpc_sync::write_request_sync(&mut stream, &request).unwrap();
        crate::rpc_sync::read_response_sync(&mut stream).unwrap();
      }
      let started = std::time::Instant::now();
      for _ in 0..20 {
        crate::rpc_sync::write_request_sync(&mut stream, &request).unwrap();
        crate::rpc_sync::read_response_sync(&mut stream).unwrap();
      }
      started.elapsed()
    })
    .await
    .unwrap();

    // Stalled framing costs >= 20 x 40 ms; healthy loopback is a few ms total.
    assert!(
      elapsed < Duration::from_millis(400),
      "20 roundtrips took {:?}; frames are stalling on Nagle/delayed ACK",
      elapsed
    );

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }
}
