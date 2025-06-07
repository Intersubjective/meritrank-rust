use meritrank_service;


#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_data;

use meritrank_service::request_handler::main_async;

fn main() {
  let _ = ctrlc::set_handler(move || {
    println!();
    std::process::exit(0);
  });

  if let Err(e) = main_async() {
    eprintln!("Error in main_async: {:?}", e);
    std::process::exit(1);
  }
}
