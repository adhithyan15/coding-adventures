//! The `reduce` binary — an interactive prompt for Reduce (a subset).
//!
//! Reads one physical line at a time (continuing across an open `(`/`)`,
//! `{`/`}`, or `<<`/`>>` until balanced), evaluates over the reused shared
//! symbolic stack, and echoes each result — a plain, non-numbered
//! read-eval-print loop (MA08 §2/§5; unlike `derive`'s `#n:` worksheet
//! convention). Type `QUIT`/`EXIT` (or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_reduce_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_reduce_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("reduce: I/O error: {e}");
        std::process::exit(1);
    }
}
