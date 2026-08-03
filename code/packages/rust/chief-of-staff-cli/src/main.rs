//! Process entry point for the D18 Chief command-line client.

fn main() {
    if let Err(error) = chief_of_staff_cli::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
