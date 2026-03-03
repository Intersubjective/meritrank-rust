use meritrank_service::legacy_protocol::*;
use nng::options::{Options, RecvTimeout};
use nng::*;
use serde::{de::Deserialize, Serialize};
use std::env::var;
use std::error::Error;
use std::result::Result as StdResult;
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

fn to_box_err(e: impl std::error::Error + Send + Sync + 'static) -> Box<dyn Error + 'static> {
  Box::new(e)
}

fn request_raw(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> StdResult<Message, Box<dyn Error + 'static>> {
  let client = Socket::new(Protocol::Req0).map_err(to_box_err)?;
  if let Some(t) = timeout_msec {
    client
      .set_opt::<RecvTimeout>(Some(Duration::from_millis(t)))
      .map_err(to_box_err)?;
  }
  client.dial(&SERVICE_URL).map_err(to_box_err)?;
  client
    .send(Message::from(payload.as_slice()))
    .map_err(|(_, err)| to_box_err(err))?;
  client.recv().map_err(to_box_err)
}

fn request<T>(
  payload: Vec<u8>,
  timeout_msec: Option<u64>,
) -> StdResult<T, Box<dyn Error + 'static>>
where
  T: Clone + for<'a> Deserialize<'a>,
{
  let msg = request_raw(payload, timeout_msec)?;
  decode_response(msg.as_slice()).map_err(|s: String| s.into())
}

pub fn call<T>(
  cmd: &str,
  context: &str,
  blocking: bool,
  args: impl Serialize,
) -> StdResult<T, Box<dyn Error + 'static>>
where
  T: Clone + for<'a> Deserialize<'a>,
{
  let payload = encode_request(&Command {
    id:       cmd.to_string(),
    context:  context.to_string(),
    blocking,
    payload:  rmp_serde::to_vec(&args).map_err(to_box_err)?,
  })
  .map_err(|e: String| -> Box<dyn Error + 'static> { e.into() })?;
  request(payload, Some(*RECV_TIMEOUT_MSEC))
}

pub fn call_void(
  cmd: &str,
  context: &str,
  blocking: bool,
  args: impl Serialize,
  timeout: Option<u64>,
) -> StdResult<&'static str, Box<dyn Error + 'static>> {
  let payload = encode_request(&Command {
    id:       cmd.to_string(),
    context:  context.to_string(),
    blocking,
    payload:  rmp_serde::to_vec(&args).map_err(to_box_err)?,
  })
  .map_err(|e: String| -> Box<dyn Error + 'static> { e.into() })?;
  let _: () = request(payload, timeout)?;
  Ok("Ok")
}

pub fn service_wrapped() -> StdResult<String, Box<dyn Error + 'static>> {
  call(CMD_VERSION, "", true, ())
}
