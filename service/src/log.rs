use std::{sync::atomic::AtomicBool, sync::Mutex, thread};

pub static ERROR: AtomicBool = AtomicBool::new(true);
pub static WARNING: AtomicBool = AtomicBool::new(true);
pub static INFO: AtomicBool = AtomicBool::new(true);
pub static VERBOSE: AtomicBool = AtomicBool::new(true);
pub static TRACE: AtomicBool = AtomicBool::new(true);

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log_with_time(
  prefix: &str,
  message: &str,
) {
  let time = chrono::offset::Local::now();
  let time_str = time.format("%Y-%m-%d %H:%M:%S");
  let millis = time.timestamp_millis() % 1000;
  let thread_id = thread::current().id();

  match LOG_MUTEX.lock() {
    Ok(_) => {
      println!(
        "{}.{:03} {:3?}  {}{}",
        time_str, millis, thread_id, prefix, message
      );
    },
    _ => {
      println!(
        "{}.{:03} {:3?}  LOG MUTEX FAILED",
        time_str, millis, thread_id
      );
    },
  };
}

#[macro_export]
macro_rules! log_error {
  ($($arg:expr),*) => {
    if ERROR.load(Ordering::Relaxed) {
      log_with_time("ERROR   ", format!($($arg),*).as_str());
    }
  };
}

#[macro_export]
macro_rules! log_warning {
  ($($arg:expr),*) => {
    if WARNING.load(Ordering::Relaxed) {
      log_with_time("WARNING ", format!($($arg),*).as_str());
    }
  };
}

#[macro_export]
macro_rules! log_info {
  ($($arg:expr),*) => {
    if INFO.load(Ordering::Relaxed) {
      log_with_time("INFO    ", format!($($arg),*).as_str());
    }
  };
}

#[macro_export]
macro_rules! log_verbose {
  ($($arg:expr),*) => {
    if VERBOSE.load(Ordering::Relaxed) {
      log_with_time("VERBOSE --- ", format!($($arg),*).as_str());
    }
  };
}

#[macro_export]
macro_rules! log_trace {
  ($($arg:expr),*) => {
    if TRACE.load(Ordering::Relaxed) {
      log_with_time("TRACE   --- --- ", format!($($arg),*).as_str());
    }
  };
}
