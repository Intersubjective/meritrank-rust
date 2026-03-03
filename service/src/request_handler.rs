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
      log_trace!("read request");

      let req = match read_request(&mut stream).await {
        Ok(x) => x,
        Err(e) => {
          log_error!("Failed to read the request: {}", e);
          return;
        },
      };

      let response = processor_cloned.process_request(&req).await;

      match write_response(&mut stream, response).await {
        Ok(_) => {},
        Err(e) => {
          log_error!("Failed to write the response: {}", e);
          return;
        },
      };
    });
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  use tokio::{
    net::TcpSocket,
    time::{sleep, timeout, Duration},
  };

  fn test_settings(port: u16) -> Settings {
    Settings {
      server_port: port,
      sleep_duration_after_publish_ms: 0,
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

  /// Sends a request and returns the response (convenience for tests).
  async fn roundtrip(stream: &mut TcpStream, request: Request) -> Response {
    write_request(stream, request).await.unwrap();
    read_response(stream).await.unwrap()
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

    //  FIXME: Sleep must not be required here!
    sleep(Duration::from_millis(10)).await;

    let mut stream = connect_to(8082).await;
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

    let mut stream = connect_to(8082).await;
    let _ = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::Sync(1),
      },
    )
    .await;

    let mut stream = connect_to(8082).await;
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
        //  With auto-calculate (D1 in JOURNAL): WriteEdge now triggers
        //  calculate for new nodes inside aug_graph::set_edge, so scores
        //  are non-empty after put_edge + sync even without explicit
        //  WriteCalculate. U2 is not "owned by U1" so hide_personal=true
        //  does not filter it out.
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

    //  FIXME: Sleep must not be required here!
    sleep(Duration::from_millis(10)).await;

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

    let mut stream = connect_to(8083).await;
    let _ = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::WriteCalculate(OpWriteCalculate { ego: "U1".into() }),
      },
    )
    .await;

    let mut stream = connect_to(8083).await;
    let _ = roundtrip(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::Sync(1),
      },
    )
    .await;

    let mut stream = connect_to(8083).await;
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
