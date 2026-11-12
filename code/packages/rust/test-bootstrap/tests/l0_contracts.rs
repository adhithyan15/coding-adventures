//! # L0 contracts: "fails when it should" (verdict, runner, TAP, exit codes)
//!
//! Every other verification crate is graded by this runner, so the one error
//! that must never happen is a *failing check reported as passing*. These
//! tests attack that directly, from the verdict core up to the real process
//! exit status:
//!
//! ```text
//!   verdict path ──► Suite::run ──► TAP bytes ──► process exit status
//!   (each one)       (in process)   (exact)       (real binary, exit_fixture)
//! ```
//!
//! They run on libtest, not on the harness under test, so the harness is not
//! the thing reporting on itself.
//!
//! ## What the harness promises about "vacuous" tests
//!
//! A test body is `Fn(&Ctx) -> Verdict`. The harness reports *the verdict the
//! body returns*. It does not count assertions, so a body that returns
//! `Verdict::Pass` early, or without calling the core at all, is a pass.
//! That is the spec'd L0 behaviour (TF01 §7: "no assertion beyond `deep_equal`
//! and `error_is`"); `a_body_with_zero_checks_passes_by_design` pins it so a
//! change is deliberate. A body that *panics* is never a pass.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};

use coding_adventures_test_bootstrap::prng::shuffled;
use coding_adventures_test_bootstrap::run::{self, parse_args, Options, Suite, Summary};
use coding_adventures_test_bootstrap::tfv::{self, Value};
use coding_adventures_test_bootstrap::verdict::{self, deep_equal, error_is};
use coding_adventures_test_bootstrap::{pbt, Verdict};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_suite(suite: &Suite, options: &Options) -> (String, Summary) {
    let mut buffer = Vec::new();
    let summary = suite.run(options, &mut buffer).expect("write to a Vec");
    (String::from_utf8(buffer).expect("TAP is UTF-8"), summary)
}

fn text_of(source: &str) -> Value {
    let records = tfv::parse_file(&format!("tfv 1\nr R1 Kv {source}\n")).expect(source);
    records[0].fields[0].1.clone()
}

/// Cargo supplies absolute paths for package binaries when compiling tests.
/// Keep this lookup independent of the integration-test executable's location:
/// Cargo may place test executables in per-target `OUT_DIR` directories.
fn sibling_binary(name: &str) -> PathBuf {
    match name {
        "self_corpus" => PathBuf::from(env!("CARGO_BIN_EXE_self_corpus")),
        "exit_fixture" => PathBuf::from(env!("CARGO_BIN_EXE_exit_fixture")),
        other => panic!("unregistered binary {other:?}"),
    }
}

fn spawn_fixture(mode: &str, args: &[&str]) -> Output {
    Command::new(sibling_binary("exit_fixture"))
        .args(args)
        .env("TFB_FIXTURE", mode)
        .output()
        .expect("spawn exit_fixture")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("TAP is UTF-8")
}

fn not_ok_ids(tap: &str) -> BTreeSet<String> {
    tap.lines()
        .filter_map(|l| l.strip_prefix("not ok "))
        .map(|rest| rest.split_once(" - ").expect("not ok N - id").1.to_owned())
        .collect()
}

fn ok_count(tap: &str) -> usize {
    tap.lines().filter(|l| l.starts_with("ok ")).count()
}

fn not_ok_count(tap: &str) -> usize {
    tap.lines().filter(|l| l.starts_with("not ok ")).count()
}

/// What a supervisor must check beyond the exit status: the plan line `1..N`
/// matches the number of result lines, and no `Bail out!` appeared.
fn tap_is_complete(tap: &str) -> bool {
    let plan: Option<usize> = tap
        .lines()
        .find_map(|l| l.strip_prefix("1.."))
        .and_then(|n| n.parse().ok());
    let results = ok_count(tap) + not_ok_count(tap);
    plan == Some(results) && !tap.lines().any(|l| l.starts_with("Bail out!"))
}

// ---------------------------------------------------------------------------
// Verdict core: every failing path yields a registered, non-pass verdict
// ---------------------------------------------------------------------------

/// Pairs that are NOT deep-equal, covering every row of TF01 §3.1.
fn unequal_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("N", "F"),
        ("N", "I0"),
        ("T", "F"),
        ("F", "T"),
        ("T", "I1"),
        ("I1", "I2"),
        ("I-1", "I1"),
        ("I10", "I1"),
        ("I1", "D3ff0000000000000"),
        ("D0000000000000000", "D8000000000000000"),
        ("D3ff0000000000000", "D3ff0000000000001"),
        ("D7ff0000000000000", "D7ff8000000000000"),
        ("D7ff0000000000000", "Dfff0000000000000"),
        ("X", "S"),
        ("X61", "S61"),
        ("X61", "X6162"),
        ("X6162", "X6261"),
        ("S61", "S62"),
        ("S", "S00"),
        ("L0", "L1 N"),
        ("L2 N T", "L2 T N"),
        ("L1 I1", "L1 I2"),
        ("L0", "M0"),
        ("M0", "L0"),
        ("M1 S61 N", "M1 S62 N"),
        ("M1 S61 N", "M1 S61 T"),
        ("M1 S61 N", "M0"),
        ("M2 N N T T", "M2 N N T F"),
        ("L1 L1 N", "L1 L1 T"),
    ]
}

#[test]
fn every_unequal_pair_fails_with_not_deep_equal_in_both_directions() {
    for (a, b) in unequal_pairs() {
        for (x, y) in [(a, b), (b, a)] {
            match deep_equal(&text_of(x), &text_of(y)) {
                Verdict::Fail {
                    failure_id,
                    message,
                } => {
                    assert_eq!(failure_id, "not-deep-equal", "{x} vs {y}");
                    assert!(verdict::is_registered_failure_id(failure_id));
                    // The message shows both sides in corpus notation.
                    assert_eq!(message, format!("expected {y}, got {x}"));
                }
                Verdict::Pass => panic!("{x} and {y} must not be deep-equal"),
            }
        }
    }
}

#[test]
fn equal_pairs_pass() {
    for (a, b) in [
        ("N", "N"),
        ("I5", "I5"),
        ("D7ff8000000000000", "D7ff0000000000001"),
        ("D8000000000000000", "D8000000000000000"),
        ("X00ff", "X00ff"),
        ("S6869", "S6869"),
        ("L2 I1 L1 N", "L2 I1 L1 N"),
        ("M2 N T I1 F", "M2 I1 F N T"),
    ] {
        assert_eq!(
            deep_equal(&text_of(a), &text_of(b)),
            Verdict::Pass,
            "{a} {b}"
        );
    }
}

#[test]
fn error_is_covers_all_three_rows_and_never_passes_a_mismatch() {
    assert_eq!(error_is(Some("A"), "A"), Verdict::Pass);
    for (raised, expected) in [
        (Some("B"), "A"),
        (Some("a"), "A"),
        (Some("A "), "A"),
        (Some(""), "A"),
        (Some("A"), ""),
        (Some("A"), "AA"),
    ] {
        assert!(
            matches!(
                error_is(raised, expected),
                Verdict::Fail {
                    failure_id: "wrong-error",
                    ..
                }
            ),
            "{raised:?} vs {expected:?}"
        );
    }
    assert!(matches!(
        error_is(None, "A"),
        Verdict::Fail {
            failure_id: "no-error",
            ..
        }
    ));
    // "no error expected" is not a thing at this layer: None never passes.
    assert!(matches!(
        error_is(None, ""),
        Verdict::Fail {
            failure_id: "no-error",
            ..
        }
    ));
    // Two empty ids are equal (an empty id is a valid text), by the table.
    assert_eq!(error_is(Some(""), ""), Verdict::Pass);
}

// ---------------------------------------------------------------------------
// Failure vocabulary closure
// ---------------------------------------------------------------------------

const VOCABULARY: [&str; 15] = [
    // TF01 §5.3
    "not-deep-equal",
    "no-error",
    "wrong-error",
    "panic",
    "corpus-mismatch",
    "corpus-malformed",
    // VER05 §5.1
    "property-counterexample",
    "property-exhausted",
    "property-timeout",
    "property-panic",
    // VER06 §6
    "fuzz-crash",
    "fuzz-timeout",
    "fuzz-oom",
    "fuzz-leak",
    "fuzz-panic",
];

#[test]
fn the_vocabulary_is_exactly_the_fifteen_ids_of_the_specs() {
    for id in VOCABULARY {
        assert!(verdict::is_registered_failure_id(id), "{id}");
    }
}

#[test]
fn no_one_edit_neighbour_of_a_registered_id_is_registered() {
    let registered: BTreeSet<&str> = VOCABULARY.into_iter().collect();
    let mut neighbours = 0;
    for id in VOCABULARY {
        let chars: Vec<char> = id.chars().collect();
        let mut candidates: Vec<String> = Vec::new();
        for i in 0..chars.len() {
            // deletion
            let mut deleted = chars.clone();
            deleted.remove(i);
            candidates.push(deleted.into_iter().collect());
            // substitution by a letter, a digit, an underscore, a homoglyph
            for replacement in ['x', 'A', '0', '_', ' ', '\u{0}', '\u{430}'] {
                let mut sub = chars.clone();
                sub[i] = replacement;
                candidates.push(sub.into_iter().collect());
            }
            // insertion
            for inserted in ['-', 'x', ' ', '\n'] {
                let mut ins = chars.clone();
                ins.insert(i, inserted);
                candidates.push(ins.into_iter().collect());
            }
        }
        candidates.push(format!("{id} "));
        candidates.push(format!(" {id}"));
        candidates.push(format!("{id}\n"));
        candidates.push(format!("{id}\0"));
        candidates.push(id.to_uppercase());
        candidates.push(id.replace('-', "_"));
        for candidate in candidates {
            if registered.contains(candidate.as_str()) {
                continue; // an edit that lands on another registered id
            }
            neighbours += 1;
            assert!(
                !verdict::is_registered_failure_id(&candidate),
                "{candidate:?} (near {id}) was accepted"
            );
        }
    }
    assert!(neighbours > 1500, "domain too small: {neighbours}");
}

#[test]
fn the_module_vocabularies_are_subsets_of_the_closed_one() {
    for id in [
        "property-counterexample",
        "property-exhausted",
        "property-timeout",
        "property-panic",
    ] {
        assert!(pbt::is_registered_failure_id(id) && verdict::is_registered_failure_id(id));
    }
    for id in [
        "fuzz-crash",
        "fuzz-timeout",
        "fuzz-oom",
        "fuzz-leak",
        "fuzz-panic",
    ] {
        assert!(
            coding_adventures_test_bootstrap::fuzz::is_registered_failure_id(id)
                && verdict::is_registered_failure_id(id)
        );
    }
    // The module predicates do not accept the L0 ids (they are separate sets).
    assert!(!pbt::is_registered_failure_id("panic"));
    assert!(!coding_adventures_test_bootstrap::fuzz::is_registered_failure_id("panic"));
}

// ---------------------------------------------------------------------------
// Runner: every failing verdict is "not ok", counted, and exits non-zero
// ---------------------------------------------------------------------------

#[test]
fn each_registered_failure_id_is_reported_not_ok_with_exit_one() {
    for id in VOCABULARY {
        let mut suite = Suite::new();
        suite.add("only", move |_| Verdict::fail(id, "detail"));
        let (tap, summary) = run_suite(&suite, &Options::default());
        assert_eq!(
            summary,
            Summary {
                passed: 0,
                failed: 1,
                bailed: false
            },
            "{id}"
        );
        assert_eq!(summary.exit_code(), 1);
        assert_eq!(not_ok_count(&tap), 1);
        assert_eq!(ok_count(&tap), 0);
        assert!(tap.contains(&format!("  failure_id: \"{id}\"\n")));
    }
}

#[test]
fn every_failing_core_path_through_a_suite_is_not_ok() {
    for (a, b) in unequal_pairs() {
        let (x, y) = (text_of(a), text_of(b));
        let mut suite = Suite::new();
        suite.add("neq", move |ctx| ctx.deep_equal(&x, &y));
        let (tap, summary) = run_suite(&suite, &Options::default());
        assert_eq!((summary.passed, summary.failed), (0, 1), "{a} vs {b}");
        assert_eq!(summary.exit_code(), 1);
        assert!(tap.contains("not ok 1 - neq\n"));
        assert!(tap.contains("  failure_id: \"not-deep-equal\"\n"));
    }
    for (raised, expected, id) in [(None, "E", "no-error"), (Some("F"), "E", "wrong-error")] {
        let mut suite = Suite::new();
        suite.add("err", move |ctx| ctx.error_is(raised, expected));
        let (tap, summary) = run_suite(&suite, &Options::default());
        assert_eq!((summary.passed, summary.failed), (0, 1));
        assert!(tap.contains(&format!("  failure_id: \"{id}\"\n")));
    }
}

#[test]
fn panics_of_every_shape_are_failures_never_passes() {
    use std::panic::panic_any;
    let mut suite = Suite::new();
    suite.add("p/str", |_| panic!("a static message"));
    suite.add("p/string", |_| panic!("{}", String::from("formatted")));
    suite.add("p/any-int", |_| panic_any(42u32));
    suite.add("p/unwrap-none", |_| {
        let none: Option<u8> = std::hint::black_box(None);
        none.unwrap();
        Verdict::Pass
    });
    suite.add("p/assert", |_| {
        assert_eq!(1 + 1, 3);
        Verdict::Pass
    });
    suite.add("p/index", |_| {
        let v: Vec<u8> = Vec::new();
        let i = std::hint::black_box(3usize);
        let _ = v[i];
        Verdict::Pass
    });
    suite.add("p/after-a-check", |ctx| {
        // A passing check followed by a panic must not leave a pass behind.
        let first = ctx.deep_equal(&Value::Null, &Value::Null);
        assert_eq!(first, Verdict::Pass);
        panic!("late");
    });
    suite.add("p/nested", |_| {
        let _ = std::panic::catch_unwind(|| panic!("inner is swallowed by its own catch"));
        panic!("outer still counts");
    });
    suite.add("z/control-pass", |_| Verdict::Pass);
    let (tap, summary) = run_suite(&suite, &Options::default());
    assert_eq!(
        summary,
        Summary {
            passed: 1,
            failed: 8,
            bailed: false
        }
    );
    assert_eq!(summary.exit_code(), 1);
    assert_eq!(tap.matches("  failure_id: \"panic\"\n").count(), 8);
    assert_eq!(
        not_ok_ids(&tap).into_iter().collect::<Vec<_>>(),
        [
            "p/any-int",
            "p/assert",
            "p/after-a-check",
            "p/index",
            "p/nested",
            "p/str",
            "p/string",
            "p/unwrap-none"
        ]
        .map(String::from)
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );
    // The panic text never reaches TAP (it may quote inputs; VER00 §7.3).
    for leaked in [
        "a static message",
        "formatted",
        "late",
        "outer still counts",
    ] {
        assert!(!tap.contains(leaked), "{leaked:?} leaked into TAP");
    }
}

#[test]
fn a_body_with_zero_checks_passes_by_design() {
    // TF01 §7: L0 has no assertion counting. A body that returns Pass
    // without consulting the core IS a pass. Layers above (Warrant, VER05)
    // generate the checks; this layer reports what it is told. The test
    // pins the behaviour so any change to it is a deliberate spec change.
    let mut suite = Suite::new();
    suite.add("vacuous/no-checks", |_| Verdict::Pass);
    suite.add("vacuous/early-return", |ctx| {
        if ctx.seed() != u64::MAX - 1 {
            return Verdict::Pass; // returns before the check below
        }
        ctx.deep_equal(&Value::Null, &Value::Bool(true))
    });
    let (tap, summary) = run_suite(&suite, &Options::default());
    assert_eq!(
        summary,
        Summary {
            passed: 2,
            failed: 0,
            bailed: false
        }
    );
    assert_eq!(not_ok_count(&tap), 0);
}

#[test]
fn an_empty_suite_is_a_pass_with_a_visible_empty_plan() {
    let (tap, summary) = run_suite(&Suite::new(), &Options::default());
    assert_eq!(summary.exit_code(), 0);
    // The vacuity is visible to a supervisor: plan 1..0.
    assert!(tap.contains("\n1..0\n"));
}

#[test]
fn the_summary_always_adds_up() {
    for n_pass in 0..4usize {
        for n_fail in 0..4usize {
            let mut suite = Suite::new();
            for i in 0..n_pass {
                suite.add(format!("p{i}"), |_| Verdict::Pass);
            }
            for i in 0..n_fail {
                suite.add(format!("f{i}"), |ctx| ctx.error_is(None, "E"));
            }
            let (tap, summary) = run_suite(&suite, &Options::default());
            assert_eq!((summary.passed, summary.failed), (n_pass, n_fail));
            assert_eq!(ok_count(&tap), n_pass);
            assert_eq!(not_ok_count(&tap), n_fail);
            assert!(tap_is_complete(&tap));
            assert_eq!(summary.exit_code(), i32::from(n_fail > 0));
        }
    }
}

#[test]
fn every_test_runs_exactly_once_and_numbers_are_dense() {
    let ids: Vec<String> = (0..30).map(|i| format!("t/{i:02}")).collect();
    for seed in [0u64, 1, 2, 99, u64::MAX] {
        let mut suite = Suite::new();
        for id in &ids {
            suite.add(id.clone(), |_| Verdict::Pass);
        }
        let (tap, _) = run_suite(
            &suite,
            &Options {
                seed,
                sabotage: false,
            },
        );
        let mut seen = Vec::new();
        for (index, line) in tap.lines().filter(|l| l.starts_with("ok ")).enumerate() {
            let (number, id) = line["ok ".len()..].split_once(" - ").unwrap();
            assert_eq!(number, (index + 1).to_string());
            seen.push(id.to_owned());
        }
        // The printed order IS the specified shuffle of the sorted ids.
        assert_eq!(seen, shuffled(seed, &ids));
        seen.sort();
        assert_eq!(seen, ids);
    }
}

// ---------------------------------------------------------------------------
// TAP exactness (golden bytes)
// ---------------------------------------------------------------------------

#[test]
fn tap_golden_output_for_a_mixed_suite() {
    let mut suite = Suite::new();
    suite.add("a", |ctx| {
        ctx.deep_equal(&Value::Bool(true), &Value::Bool(true))
    });
    suite.add("b", |ctx| ctx.error_is(None, "E"));
    // Seed 0 shuffles ["a","b"] to ["a","b"]: the first output of
    // SplitMix64(0) is odd, so bounded(2) = 1 and swap(1, 1) is a no-op.
    let (tap, _) = run_suite(&suite, &Options::default());
    assert_eq!(
        tap,
        "TAP version 14\n\
         # tf-lane: rust\n\
         # tf-seed: 0\n\
         1..2\n\
         ok 1 - a\n\
         not ok 2 - b\n\
         \x20 ---\n\
         \x20 failure_id: \"no-error\"\n\
         \x20 message: \"expected error \\\"E\\\", none raised\"\n\
         \x20 ...\n"
    );
}

#[test]
fn tap_golden_output_for_a_panic_a_bail_and_a_seed() {
    let mut suite = Suite::new();
    suite.add("x", |_| panic!("hidden"));
    let (tap, summary) = run_suite(
        &suite,
        &Options {
            seed: 18446744073709551615,
            sabotage: false,
        },
    );
    assert_eq!(
        tap,
        "TAP version 14\n\
         # tf-lane: rust\n\
         # tf-seed: 18446744073709551615\n\
         1..1\n\
         not ok 1 - x\n\
         \x20 ---\n\
         \x20 failure_id: \"panic\"\n\
         \x20 message: \"the test panicked\"\n\
         \x20 ...\n"
    );
    assert_eq!(summary.exit_code(), 1);

    let mut bad = Suite::new();
    bad.add("a b", |_| Verdict::Pass);
    let (tap, _) = run_suite(&bad, &Options::default());
    assert_eq!(
        tap,
        "TAP version 14\n# tf-lane: rust\n# tf-seed: 0\nBail out! invalid test id \"a b\"\n"
    );
}

#[test]
fn test_id_validation_matches_the_grammar_exhaustively() {
    // Every id of length 0..=2 over an alphabet mixing valid and hostile characters.
    let alphabet = [
        'a', 'Z', '0', '.', '_', '/', '-', ' ', '\n', '#', '\u{e9}', '\r', '\t',
    ];
    let mut ids = vec![String::new()];
    let mut frontier = vec![String::new()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for c in alphabet {
                next.push(format!("{prefix}{c}"));
            }
        }
        ids.extend(next.iter().cloned());
        frontier = next;
    }
    assert_eq!(ids.len(), 1 + 13 + 169);
    let valid = |id: &str| {
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
    };
    for id in &ids {
        let mut suite = Suite::new();
        suite.add(id.clone(), |_| Verdict::Pass);
        let (tap, summary) = run_suite(&suite, &Options::default());
        assert_eq!(summary.bailed, !valid(id), "{id:?}");
        if valid(id) {
            assert!(tap.contains(&format!("ok 1 - {id}\n")));
        } else {
            // A bad id never produces a result line, and its bytes cannot
            // add a line to the output: exactly one `Bail out!` line.
            assert_eq!(tap.lines().count(), 4, "{id:?}: {tap:?}");
            assert!(!tap
                .lines()
                .any(|l| l.starts_with("ok ") || l.starts_with("not ok ")));
        }
    }
}

#[test]
fn no_message_character_can_forge_a_tap_line() {
    // Every char U+0000..=U+2FFF in the middle of a failure message.
    for code in 0u32..0x3000 {
        let Some(c) = char::from_u32(code) else {
            continue;
        };
        let message = format!("a{c}ok 1 - forged{c}b");
        let mut suite = Suite::new();
        suite.add("m", move |_| Verdict::fail("no-error", message.clone()));
        let (tap, summary) = run_suite(&suite, &Options::default());
        assert_eq!((summary.passed, summary.failed), (0, 1), "U+{code:04X}");
        let lines: Vec<&str> = tap.split('\n').collect();
        // header(4) + not ok + --- + failure_id + message + ... + trailing "".
        assert_eq!(
            lines.len(),
            10,
            "U+{code:04X} changed the line structure: {tap:?}"
        );
        assert!(lines[7].starts_with("  message: \"") && lines[7].ends_with('"'));
        assert_eq!(ok_count(&tap), 0, "U+{code:04X} forged an ok line");
        assert!(!lines
            .iter()
            .any(|l| l.contains('\r') || l.contains('\u{2028}')));
    }
}

// ---------------------------------------------------------------------------
// Sabotage semantics: what must fail, and how
// ---------------------------------------------------------------------------

#[test]
fn sabotage_turns_every_core_verdict_into_pass_but_nothing_else() {
    let sabotage = Options {
        seed: 0,
        sabotage: true,
    };
    // 1. Every unequal pair now "passes" through Ctx...
    let mut suite = Suite::new();
    for (i, (a, b)) in unequal_pairs().into_iter().enumerate() {
        let (x, y) = (text_of(a), text_of(b));
        suite.add(format!("neq/{i:02}"), move |ctx| ctx.deep_equal(&x, &y));
    }
    suite.add("err/none", |ctx| ctx.error_is(None, "E"));
    suite.add("err/wrong", |ctx| ctx.error_is(Some("X"), "E"));
    let (tap, summary) = run_suite(&suite, &sabotage);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.exit_code(), 0);
    assert_eq!(not_ok_count(&tap), 0);
    // ...and without sabotage, every one fails (the control).
    let (_, control) = run_suite(&suite, &Options::default());
    assert_eq!(control.passed, 0);
    assert_eq!(control.failed, unequal_pairs().len() + 2);

    // 2. Sabotage does NOT hide panics, hand-built failures, or bad ids.
    let mut rest = Suite::new();
    rest.add("panics", |_| panic!("still a panic"));
    rest.add("direct-fail", |_| {
        Verdict::fail("corpus-mismatch", "constructed directly")
    });
    let (_, summary) = run_suite(&rest, &sabotage);
    assert_eq!((summary.passed, summary.failed), (0, 2));
    let mut bad = Suite::new();
    bad.add("bad", |_| Verdict::Fail {
        failure_id: "nope",
        message: String::new(),
    });
    let (tap, summary) = run_suite(&bad, &sabotage);
    assert!(summary.bailed);
    assert!(tap.contains("Bail out! invalid failure id"));

    // 3. The pure functions in `verdict` are not sabotaged (only Ctx is).
    assert!(matches!(
        deep_equal(&Value::Null, &Value::Bool(true)),
        Verdict::Fail { .. }
    ));
}

#[test]
fn sabotaged_self_corpus_fails_exactly_the_expected_fail_cases() {
    // TF01 §6: under `--sabotage always-pass`, the corpus's expected-`fail`
    // core cases must be reported `not ok`, and nothing else may be.
    let corpus = include_str!("../../../../specs/fixtures/test-bootstrap-v1/cases.tfv");
    let records = tfv::parse_file(corpus).expect("corpus parses");
    let mut expected: BTreeSet<String> = BTreeSet::new();
    let mut core_cases = 0;
    for record in &records {
        let op = record.field("op").and_then(Value::as_text).unwrap();
        if matches!(op, "deep_equal" | "error_is") {
            core_cases += 1;
            if record.field("verdict").and_then(Value::as_text) == Some("fail") {
                expected.insert(record.id.clone());
            }
        }
    }
    assert!(
        expected.len() >= 10,
        "the corpus must carry real fail cases"
    );
    assert!(core_cases > expected.len(), "and real pass cases");

    let binary = sibling_binary("self_corpus");
    let sabotaged = Command::new(&binary)
        .args(["--sabotage", "always-pass"])
        .output()
        .expect("spawn self_corpus");
    let tap = stdout_of(&sabotaged);
    assert_eq!(
        sabotaged.status.code(),
        Some(1),
        "sabotaged run must exit 1"
    );
    assert_eq!(not_ok_ids(&tap), expected);
    assert!(tap_is_complete(&tap));
    // Every sabotage failure is a corpus disagreement, nothing else.
    assert_eq!(
        tap.matches("  failure_id: \"corpus-mismatch\"\n").count(),
        expected.len()
    );

    let honest = Command::new(&binary).output().expect("spawn self_corpus");
    assert_eq!(honest.status.code(), Some(0));
    let honest_tap = stdout_of(&honest);
    assert_eq!(not_ok_count(&honest_tap), 0);
    assert_eq!(ok_count(&honest_tap), records.len());
}

// ---------------------------------------------------------------------------
// Real processes: exit statuses
// ---------------------------------------------------------------------------

#[test]
fn exit_status_zero_only_for_a_clean_run() {
    let pass = spawn_fixture("pass", &[]);
    assert_eq!(pass.status.code(), Some(0));
    let tap = stdout_of(&pass);
    assert!(tap_is_complete(&tap));
    assert_eq!(ok_count(&tap), 2);

    let empty = spawn_fixture("empty", &[]);
    assert_eq!(empty.status.code(), Some(0));
    assert!(stdout_of(&empty).contains("1..0"));

    let vacuous = spawn_fixture("vacuous", &[]);
    assert_eq!(
        vacuous.status.code(),
        Some(0),
        "a vacuous pass is a pass (documented)"
    );
}

#[test]
fn exit_status_one_for_failure_panic_and_bailout() {
    for (mode, want_not_ok) in [
        ("fail", 1),
        ("panic", 1),
        ("bad-failure-id", 0),
        ("duplicate", 0),
    ] {
        let out = spawn_fixture(mode, &[]);
        assert_eq!(out.status.code(), Some(1), "{mode}");
        let tap = stdout_of(&out);
        assert_eq!(not_ok_count(&tap), want_not_ok, "{mode}");
        assert!(!tap_is_complete(&tap) || want_not_ok > 0, "{mode}");
    }
    let forged = stdout_of(&spawn_fixture("bad-failure-id", &[]));
    assert!(forged.ends_with("Bail out! invalid failure id\n"));
    assert!(!forged.contains("made-up"));
}

#[test]
fn exit_status_two_for_a_command_line_it_does_not_understand() {
    for args in [
        &["--bogus"][..],
        &["--seed"],
        &["--seed", "x"],
        &["--seed", "-1"],
        &["--seed", "+1"],
        &["--seed", "18446744073709551616"],
        &["--sabotage"],
        &["--sabotage", "never-pass"],
        &["positional"],
        &["--seed", "1", "--oops"],
    ] {
        let out = spawn_fixture("pass", args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        // Nothing is run, nothing is reported on stdout.
        assert!(out.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("usage:"), "{args:?}: {stderr}");
    }
}

#[test]
fn a_usage_error_beats_a_failing_suite() {
    // A failing suite with a bad argument still exits 2 and prints no verdicts.
    let out = spawn_fixture("fail", &["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn accepted_arguments_reach_the_run() {
    let out = spawn_fixture("pass", &["--seed", "007", "--nocapture"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_of(&out).contains("# tf-seed: 7\n"));
    // The last --seed wins.
    let out = spawn_fixture("pass", &["--seed", "1", "--seed", "2"]);
    assert!(stdout_of(&out).contains("# tf-seed: 2\n"));
    // Sabotage through the real process: a failing fixture now exits 0.
    let out = spawn_fixture("fail", &["--sabotage", "always-pass"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn parse_args_accepts_exactly_the_documented_forms() {
    let none: [&str; 0] = [];
    assert_eq!(parse_args(none), Ok(Options::default()));
    assert_eq!(parse_args(["--seed", "0"]).unwrap().seed, 0);
    assert_eq!(
        parse_args(["--seed", "00000000000000000000"]).unwrap().seed,
        0
    );
    assert!(parse_args(["--seed", "18446744073709551615"]).is_ok());
    assert!(parse_args(["--seed", "18446744073709551616"]).is_err());
    assert!(parse_args(["--seed", "1 "]).is_err());
    assert!(parse_args(["--seed", "\u{661}"]).is_err());
    assert!(parse_args(["--SEED", "1"]).is_err());
    assert!(parse_args(["--seed=1"]).is_err());
    assert!(parse_args(["--sabotage", "ALWAYS-PASS"]).is_err());
}

// Linux only: /dev/full is a Linux device (macOS has none). It exercises the
// "write fails with ENOSPC" path. The EBADF paths (closed and read-only fd 1),
// which Rust's std would otherwise treat as a successful write, are covered
// for every Unix by the two tests after it (LH-003c). The closed-pipe test
// further below covers a failed report on every platform.
#[cfg(target_os = "linux")]
#[test]
fn a_report_that_cannot_be_written_is_not_a_pass() {
    // /dev/full accepts open() and fails every write with ENOSPC.
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else {
        panic!("/dev/full must exist on this platform");
    };
    let out = Command::new(sibling_binary("exit_fixture"))
        .env("TFB_FIXTURE", "pass")
        .stdout(full)
        .status()
        .expect("spawn");
    assert_eq!(out.code(), Some(1));
}

// LH-003c. Rust's std treats EBADF on stdout as a successful write, so without
// an explicit check a harness whose fd 1 is closed or read-only prints nothing
// and exits 0: a false pass. Both cases must exit 1 and say why on stderr.
//
// A closed fd 1 is only visible to a pre-`main` constructor (std re-opens it
// as /dev/null before `main`), so this test runs exactly where the runner
// installs one: Linux, Android and macOS. Other Unix systems are a
// documented known limit (TF01 section 8.3).
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
#[test]
fn a_closed_stdout_exits_one_not_zero() {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    let mut command = Command::new(sibling_binary("exit_fixture"));
    command
        .env("TFB_FIXTURE", "pass")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // SAFETY: runs in the forked child before exec; it calls only `close`, an
    // async-signal-safe function, and allocates nothing.
    unsafe {
        command.pre_exec(|| {
            close(1);
            Ok(())
        });
    }
    let out = command.output().expect("spawn");
    assert_eq!(out.status.code(), Some(1), "closed fd 1 must not pass");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("standard output is not writable") && stderr.contains("closed"),
        "{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn a_read_only_stdout_exits_one_not_zero() {
    use std::process::Stdio;
    // /dev/null opened read-only: a write to it fails with EBADF.
    let read_only = std::fs::File::open("/dev/null").expect("open /dev/null");
    let out = Command::new(sibling_binary("exit_fixture"))
        .env("TFB_FIXTURE", "pass")
        .stdout(read_only)
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1), "read-only fd 1 must not pass");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("read-only"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn a_writable_stdout_that_discards_is_still_accepted() {
    // KNOWN LIMIT (TF01 section 8): a stdout that accepts and discards bytes
    // (/dev/null) cannot be told apart from a working one. The run passes;
    // only a supervisor reading the real stream can notice nothing arrived.
    let sink = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    let out = Command::new(sibling_binary("exit_fixture"))
        .env("TFB_FIXTURE", "pass")
        .stdout(sink)
        .status()
        .expect("spawn");
    assert_eq!(out.code(), Some(0));
}

#[test]
fn an_early_process_exit_leaves_a_truncated_report_that_a_supervisor_can_detect() {
    // KNOWN LIMIT (documented in TF01 §8): a test that calls process::exit(0)
    // ends the process with status 0 and never prints its own result. The
    // harness cannot prevent that from inside the process. What it does
    // provide is the up-front plan `1..N`, so a supervisor comparing the plan
    // with the result lines sees the run is incomplete.
    let out = spawn_fixture("exit-zero", &[]);
    let tap = stdout_of(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "exit status alone cannot see this"
    );
    assert!(tap.contains("1..2\n"));
    assert!(
        !tap_is_complete(&tap),
        "a supervisor must treat this as a failure: {tap}"
    );
}

#[test]
fn a_closed_pipe_mid_report_is_not_a_pass() {
    // Writer that accepts the header then fails, as `| head -n1` would.
    struct FailsAfter(usize);
    impl Write for FailsAfter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0 == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "closed",
                ));
            }
            let n = self.0.min(bytes.len());
            self.0 -= n;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut suite = Suite::new();
    suite.add("a", |_| Verdict::Pass);
    suite.add("b", |_| Verdict::Pass);
    for budget in 0..60 {
        let result = suite.run(&Options::default(), &mut FailsAfter(budget));
        assert!(
            result.is_err(),
            "budget {budget}: a truncated report returned Ok"
        );
    }
    let _ = run::LANE;
}
