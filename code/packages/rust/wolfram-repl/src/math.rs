//! The `math` binary — an alias for the `wolfram` interactive prompt.
//!
//! Wolfram historically shipped the command-line kernel as `math`; we offer the
//! same entry point. Behaviour is identical to the `wolfram` binary — both drive
//! [`coding_adventures_wolfram_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_wolfram_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("math: I/O error: {e}");
        std::process::exit(1);
    }
}
