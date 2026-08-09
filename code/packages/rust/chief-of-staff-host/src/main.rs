//! Process entry point for the concrete D18 Chief host.

fn main() {
    if let Err(error) = chief_of_staff_host::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
