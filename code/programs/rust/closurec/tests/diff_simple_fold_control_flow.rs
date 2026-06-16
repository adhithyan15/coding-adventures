//! Integration test for the `tests/diff/simple-fold-control-flow/` fixture.
//!
//! Exercises the CLOC12.156 addition of the `fold-control-flow` pass to
//! the `--compilation_level SIMPLE` pipeline. The pipeline is now
//! `constant-fold → fold-control-flow`, so an `if` with a
//! statically-decidable condition has its dead branch pruned:
//!
//! ```text
//! if (2 > 3) { keepElse(); } else { takeThis(); }   ⇒  { takeThis(); }
//! if (true)  { alsoKept(); } else { dropped();  }   ⇒  { alsoKept(); }
//! if (4 > 5) { vanishes();  }                       ⇒  ;   (empty)
//! ```
//!
//! The `if (2 > 3)` case is the load-bearing one: it proves the two
//! passes compose — `constant-fold` must first turn `2 > 3` into the
//! literal `false` before `fold-control-flow` can decide the branch.
//! The same input under WHITESPACE_ONLY keeps every `if` verbatim (see
//! the `simple_fold_control_flow_*` unit tests in `src/run.rs`).
//!
//! This is the behavioral oracle for the SIMPLE level's control-flow
//! folding: when we later diff against the real `closure-compiler.jar`,
//! this expected file is the diff target.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-control-flow/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_control_flow_fixture_matches_expected_stdout() {
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
    let expected =
        std::fs::read_to_string("tests/diff/simple-fold-control-flow/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
