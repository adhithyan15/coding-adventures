//! The `scilab` binary — an interactive prompt for Scilab (a subset).
//!
//! Reads one physical line at a time (continuing across an open `(`/`)`,
//! `[`/`]`, `{`/`}`, an unclosed `if`/`select`/`while`/`for` until its `end`,
//! or an unclosed `function` until its `endfunction`), evaluates over
//! `array-runtime` via `scilab-runtime`, and echoes each *displayed*
//! result — a `-->` prompt (`> ` while continuing), matching real Scilab's
//! own console convention. Type `quit`/`exit` (or Ctrl-D) to leave. All
//! logic lives in [`coding_adventures_scilab_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_scilab_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("scilab: I/O error: {e}");
        std::process::exit(1);
    }
}
