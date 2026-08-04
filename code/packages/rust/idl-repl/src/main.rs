//! The `idl` binary — an interactive prompt for IDL (Interactive Data
//! Language).
//!
//! Reads one line at a time (continuing across an open `(`/`[`, a trailing
//! `$` line-continuation, or an open `BEGIN...ENDxxx`/`PRO`/`FUNCTION`
//! block), evaluates over `array-runtime`, and prints `PRINT`/Implied-Print
//! output. Type `quit` / `exit` (or send EOF with Ctrl-D) to leave. All
//! logic lives in [`coding_adventures_idl_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_idl_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("idl: I/O error: {e}");
        std::process::exit(1);
    }
}
