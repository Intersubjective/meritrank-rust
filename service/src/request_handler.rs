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
  let len_bytes = (out.len() as u32).to_be_bytes();
  stream.write_all(&len_bytes).await?;
  stream.write_all(&out).await?;
  Ok(())
}

/// Reads a length-prefixed (4-byte big-endian) bincode message.
async fn read_message<T: Decode<()>>(stream: &mut TcpStream) -> Result<T, Box<dyn Error>> {
  log_trace!();
  let mut len_buf = [0u8; 4];
  stream.read_exact(&mut len_buf).await?;
  let len = u32::from_be_bytes(len_buf) as usize;
  let mut buf = vec![0u8; len];
  stream.read_exact(&mut buf).await?;
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
          Ok((s, _)) => stream = s,
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
        assert!(scores.scores.len() == 2);
        assert!(scores.scores[0].score > 0.35);
        assert!(scores.scores[0].score < 0.50);
        assert!(scores.scores[1].score > 0.25);
        assert!(scores.scores[1].score < 0.45);
      },
      _ => assert!(false),
    };

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }
}
