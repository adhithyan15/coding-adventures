//! The `maxima` binary — an interactive prompt for the Maxima CAS.
//!
//! Reads one line at a time (continuing across open brackets and unterminated
//! statements until a `;` or `$` terminator), evaluates over the reused Macsyma
//! symbolic stack, and echoes each displayed result as `(%o«n») «value»`. Type
//! `quit;`/`exit` (or Ctrl-D) to leave. All logic lives in
//! [`coding_adventures_maxima_repl::run`].

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = coding_adventures_maxima_repl::run(stdin.lock(), stdout.lock()) {
        eprintln!("maxima: I/O error: {e}");
        std::process::exit(1);
    }
}
