//! The `q` binary — an interactive prompt for the Q language.
//!
//! Reads one line at a time (continuing across an open `(`, `{`, or `[`),
//! evaluates over `array-runtime`, and prints the auto-printed result. Type
//! `quit` / `exit` (or send EOF with Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_q_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_q_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("q: I/O error: {e}");
        std::process::exit(1);
    }
}
