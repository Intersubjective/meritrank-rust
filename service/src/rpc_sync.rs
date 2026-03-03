use crate::data::{Request, Response};

use bincode::{config::standard, decode_from_slice, encode_to_vec};

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;


pub fn write_request_sync(
  stream: &mut TcpStream,
  request: &Request,
) -> io::Result<()> {
  let payload = encode_to_vec(request, standard())
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
  let len_bytes = (payload.len() as u32).to_be_bytes();
  stream.write_all(&len_bytes)?;
  stream.write_all(&payload)?;
  Ok(())
}

pub fn read_response_sync(stream: &mut TcpStream) -> io::Result<Response> {
  let mut len_buf = [0u8; 4];
  stream.read_exact(&mut len_buf)?;
  let len = u32::from_be_bytes(len_buf) as usize;
  let mut buf = vec![0u8; len];
  stream.read_exact(&mut buf)?;
  decode_from_slice(&buf, standard())
    .map(|(v, _)| v)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

pub fn set_read_timeout(
  stream: &mut TcpStream,
  timeout_msec: Option<u64>,
) -> io::Result<()> {
  let t = timeout_msec.map(Duration::from_millis);
  stream.set_read_timeout(t)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::data::*;

  fn encode_framed<T: bincode::Encode>(val: &T) -> Vec<u8> {
    let payload = encode_to_vec(val, standard()).unwrap();
    let mut buf = (payload.len() as u32).to_be_bytes().to_vec();
    buf.extend_from_slice(&payload);
    buf
  }

  fn decode_framed<T: bincode::Decode<()>>(buf: &[u8]) -> T {
    let len = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
    decode_from_slice(&buf[4..4 + len], standard()).unwrap().0
  }

  #[test]
  fn request_write_edge_roundtrip() {
    let req = Request {
      subgraph: "ctx".into(),
      data:     ReqData::WriteEdge(OpWriteEdge {
        src:       "U1".into(),
        dst:       "U2".into(),
        amount:    1.5,
        magnitude: 3,
      }),
    };
    let framed = encode_framed(&req);
    let decoded: Request = decode_framed(&framed);
    if let ReqData::WriteEdge(op) = decoded.data {
      assert_eq!(op.src, "U1");
      assert_eq!(op.dst, "U2");
      assert_eq!(op.amount, 1.5);
      assert_eq!(op.magnitude, 3);
    } else {
      panic!("wrong variant");
    }
  }

  #[test]
  fn response_scores_roundtrip() {
    let resp = Response::Scores(ResScores {
      scores: vec![ScoreResult {
        ego:             "U1".into(),
        target:          "U2".into(),
        score:           0.42,
        reverse_score:   0.1,
        cluster:         2,
        reverse_cluster: 1,
      }],
    });
    let framed = encode_framed(&resp);
    let decoded: Response = decode_framed(&framed);
    if let Response::Scores(s) = decoded {
      assert_eq!(s.scores.len(), 1);
      assert_eq!(s.scores[0].score, 0.42);
    } else {
      panic!("wrong variant");
    }
  }

  #[test]
  fn response_ok_roundtrip() {
    let framed = encode_framed(&Response::Ok);
    let decoded: Response = decode_framed(&framed);
    assert!(matches!(decoded, Response::Ok));
  }
}
