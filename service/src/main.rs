mod aug_graph;
mod nonblocking_loop;
mod proc_graph;
mod proc_multigraph;
mod request_response;
mod clustering;
mod node_registry;
mod nodes;
mod read;
mod settings;
mod vsids;
mod write;
mod utils;

use crate::utils::log::*;
use crate::proc_multigraph::{
  MultiGraphProcessor, MultiGraphProcessorSettings,
};
use crate::request_response::{Request, Response};

use std::error::Error;
use std::sync::Arc;

use bincode::{config::standard, decode_from_slice, encode_to_vec};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

struct ServerSettings {
  address: String,
}

async fn write_request(
  stream: &mut TcpStream,
  request: Request,
) -> Result<(), Box<dyn Error>> {
  log_trace!();

  let out = encode_to_vec(&request, standard())?;
  let len_bytes = (out.len() as u32).to_be_bytes();

  stream.write_all(&len_bytes).await?;
  stream.write_all(&out).await?;

  Ok(())
}

async fn read_request(
  stream: &mut TcpStream
) -> Result<Request, Box<dyn Error>> {
  log_trace!();

  let mut len_buf = [0u8; 4];

  stream.read_exact(&mut len_buf).await?;

  let len = u32::from_be_bytes(len_buf) as usize;
  let mut buf = vec![0u8; len];

  stream.read_exact(&mut buf).await?;

  Ok(decode_from_slice(&buf, standard())?.0)
}

async fn write_response(
  stream: &mut TcpStream,
  response: Response,
) -> Result<(), Box<dyn Error>> {
  log_trace!();

  let out = encode_to_vec(&response, standard())?;
  let len_bytes = (out.len() as u32).to_be_bytes();

  stream.write_all(&len_bytes).await?;
  stream.write_all(&out).await?;

  Ok(())
}

async fn read_response(
  stream: &mut TcpStream
) -> Result<Response, Box<dyn Error>> {
  log_trace!();

  let mut len_buf = [0u8; 4];

  stream.read_exact(&mut len_buf).await?;

  let len = u32::from_be_bytes(len_buf) as usize;
  let mut buf = vec![0u8; len];

  stream.read_exact(&mut buf).await?;

  Ok(decode_from_slice(&buf, standard())?.0)
}

async fn run_server(
  settings: ServerSettings,
  running: CancellationToken,
) -> Result<(), Box<dyn Error>> {
  log_trace!();

  let listener = TcpListener::bind(&settings.address).await?;

  log_verbose!("Server running on {}", settings.address);

  let processor = Arc::new(MultiGraphProcessor::new(
    MultiGraphProcessorSettings::default(),
  ));

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
          log_error!("Failed to wtite the response: {}", e);
          return;
        },
      };
    });
  }

  Ok(())
}

const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  log_info!("MeritRank Service");

  //  TODO: Run the legacy server.

  let running = CancellationToken::new();

  run_server(
    ServerSettings {
      address: DEFAULT_SERVER_ADDRESS.to_string(),
    },
    running,
  )
  .await
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::read::FilterOptions;
  use crate::request_response::*;

  use tokio::{
    net::TcpSocket,
    time::{sleep, timeout, Duration},
  };

  //  TODO: Server end-to-end tests.

  #[tokio::test]
  async fn cancel_server() {
    let running = CancellationToken::new();

    let running_clonned = running.clone();

    let mut server_task = tokio::spawn(async move {
      let _ = run_server(
        ServerSettings {
          address: "127.0.0.1:8081".into(),
        },
        running_clonned,
      )
      .await
      .unwrap();
    });

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn request_response() {
    let running = CancellationToken::new();

    let running_clonned = running.clone();

    let mut server_task = tokio::spawn(async move {
      let _ = run_server(
        ServerSettings {
          address: "127.0.0.1:8082".into(),
        },
        running_clonned,
      )
      .await
      .unwrap();
    });

    //  FIXME: Sleep must not be required here!
    sleep(Duration::from_millis(10)).await;

    let connect = async || {
      TcpSocket::new_v4()
        .unwrap()
        .connect("127.0.0.1:8082".parse().unwrap())
        .await
        .unwrap()
    };

    let mut stream = connect().await;

    write_request(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::WriteEdge(ReqWriteEdge {
          src:       "U1".into(),
          dst:       "U2".into(),
          amount:    1.0,
          magnitude: 1,
        }),
      },
    )
    .await
    .unwrap();

    let _ = read_response(&mut stream).await.unwrap();

    //  FIXME: Wait for the writing to take effect. This should not be required!!!
    sleep(Duration::from_millis(200)).await;

    //  Reconnect again.
    let mut stream = connect().await;

    write_request(
      &mut stream,
      Request {
        subgraph: "".into(),
        data:     ReqData::ReadScores(ReqReadScores {
          ego:           "U1".into(),
          score_options: FilterOptions {
            //  FIXME: Impl default.
            node_kind:     None,
            hide_personal: true,
            score_lt:      100.0,
            score_lte:     false,
            score_gt:      -100.0,
            score_gte:     false,
            index:         0,
            count:         100,
          },
        }),
      },
    )
    .await
    .unwrap();

    let scores = read_response(&mut stream).await.unwrap();

    match scores {
      Response::Scores(scores) => {
        assert!(scores.data.len() == 2);
        assert!(scores.data[0].score > 0.35);
        assert!(scores.data[0].score < 0.50);
        assert!(scores.data[1].score > 0.25);
        assert!(scores.data[1].score < 0.45);
      },
      _ => assert!(false),
    };

    running.cancel();
    let _ = timeout(Duration::from_secs(1), &mut server_task)
      .await
      .unwrap();
  }
}
