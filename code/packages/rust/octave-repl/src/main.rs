//! The `octave` binary — an interactive prompt for GNU Octave.
//!
//! Reads one line at a time (continuing across open brackets and unterminated
//! `if`/`for`/`while`/… blocks, closed by `end` or `endif`/`endfor`/…),
//! normalizes Octave syntax to MATLAB, and evaluates over `array-runtime`. Type
//! `quit`/`exit` (or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_octave_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_octave_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("octave: I/O error: {e}");
        std::process::exit(1);
    }
}
