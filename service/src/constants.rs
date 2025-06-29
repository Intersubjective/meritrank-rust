pub use std::num::NonZeroUsize;

pub const VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
  Some(x) => x,
  None => "dev",
};
