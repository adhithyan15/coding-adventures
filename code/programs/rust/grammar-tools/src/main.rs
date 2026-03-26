// =============================================================================
// grammar-tools — binary entry point
// =============================================================================
//
// This is the thin binary wrapper. All logic lives in lib.rs so it can be
// independently unit-tested without spawning a subprocess.

use std::env;
use std::process;

use grammar_tools_cli::run;

fn main() {
    let argv: Vec<String> = env::args().collect();
    let code = run(argv);
    process::exit(code);
}
