//! Process entry point for the D18 Chief daemon.

fn main() {
    if let Err(error) = chief_of_staff_daemon::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
