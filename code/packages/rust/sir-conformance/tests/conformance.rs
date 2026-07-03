//! The golden conformance matrix.
//!
//! Each [`Program`] in `CORPUS` is real Ruby source paired with the exact
//! stdout a Ruby interpreter would produce. The matrix test lowers every
//! program through the frontend and runs it through **every** backend's real
//! toolchain, asserting the output equals the reference on each — the single
//! test that drives Ruby *source* all the way to Python, JavaScript, Go, and
//! Rust and proves they agree.

use sir_conformance::{run, Program, RunOutcome, Target};

/// The reference corpus. Outputs are intentionally strings and integers only —
/// booleans render differently across backends (`#t` vs `true`), which is a
/// separate formatting concern, not a behavioural one, so we keep the oracle
/// unambiguous.
const CORPUS: &[Program] = &[
    // Operator precedence: `*` binds tighter than `+`.
    Program {
        name: "arithmetic",
        ruby: "puts(2 + 3 * 4)\n",
        expected: "14",
    },
    // A method with parameters, returning its last expression.
    Program {
        name: "def_params",
        ruby: "def add(a, b)\n  a + b\nend\n\nputs add(2, 3)\n",
        expected: "5",
    },
    // Implicit return of a trailing `if` (FC — shipped, proven here end-to-end).
    Program {
        name: "tail_if",
        ruby: "def bigger(a, b)\n  if a > b\n    a\n  else\n    b\n  end\nend\n\nputs bigger(10, 7)\n",
        expected: "10",
    },
    // Implicit return of a trailing `case` (relies on the `case_eq` builtin
    // that had been missing from Go/Rust/JS — this is the regression oracle for
    // that whole class of bug).
    Program {
        name: "tail_case",
        ruby: "def grade(n)\n  case n\n  when 90\n    \"A\"\n  when 80\n    \"B\"\n  else\n    \"C\"\n  end\nend\n\nputs grade(90)\nputs grade(80)\nputs grade(50)\n",
        expected: "A\nB\nC",
    },
    // String concatenation (polymorphic `+`).
    Program {
        name: "string_concat",
        ruby: "puts(\"ab\" + \"cd\")\n",
        expected: "abcd",
    },
    // A user-defined class, `.new`, and an instance method call.
    Program {
        name: "oop_method",
        ruby: "class Dog\n  def speak\n    \"woof\"\n  end\nend\n\nputs Dog.new.speak\n",
        expected: "woof",
    },
];

/// Every program must produce its reference output on every available backend.
/// A backend whose toolchain is absent is skipped (logged), never failed; a
/// mismatch or a crash is a hard failure naming the exact `(program, backend)`.
#[test]
fn corpus_agrees_across_all_backends() {
    let mut ran = 0usize;
    let mut skipped = 0usize;

    for program in CORPUS {
        for &target in Target::all() {
            match run(program, target) {
                RunOutcome::Ran(stdout) => {
                    assert_eq!(
                        stdout, program.expected,
                        "\nCONFORMANCE MISMATCH\n  program:  {}\n  backend:  {}\n  expected: {:?}\n  actual:   {:?}\n",
                        program.name,
                        target.tag(),
                        program.expected,
                        stdout,
                    );
                    ran += 1;
                }
                RunOutcome::Skipped(why) => {
                    eprintln!("skip {}/{}: {why}", program.name, target.tag());
                    skipped += 1;
                }
                RunOutcome::Failed(msg) => {
                    panic!(
                        "\nCONFORMANCE FAILURE\n  program: {}\n  backend: {}\n  {msg}\n",
                        program.name,
                        target.tag(),
                    );
                }
            }
        }
    }

    eprintln!(
        "conformance matrix: {} corpus x {} backends = {ran} ran, {skipped} skipped",
        CORPUS.len(),
        Target::all().len(),
    );
    // The harness is worthless if it proved nothing; require at least one real
    // run so a fully-toolchain-less CI box can't report a hollow pass.
    assert!(
        ran > 0,
        "no backend toolchain was available — conformance proved nothing"
    );
}

/// Sanity: every corpus program lowers through the frontend (independent of any
/// backend toolchain), so a parse/lower regression is caught even on a host
/// with no toolchains at all.
#[test]
fn corpus_all_lowers() {
    for program in CORPUS {
        sir_conformance::lower(program)
            .unwrap_or_else(|e| panic!("`{}` failed to lower: {e}", program.name));
    }
}
