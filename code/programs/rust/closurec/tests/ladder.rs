//! CCR-047 — the differential complexity ladder.
//!
//! # Why this exists when 462 golden fixtures already pass
//!
//! The `minify_*` corpus reports 100% agreement against the pinned oracle, and
//! it is not lying — but it measures the wrong thing. Its fixtures were
//! captured from behaviour `closurec` already implemented, so it proves we have
//! not regressed. It cannot tell us what we never supported, because nothing in
//! it was chosen to find the edge.
//!
//! A deliberately ordered ladder does. Running the same pinned oracle over 52
//! rungs ramped from trivial to hard found agreement of 49/52 at
//! `WHITESPACE_ONLY`, 31/52 at `SIMPLE`, and 13/52 at `ADVANCED` — against a
//! corpus simultaneously reporting 100%.
//!
//! # The ordering is the point
//!
//! Every rung is simpler than the rungs after it. A failure is therefore
//! attributable to the simplest construct that produces it, and the ladder is
//! worked **bottom-up**: a tier 3 failure is a reason to fix tier 3, not to
//! move on to tier 4. Without that ordering a differential corpus is just a
//! pile, and the temptation is to work whichever gap looks most interesting
//! rather than whichever is most foundational.
//!
//! # `expected.stdout` is upstream's output, not ours
//!
//! That is the whole design. These fixtures measure **parity**, so the
//! committed bytes are what Closure produced. A rung where we differ is not
//! quietly re-baselined to our output — it is recorded in
//! `tests/ladder/divergences.json` with the reason and a tracking issue.
//!
//! The ledger pins our *current* output too, which gives it two properties:
//!
//!   * a gap cannot silently widen — if our output changes, this fails; and
//!   * a gap cannot be silently closed — if we start matching upstream, this
//!     also fails, and that failure is the signal to delete the entry.
//!
//! So the known-gap list is machine-checked rather than prose, and it can only
//! shrink deliberately.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const LEDGER: &str = "tests/ladder/divergences.json";

/// One recorded divergence: what upstream produces, and what we produce today.
#[derive(Debug)]
struct Divergence {
    upstream_stdout: String,
    closurec_stdout: String,
    closurec_exit: i32,
    issue: String,
    /// Cross-checked against the fixture-name suffix, so a copy-pasted entry
    /// cannot describe the wrong rung as the ledger grows.
    level: String,
    reason: String,
}

fn load_ledger() -> BTreeMap<String, Divergence> {
    let raw = std::fs::read_to_string(LEDGER).expect("read divergence ledger");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("ledger is json");
    let entries = doc
        .get("divergences")
        .and_then(|d| d.as_object())
        .expect("ledger has a `divergences` object");
    entries
        .iter()
        .map(|(fixture, body)| {
            let s = |k: &str| {
                body.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| panic!("{fixture}: ledger entry missing string `{k}`"))
                    .to_string()
            };
            (
                fixture.clone(),
                Divergence {
                    upstream_stdout: s("upstream_stdout"),
                    closurec_stdout: s("closurec_stdout"),
                    closurec_exit: i32::try_from(
                        body.get("closurec_exit")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| {
                                panic!("{fixture}: ledger entry missing `closurec_exit`")
                            }),
                    )
                    .expect("closurec_exit fits in i32"),
                    issue: s("issue"),
                    level: s("level"),
                    reason: s("reason"),
                },
            )
        })
        .collect()
}

/// Every `tests/diff/ladder_*` directory, sorted so failures read in ladder
/// order rather than filesystem order.
fn ladder_fixtures() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir("tests/diff")
        .expect("read tests/diff")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("ladder_"))
        })
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no ladder_* fixtures found");
    found
}

fn fixture_name(dir: &Path) -> String {
    dir.file_name().unwrap().to_string_lossy().to_string()
}

/// The flags file is one argv element per nonblank line, matching the
/// `closure-flags-file-v1` command in `tests/oracle/manifest.json`. We
/// translate only the compilation level, because upstream spells the levels
/// `SIMPLE_OPTIMIZATIONS` / `ADVANCED_OPTIMIZATIONS` and `closurec` accepts the
/// short forms.
fn closurec_args(dir: &Path) -> Vec<String> {
    let name = fixture_name(dir);
    let raw = std::fs::read_to_string(dir.join("flags.txt")).expect("read flags.txt");
    let args: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| match l {
            "SIMPLE_OPTIMIZATIONS" => "SIMPLE".to_string(),
            "ADVANCED_OPTIMIZATIONS" => "ADVANCED".to_string(),
            other => other.to_string(),
        })
        .collect();

    // A rung must be exactly the reviewed two-flag shape, compiling its OWN
    // input at the level its name claims. Passing `flags.txt` through unchecked
    // would let a rung be silently rewired — pointed at another rung's input,
    // where it might start matching upstream and the harness would then demand
    // its ledger entry be deleted. That is a green build measuring nothing.
    // It also keeps argv closed: no `--js_output_file`, no `--create_source_map`,
    // nothing that writes.
    let level = match name.rsplit('_').next() {
        Some("ws") => "WHITESPACE_ONLY",
        Some("simple") => "SIMPLE",
        Some("advanced") => "ADVANCED",
        other => panic!("{name}: fixture name must end in _ws/_simple/_advanced, got {other:?}"),
    };
    let want = [
        "--compilation_level".to_string(),
        level.to_string(),
        "--js".to_string(),
        format!("tests/diff/{name}/input/a.js"),
    ];
    assert_eq!(
        args, want,
        "{name}: flags.txt must be the reviewed two-flag shape for this rung"
    );
    args
}

/// Returns raw bytes, not a lossy string. `String::from_utf8_lossy` maps every
/// invalid sequence to U+FFFD, so two outputs differing only in invalid UTF-8
/// would compare equal and an encoding regression would go green — and the
/// corpus carries `diff_charset` and `diff_control_chars` fixtures precisely
/// because encoding fidelity is a live concern here. The sibling harness
/// `tests/diff_minify.rs` is strict for the same reason.
fn run_closurec(dir: &Path) -> (i32, Vec<u8>) {
    let out = Command::new(BINARY)
        .args(closurec_args(dir))
        .output()
        .expect("run closurec");
    (out.status.code().unwrap_or(-1), out.stdout)
}

/// What the gate concluded about one rung.
///
/// Factored out of the test loop so the failure paths can be exercised
/// directly. A gate whose failure branches are never executed is a gate nobody
/// has checked: an inverted condition or a branch that forgets to record a
/// failure would leave the suite green while measuring nothing.
#[derive(Debug, PartialEq)]
enum Verdict {
    /// Byte-identical to upstream, and not expected to diverge.
    Matches,
    /// Diverges exactly as the ledger records.
    KnownDivergence,
    /// Anything else. The string is the operator-facing explanation.
    Fail(String),
}

fn verdict(
    name: &str,
    expected: &[u8],
    actual: &[u8],
    code: i32,
    known: Option<&Divergence>,
) -> Verdict {
    let agrees = actual == expected && code == 0;
    match known {
        None if agrees => Verdict::Matches,
        None => Verdict::Fail(format!(
            "{name}: diverged from upstream with no ledger entry.\n    \
             upstream: {:?}\n    ours (exit {code}): {:?}\n    \
             If this is a real regression, fix it. If upstream genuinely differs, \
             add an entry to {LEDGER} with a tracking issue.",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(actual),
        )),
        Some(k) if k.upstream_stdout.as_bytes() != expected => Verdict::Fail(format!(
            "{name}: ledger's upstream_stdout disagrees with expected.stdout.\n    \
             ledger:  {:?}\n    fixture: {:?}",
            k.upstream_stdout,
            String::from_utf8_lossy(expected),
        )),
        Some(k) if agrees => Verdict::Fail(format!(
            "{name}: NOW MATCHES UPSTREAM — delete its entry from {LEDGER}.\n    \
             This gap was tracked in {}. Closing it is the good outcome; this failure \
             exists so the ledger cannot quietly keep stale entries.",
            k.issue
        )),
        Some(k) if actual != k.closurec_stdout.as_bytes() || code != k.closurec_exit => {
            Verdict::Fail(format!(
                "{name}: our output changed but still does not match upstream.\n    \
                 ledger expected ours (exit {}): {:?}\n    actual (exit {code}): {:?}\n    \
                 Tracked in {}. Update the ledger only if this change was intended.",
                k.closurec_exit,
                k.closurec_stdout,
                String::from_utf8_lossy(actual),
                k.issue
            ))
        }
        Some(_) => Verdict::KnownDivergence,
    }
}

/// The gate. Every rung either matches upstream, or is a recorded divergence
/// whose recorded output still holds exactly.
#[test]
fn ladder_matches_upstream_or_a_recorded_divergence() {
    let ledger = load_ledger();
    let mut failures = Vec::new();
    let (mut matched, mut diverged) = (0usize, 0usize);

    for dir in ladder_fixtures() {
        let name = fixture_name(&dir);
        let expected = std::fs::read(dir.join("expected.stdout")).expect("read expected.stdout");
        let (code, actual) = run_closurec(&dir);
        match verdict(&name, &expected, &actual, code, ledger.get(&name)) {
            Verdict::Matches => matched += 1,
            Verdict::KnownDivergence => diverged += 1,
            Verdict::Fail(why) => failures.push(why),
        }
    }

    assert!(
        failures.is_empty(),
        "{} ladder rung(s) failed ({matched} matched, {diverged} known divergences):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// A ledger entry naming a fixture that no longer exists is dead weight that
/// would otherwise never be noticed.
#[test]
fn every_ledger_entry_names_a_real_fixture() {
    let known: Vec<String> = ladder_fixtures().iter().map(|d| fixture_name(d)).collect();
    let orphans: Vec<String> = load_ledger()
        .keys()
        .filter(|f| !known.contains(f))
        .cloned()
        .collect();
    assert!(
        orphans.is_empty(),
        "{LEDGER} names fixtures that do not exist: {orphans:?}"
    );
}

/// Every divergence must say where it is tracked, and describe itself
/// consistently. A gap with no issue is a gap nobody is going to close.
#[test]
fn every_divergence_is_well_formed() {
    for (fixture, d) in load_ledger() {
        let tail = d
            .issue
            .strip_prefix("https://github.com/")
            .and_then(|r| r.split("/issues/").nth(1));
        assert!(
            tail.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
            "{fixture}: issue must be a .../issues/<number> URL, got {:?}",
            d.issue
        );
        assert!(
            !d.reason.trim().is_empty(),
            "{fixture}: divergence must say why it diverges"
        );
        let suffix = match d.level.as_str() {
            "WHITESPACE_ONLY" => "_ws",
            "SIMPLE" => "_simple",
            "ADVANCED" => "_advanced",
            other => panic!("{fixture}: unknown level {other:?}"),
        };
        assert!(
            fixture.ends_with(suffix),
            "{fixture}: ledger says level {} but the fixture name says otherwise",
            d.level
        );
    }
}

/// The ledger is the escape hatch from the gate, so its size is pinned in the
/// same style as the reviewed fixture inventory in `oracle_manifest.rs`.
///
/// Without this, a change that breaks a currently-passing rung can be made
/// green by appending a ledger entry and citing an existing issue — the gate
/// would pass and only human review would catch it. Growing the known-gap list
/// should be a deliberate, reviewer-visible act, so update this number when you
/// mean to, and never to make a build pass.
#[test]
fn reviewed_divergence_count_is_pinned() {
    assert_eq!(
        load_ledger().len(),
        6,
        "reviewed divergence ledger changed — see the note on this test"
    );
}

// ---------------------------------------------------------------------------
// Negative tests: prove the gate's failure paths actually fire.
// ---------------------------------------------------------------------------

fn fake_divergence() -> Divergence {
    Divergence {
        upstream_stdout: "UP".into(),
        closurec_stdout: "OURS".into(),
        closurec_exit: 0,
        issue: "https://github.com/adhithyan15/coding-adventures/issues/15837".into(),
        level: "SIMPLE".into(),
        reason: "fake, for exercising the gate".into(),
    }
}

/// (a) A rung that differs from upstream with nothing recorded must fail.
#[test]
fn gate_fires_on_an_unrecorded_divergence() {
    let v = verdict("rung_simple", b"UP", b"OURS", 0, None);
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("no ledger entry")),
        "expected an unrecorded-divergence failure, got {v:?}"
    );
}

/// (b) A recorded gap that has started matching upstream must fail, so the
/// ledger cannot keep stale entries. This is the good outcome announcing
/// itself.
#[test]
fn gate_fires_when_a_recorded_gap_starts_matching() {
    let v = verdict("rung_simple", b"UP", b"UP", 0, Some(&fake_divergence()));
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("NOW MATCHES UPSTREAM")),
        "expected a now-matching failure, got {v:?}"
    );
}

/// (c) A recorded gap whose output has moved — but still is not upstream's —
/// must fail, so a gap cannot silently widen.
#[test]
fn gate_fires_when_a_recorded_gap_shifts() {
    let v = verdict(
        "rung_simple",
        b"UP",
        b"SOMETHING ELSE",
        0,
        Some(&fake_divergence()),
    );
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("output changed")),
        "expected a shifted-divergence failure, got {v:?}"
    );
    // An unchanged stdout but a changed exit code is also a shift.
    let v = verdict("rung_simple", b"UP", b"OURS", 1, Some(&fake_divergence()));
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("output changed")),
        "expected an exit-code shift to fail, got {v:?}"
    );
}

/// (d) A ledger whose record of upstream has drifted from the fixture bytes
/// must fail, or the two sources of truth diverge unnoticed.
#[test]
fn gate_fires_when_the_ledger_and_fixture_disagree_about_upstream() {
    let v = verdict(
        "rung_simple",
        b"DIFFERENT",
        b"OURS",
        0,
        Some(&fake_divergence()),
    );
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("disagrees with expected.stdout")),
        "expected an upstream-drift failure, got {v:?}"
    );
}

/// The two passing verdicts, so the negative tests above are not vacuously
/// satisfied by a `verdict` that only ever fails.
#[test]
fn gate_accepts_a_match_and_a_faithfully_recorded_divergence() {
    assert_eq!(verdict("r_simple", b"UP", b"UP", 0, None), Verdict::Matches);
    assert_eq!(
        verdict("r_simple", b"UP", b"OURS", 0, Some(&fake_divergence())),
        Verdict::KnownDivergence
    );
}
