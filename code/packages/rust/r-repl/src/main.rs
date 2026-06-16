//! The `R` binary — an interactive prompt for the R language.
//!
//! Reads one line at a time (with continuation), evaluates via the shared S
//! tree-walker, and prints any visible result. Type `q()` (or `:quit`, or send
//! EOF with Ctrl-D) to exit. All logic lives in
//! [`coding_adventures_r_repl::run`]; this binary just wires it to stdio.

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_r_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("R: I/O error: {e}");
        std::process::exit(1);
    }
}
