use coding_adventures_guest_microbenchmark_support::{replay_core_file, replay_file};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 { eprintln!("usage: consumer GUEST WORKLOAD REPORT MANIFEST"); return ExitCode::FAILURE; }
    let report = match replay_core_file(&args[3]) { Ok(value) => value, Err(e) => { eprintln!("{e}"); return ExitCode::FAILURE; } };
    let envelope = match replay_file(&args[4], &args[1], &args[2]) { Ok(value) => value, Err(e) => { eprintln!("{e}"); return ExitCode::FAILURE; } };
    if report != envelope.report { eprintln!("core and envelope report differ"); return ExitCode::FAILURE; }
    println!("{} pairs replayed for {} / {}", report.pairs.len(), args[1], args[2]);
    ExitCode::SUCCESS
}
