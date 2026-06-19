//! Integration test for the `tests/diff/simple-dce/` fixture.
//!
//! Exercises the CLOC12.157 addition of the `dce` (dead-code
//! elimination) pass to the `--compilation_level SIMPLE` pipeline. The
//! pipeline is now `constant-fold → fold-control-flow → dce`, and this
//! fixture's function body exercises both of dce's jobs while showing
//! all three passes composing inside one block:
//!
//! ```text
//! function f() {
//!   keep();                      // live, retained
//!   if (4 > 5) { neverRuns(); }  // 4>5⇒false ⇒ if(false){…}⇒; ⇒ dce sweeps it
//!   return 1;                    // terminator, retained
//!   alsoDead();                  // dead-after-return ⇒ dce drops it
//! }
//! ⇒ function f(){keep();return 1};
//! ```
//!
//! `f` is called TWICE so the single-use void statement-inliner (CLOC15)
//! declines it — once dce reduces the body to `{ keep(); return 1; }`, a
//! lone call site would otherwise be spliced away entirely (`keep();`),
//! hiding the dce-inside-the-body effect this fixture exists to show.
//!
//! The same input under WHITESPACE_ONLY keeps every statement (see the
//! `simple_dce_*` unit tests in `src/run.rs`).
//!
//! This is the behavioral oracle for the SIMPLE level's dead-code
//! elimination: when we later diff against the real
//! `closure-compiler.jar`, this expected file is the diff target.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-dce/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_dce_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-dce/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
