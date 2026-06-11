//! CLOC14 end-to-end byte-identity test harness.
//!
//! ## Why this exists
//!
//! Until this harness, every CLOC12 gap-fix PR was theoretical —
//! we'd patch a fold rule, the unit tests would go green, but we
//! had no measurement of whether the output of `closurec` actually
//! matches what Google's Closure Compiler emits. The termination
//! condition of this project ("drop-in binary-compatible
//! closurec") is a behavioural property, not a feature checklist;
//! without an end-to-end test that *measures* byte-divergence, we
//! can ship correct-in-isolation passes that compose into a
//! diverging compiler.
//!
//! ## How this harness differs from the existing `diff_*.rs` files
//!
//! The legacy `diff_<flag>.rs` files each test ONE CLI flag's
//! shape (`--charset`, `--output_wrapper`, etc.). They're
//! flag-shaped: each fixture exercises a flag, not the
//! optimization pipeline.
//!
//! CLOC14's fixtures live under `tests/diff/minify_<name>/` and
//! exercise the optimization pipeline end-to-end: an input JS
//! file goes through `closurec`'s actual `--compilation_level`
//! pipeline, and the stdout is compared against the output that
//! Google Closure Compiler produces on the same input with the
//! same flags. A failing minify fixture is a *real* divergence
//! that ships incorrect behaviour to users.
//!
//! ## Discovery + single-runner design
//!
//! Rather than one `diff_minify_<name>.rs` per fixture (which
//! creates linear boilerplate growth), this single runner walks
//! `tests/diff/minify_*/` at test time and executes every
//! fixture. Failures are collected per-fixture and reported
//! together — `cargo test diff_minify` lists every divergent
//! fixture, not just the first failure.
//!
//! ## Fixture format
//!
//! ```text
//! tests/diff/minify_<name>/
//! ├── flags.txt          # one CLI flag per line
//! ├── input/             # input files referenced by flags.txt
//! │   └── a.js
//! ├── expected.stdout    # the expected stdout, captured from
//! │                      # Google Closure Compiler on the same
//! │                      # input + flags
//! └── README.md          # (optional) what this fixture pins
//!                        # and where the golden was captured
//! ```
//!
//! ## Status of each fixture
//!
//! Each fixture's `README.md` should document:
//!   1. The Google Closure Compiler version that produced the
//!      golden (e.g. `v20240317`).
//!   2. The exact command line used to capture the golden.
//!   3. Any caveats — e.g. "expected to fail until gap-014
//!      lands" (mark such tests with `#[ignore]` in the
//!      `IGNORE_FIXTURES` list below).
//!
//! ## Authoring a new fixture
//!
//! 1. Pick the smallest input that exercises the behaviour.
//! 2. Run upstream Closure to capture the golden:
//!    ```
//!    java -jar closure-compiler.jar \
//!        --compilation_level WHITESPACE_ONLY \
//!        --js input/a.js > expected.stdout
//!    ```
//! 3. Add the fixture directory. The next `cargo test
//!    diff_minify_walk` run picks it up automatically.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Fixtures intentionally left failing — usually because they pin
/// a behaviour we KNOW we don't yet match (e.g. a CLOC12 gap that
/// hasn't shipped). Listing the fixture here documents the gap
/// while keeping CI green; the fixture is still useful as a
/// future-target.
///
/// Format: fixture name (without the `minify_` prefix) → reason.
const IGNORE_FIXTURES: &[(&str, &str)] = &[
    // gap-044: JavaScript lexer does not yet support
    // template literal SUBSTITUTIONS (`${expr}` inside
    // a backtick-delimited string). Lexer-level gap.
    ("template_subst", "gap-044: lexer does not support `${...}`"),
    ("tagged_subst",   "gap-044: lexer does not support `${...}` (tagged variant)"),
    // gap-055/056/057 all RESOLVED (CLOC12.64/65/66) — ternary
    // arms, return/throw/=> prefixes, and member-object parens
    // (`(a).b` → `a.b`) now pass and are no longer ignored.
    // gap-053 was RESOLVED in CLOC12.62 — token-stream
    // pre-pass strips outer `(` `)` around `= ( ... ) ;`
    // when contents have no top-level `,` and don't start
    // with `function`. `minify_null_undef_compare` flipped
    // IGNORED → PASS.
    // gap-054 was RESOLVED in CLOC12.63 — token-stream
    // pre-pass strips parens around single-token operand
    // of `void`/`typeof`/`delete`. `minify_void_zero_call`
    // flipped IGNORED → PASS.
    // gap-051 was RESOLVED in CLOC12.60 — token-stream
    // pre-pass reorders `} ( ) )` to `} ) ( )`. IIFE
    // inner-call form `(fn(){...}())` normalizes to
    // outer-call form `(fn(){...})()`. `minify_fn_expr_iife`
    // flipped IGNORED → PASS.
    // gap-052 was RESOLVED in CLOC12.61 — `BlockKind::Other`
    // at EOF now wants `;`. `minify_labeled_block` and
    // `minify_double_break_continue` flipped IGNORED → PASS.
    // gap-050 was RESOLVED in CLOC12.57 — token-level
    // peephole drops the empty `()` after `new IDENT`
    // when the follower is safe. `minify_new_expr`
    // flipped IGNORED → PASS.
    // gap-048 was RESOLVED in CLOC12.55 — BigInt token
    // path now strips ES2021 `_` numeric separators.
    // `minify_bigint_separator` flipped IGNORED → PASS.
    // gap-049 was RESOLVED in CLOC12.56 — gap-032's
    // flatten now peeks the token after the closing `}`;
    // if it's another `}`, the trailing `;` is suppressed
    // from the inline emission. `minify_for_await_of`
    // flipped IGNORED → PASS.
    // gap-046 was RESOLVED in CLOC12.52 — `,` immediately
    // before `]` is now suppressed. `minify_trailing_array_comma`
    // flipped IGNORED → PASS. Object-literal trailing comma
    // (gap-046b) deferred.
    // gap-047 was RESOLVED in CLOC12.53 — `}` handler now
    // adds a 5th branch in its decision: when next non-trivia
    // is a statement-starting keyword, no synthetic `;` is
    // emitted (ASI covers the boundary). `minify_multi_line_func`
    // flipped IGNORED → PASS.
];

/// Walk `tests/diff/` and collect every directory whose name
/// starts with `minify_`. The discovery is at test time so adding
/// a new fixture only requires creating the directory — no source
/// file changes.
fn discover_fixtures() -> Vec<String> {
    let diff_root = Path::new("tests/diff");
    let mut fixtures: Vec<String> = std::fs::read_dir(diff_root)
        .expect("read tests/diff/")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|ty| ty.is_dir())
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("minify_"))
        .collect();
    fixtures.sort();
    fixtures
}

/// Run one fixture: load flags.txt, exec closurec, capture stdout.
/// Returns the closurec stdout on success, or an error string
/// describing what went wrong.
fn run_fixture(fixture: &str) -> Result<String, String> {
    let flags_path = format!("tests/diff/{fixture}/flags.txt");
    let raw = std::fs::read_to_string(&flags_path)
        .map_err(|e| format!("read {flags_path}: {e}"))?;
    let flags: Vec<String> = raw
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| s.to_string())
        .collect();

    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .map_err(|e| format!("spawn closurec: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "closurec exited {:?}; stderr:\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("non-UTF-8 stdout: {e}"))
}

/// Comparison verdict for a fixture.
enum Verdict {
    Match,
    Diverge { actual: String, expected: String },
    Error(String),
    Skipped(String),
}

fn check_fixture(fixture: &str) -> Verdict {
    let bare = fixture.strip_prefix("minify_").unwrap_or(fixture);
    if let Some((_, reason)) = IGNORE_FIXTURES.iter().find(|(n, _)| *n == bare) {
        return Verdict::Skipped(reason.to_string());
    }

    let actual = match run_fixture(fixture) {
        Ok(s) => s,
        Err(e) => return Verdict::Error(e),
    };
    let expected_path = format!("tests/diff/{fixture}/expected.stdout");
    let expected = match std::fs::read_to_string(&expected_path) {
        Ok(s) => s,
        Err(e) => return Verdict::Error(format!("read {expected_path}: {e}")),
    };
    if actual == expected {
        Verdict::Match
    } else {
        Verdict::Diverge { actual, expected }
    }
}

/// Render a small diff banner showing the first divergent line.
/// Avoids dragging in the `similar` / `diff` crate just for tests
/// — line-by-line is enough for human-readable output.
fn first_diverging_line(actual: &str, expected: &str) -> String {
    let mut a = actual.lines();
    let mut e = expected.lines();
    let mut idx = 0usize;
    loop {
        idx += 1;
        match (a.next(), e.next()) {
            (Some(la), Some(le)) if la == le => continue,
            (Some(la), Some(le)) => {
                return format!(
                    "line {idx}:\n  actual:   {la:?}\n  expected: {le:?}"
                );
            }
            (Some(la), None) => {
                return format!("line {idx}: actual has extra:\n  {la:?}");
            }
            (None, Some(le)) => {
                return format!("line {idx}: expected has extra:\n  {le:?}");
            }
            (None, None) => return "(no line-level divergence — likely a trailing-byte difference)".to_string(),
        }
    }
}

/// The single test entry point. Walks all `minify_*` fixtures,
/// collects per-fixture verdicts, and asserts that every
/// non-ignored fixture matched.
///
/// **Why one test rather than one-per-fixture:** test discovery
/// happens at *runtime* not compile-time, so we can't generate a
/// `#[test]` per fixture without macros or build scripts. The
/// single-test design keeps the runner self-contained and reports
/// every failure in one shot.
#[test]
fn diff_minify_all_fixtures() {
    let fixtures = discover_fixtures();
    if fixtures.is_empty() {
        // Nothing under tests/diff/minify_*/ yet — that's not a
        // failure, but flag it so the next contributor sees the
        // empty state.
        eprintln!(
            "diff_minify: no fixtures discovered under tests/diff/minify_*/. \
             Add one via the format documented at the top of this file."
        );
        return;
    }

    let mut failures: Vec<(String, String)> = Vec::new();
    let mut matched = 0usize;
    let mut skipped: Vec<(String, String)> = Vec::new();

    for fixture in &fixtures {
        match check_fixture(fixture) {
            Verdict::Match => matched += 1,
            Verdict::Diverge { actual, expected } => {
                failures.push((
                    fixture.clone(),
                    first_diverging_line(&actual, &expected),
                ));
            }
            Verdict::Error(e) => {
                failures.push((fixture.clone(), format!("error: {e}")));
            }
            Verdict::Skipped(reason) => {
                skipped.push((fixture.clone(), reason));
            }
        }
    }

    eprintln!(
        "diff_minify: {} matched, {} failed, {} skipped (of {} total)",
        matched,
        failures.len(),
        skipped.len(),
        fixtures.len(),
    );
    for (f, r) in &skipped {
        eprintln!("  SKIP  {f}: {r}");
    }

    if !failures.is_empty() {
        let mut msg = String::from("diff_minify failures:\n");
        for (f, why) in &failures {
            msg.push_str(&format!("\n  ❌ {f}\n     {}\n", why.replace('\n', "\n     ")));
        }
        msg.push_str(&format!(
            "\n{} of {} non-ignored fixtures diverged from Google Closure golden.",
            failures.len(),
            fixtures.len() - skipped.len(),
        ));
        panic!("{msg}");
    }
}
