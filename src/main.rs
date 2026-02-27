fn main() {
    if let Err(e) = roost::cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
