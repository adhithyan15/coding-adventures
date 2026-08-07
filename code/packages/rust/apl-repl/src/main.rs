//! The `apl` binary — an interactive prompt for the APL language.
//!
//! Reads one line at a time (continuing across an open `(`), evaluates over
//! `array-runtime`, and prints the auto-printed result. Type `quit` / `exit`
//! (or send EOF with Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_apl_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_apl_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("apl: I/O error: {e}");
        std::process::exit(1);
    }
}
