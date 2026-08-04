//! The `maple` binary — an interactive prompt for Maple (a subset).
//!
//! Reads one physical line at a time (continuing across an open `(`/`)`,
//! `[`/`]`, `{`/`}`, or an unclosed `if` until its `end if`/`fi`),
//! evaluates over the reused shared symbolic stack, and echoes each
//! *displayed* result — a plain, non-numbered read-eval-print loop (MA09
//! §2/§5; unlike `derive`'s `#n:` worksheet convention). Type `QUIT`/`EXIT`
//! (or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_maple_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_maple_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("maple: I/O error: {e}");
        std::process::exit(1);
    }
}
