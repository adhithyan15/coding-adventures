//! The `derive` binary — an interactive prompt for Derive (a subset).
//!
//! Reads one physical line at a time (continuing across an open `(`/`[`
//! until balanced), evaluates over the reused shared symbolic stack, and
//! echoes each result as `#n: «value»` — Derive's own numbered-worksheet
//! convention (MA07 §5). Type `QUIT`/`EXIT` (or Ctrl-D) to leave. All logic
//! lives in [`coding_adventures_derive_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_derive_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("derive: I/O error: {e}");
        std::process::exit(1);
    }
}
