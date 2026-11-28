use coding_adventures_guest_microbenchmark_support::elapsed_ns;
use coding_adventures_starlark_microbenchmark::run;
use std::env;
use std::io;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let start = Instant::now();
    let mut clock = || elapsed_ns(&start);
    match run(&args, &mut io::stdout(), &mut clock) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("starlark-microbenchmark: {error}");
            ExitCode::FAILURE
        }
    }
}
