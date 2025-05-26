use crate::request_handler::main_async;
use ctrlc;

fn main() -> Result<(), ()> {
    let _ = ctrlc::set_handler(move || {
        println!("");
        std::process::exit(0)
    });

    main_async()
}
