pub use crate::log_command;
pub use crate::log_error;
pub use crate::log_trace;
pub use crate::log_verbose;
pub use crate::log_warning;

#[allow(unused_imports)]
pub use crate::log_info;

pub use std::sync::atomic::Ordering;
use std::{sync::atomic::AtomicBool, sync::Mutex, thread};

pub static ERROR: AtomicBool = AtomicBool::new(true);
pub static WARNING: AtomicBool = AtomicBool::new(true);
pub static INFO: AtomicBool = AtomicBool::new(true);
pub static VERBOSE: AtomicBool = AtomicBool::new(false);
pub static TRACE: AtomicBool = AtomicBool::new(false);

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log_with_time(message: String) {
  let time = chrono::offset::Local::now();
  let time_str = time.format("%Y-%m-%d %H:%M:%S");
  let millis = time.timestamp_millis() % 1000;
  let full_thread_id = format!("{:?}", thread::current().id());

  let thread_begin = full_thread_id.rfind("(").unwrap_or(0) + 1;
  let thread_len = full_thread_id.find(")").unwrap_or(0) - thread_begin;

  let thread: String = full_thread_id
    .chars()
    .skip(thread_begin)
    .take(thread_len)
    .collect();

  match LOG_MUTEX.lock() {
    Ok(_) => {
      println!("[{:3}.{:03}] {}  {}", time_str, millis, thread, message);
    },
    _ => {
      println!(
        "[{:3}.{:03}] {}  LOG MUTEX FAILED",
        time_str, millis, thread
      );
    },
  };
}

#[macro_export]
macro_rules! log_func_name {
  () => {{
    fn f() {}
    fn type_name_of<T>(_: T) -> &'static str {
      std::any::type_name::<T>()
    }
    let mut full_name = type_name_of(f).strip_suffix("::f").unwrap_or("");
    loop {
      match full_name.strip_suffix("::{{closure}}") {
        Some(s) => full_name = s,
        None => break,
      }
    }
    let name: String = full_name
      .chars()
      .skip(full_name.rfind("::").unwrap_or(0) + 2)
      .collect();
    name
  }};
}

#[macro_export]
macro_rules! log_error {
  ($($arg:expr),*) => {
    if ERROR.load(Ordering::Relaxed) {
      log_with_time(format!("{}:{} ERROR in {}: {}", file!(), line!(), $crate::log_func_name!(), format!($($arg),*)));
    }
  };
}

#[macro_export]
macro_rules! log_warning {
  ($($arg:expr),*) => {
    if WARNING.load(Ordering::Relaxed) {
      log_with_time(format!("{}:{} WARNING {}", file!(), line!(), format!($($arg),*)));
    }
  };
}

#[macro_export]
macro_rules! log_info {
  ($($arg:expr),*) => {
    if INFO.load(Ordering::Relaxed) {
      log_with_time(format!("INFO {}", format!($($arg),*)));
    }
  };
}

#[macro_export]
macro_rules! log_verbose {
  ($($arg:expr),*) => {
    if VERBOSE.load(Ordering::Relaxed) {
      log_with_time(format!("VERBOSE --- {}", format!($($arg),*)));
    }
  };
}

#[macro_export]
macro_rules! log_trace {
  () => {
    if TRACE.load(Ordering::Relaxed) {
      log_with_time(format!("{}:{} TRACE --- --- {}", file!(), line!(), $crate::log_func_name!()));
    }
  };

  ($($arg:expr),+) => {
    if TRACE.load(Ordering::Relaxed) {
      log_with_time(format!("{}:{} TRACE --- --- {}: {}", file!(), line!(), $crate::log_func_name!(), format!($($arg),*)));
    }
  };
}

#[macro_export]
macro_rules! log_command {
  () => {
    if INFO.load(Ordering::Relaxed) {
      log_with_time(format!("CMD {}", $crate::log_func_name!()));
    }
  };

  ($($arg:expr),+) => {
    if INFO.load(Ordering::Relaxed) {
      log_with_time(format!("CMD {}: {}", $crate::log_func_name!(), format!($($arg),*)));
    }
  };
}
