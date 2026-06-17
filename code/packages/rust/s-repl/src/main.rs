//! The `s` binary — an interactive prompt for the historical Bell Labs S language.
//!
//! Reads one line at a time, accumulating continuation lines until a statement
//! is complete, then evaluates it and prints any visible result. Type `q()` (or
//! `:quit`, or send EOF with Ctrl-D) to exit. All the logic lives in the
//! library's [`coding_adventures_s_repl::run`]; this binary just wires it to
//! stdin and stdout.

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_s_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("s: I/O error: {e}");
        std::process::exit(1);
    }
}
