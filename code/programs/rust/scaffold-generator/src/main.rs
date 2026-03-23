// =========================================================================
// scaffold-generator -- binary entry point
// =========================================================================
//
// This is the thin binary wrapper. All logic lives in lib.rs so it can
// be tested via integration tests.

use std::env;
use std::io;
use std::process;

use scaffold_generator::run;

fn main() {
    let argv: Vec<String> = env::args().collect();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let code = run(argv, &mut stdout, &mut stderr);
    process::exit(code);
}
