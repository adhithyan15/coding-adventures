//! The `wolfram` (alias `math`) binary — an interactive prompt for the Wolfram
//! Language.
//!
//! Reads one physical line at a time (continuing across open brackets and
//! unterminated strings/comments until the statement is balanced), evaluates over
//! the reused shared symbolic stack, and echoes each displayed result as
//! `Out[n]= «value»`. Type `Quit`/`Exit` (or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_wolfram_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_wolfram_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("wolfram: I/O error: {e}");
        std::process::exit(1);
    }
}
