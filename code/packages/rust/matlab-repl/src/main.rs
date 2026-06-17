//! The `matlab` binary — an interactive prompt for the MATLAB language.
//!
//! Reads one line at a time (continuing across open brackets and unterminated
//! `if`/`for`/`while`/`function` blocks), evaluates over `array-runtime`, and
//! prints the result. Type `quit` / `exit` (or send EOF with Ctrl-D) to leave.
//! All logic lives in [`coding_adventures_matlab_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_matlab_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("matlab: I/O error: {e}");
        std::process::exit(1);
    }
}
