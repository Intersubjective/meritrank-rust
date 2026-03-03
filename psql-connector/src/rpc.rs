use meritrank_service::legacy_protocol::*;
use nng::options::{Options, RecvTimeout};
use nng::*;
use serde::{de::Deserialize, Serialize};
use std::env::var;
use std::error::Error;
use std::sync::LazyLock;
use std::time::Duration;

pub static SERVICE_URL: LazyLock<String> = LazyLock::new(|| {
  var("MERITRANK_SERVICE_URL")
    .unwrap_or_else(|_| "tcp://127.0.0.1:10234".to_string())
});

pub static RECV_TIMEOUT_MSEC: LazyLock<u64> = LazyLock::new(|| {
  var("MERITRANK_RECV_TIMEOUT_MSEC")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(10000)
});

fn request_raw(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> Result<Message, Box<dyn Error + 'static>> {
  let client = Socket::new(Protocol::Req0)?;
  if let Some(t) = timeout_msec {
    client.set_opt::<RecvTimeout>(Some(Duration::from_millis(t)))?;
  }
  client.dial(&SERVICE_URL)?;
  client
    .send(Message::from(payload.as_slice()))
    .map_err(|(_, err)| err)?;
  Ok(client.recv()?)
}

fn request<T>(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> Result<T, Box<dyn Error + 'static>>
where
  T: Clone + for<'a> Deserialize<'a>,
{
  let msg = request_raw(payload, timeout_msec)?;
  decode_response(msg.as_slice()).map_err(|s| s.into())
}

pub fn call<T>(
  cmd: &str,
  context: &str,
  blocking: bool,
  args: impl Serialize,
) -> Result<T, Box<dyn Error + 'static>>
where
  T: Clone + for<'a> Deserialize<'a>,
{
  let payload = encode_request(&Command {
    id:       cmd.to_string(),
    context:  context.to_string(),
    blocking,
    payload:  rmp_serde::to_vec(&args)?,
  })?;
  request(payload, Some(*RECV_TIMEOUT_MSEC))
}

pub fn call_void(
  cmd: &str,
  context: &str,
  blocking: bool,
  args: impl Serialize,
  timeout: Option<u64>,
) -> Result<&'static str, Box<dyn Error + 'static>> {
  let payload = encode_request(&Command {
    id:       cmd.to_string(),
    context:  context.to_string(),
    blocking,
    payload:  rmp_serde::to_vec(&args)?,
  })?;
  let _: () = request(payload, timeout)?;
  Ok("Ok")
}

pub fn service_wrapped() -> Result<String, Box<dyn Error + 'static>> {
  call(CMD_VERSION, "", true, ())
}
