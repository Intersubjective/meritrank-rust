use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
  #[error("Internal error: {0}")]
  Internal(String),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("NNG error: {0}")]
  Nng(#[from] nng::Error),

  #[error("Serde decode error: {0}")]
  SerdeDecode(#[from] rmp_serde::decode::Error),

  #[error("Serde encode error: {0}")]
  SerdeEncode(#[from] rmp_serde::encode::Error),
}
