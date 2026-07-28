//! The `axiom` binary — an interactive prompt for Axiom (a subset).
//!
//! Reads one physical line at a time (continuing across an open `(`/`[`
//! until balanced, skipping over `--` line comments and `"..."` string
//! contents so a bracket character inside either never falsely triggers
//! continuation), evaluates over the reused shared symbolic stack plus this
//! crate's own fixed domain/category layer (MA13 §2/§3), and echoes each
//! result with real Axiom's own numbered-prompt convention, `(n)` (MA13 §5).
//! Type `)quit` (or `quit`, or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_axiom_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_axiom_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("axiom: I/O error: {e}");
        std::process::exit(1);
    }
}
