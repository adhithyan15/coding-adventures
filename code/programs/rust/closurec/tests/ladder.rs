//! CCR-066 — the differential complexity ladder.
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
    /// Upstream's exit status. Non-zero means upstream **refused this input** —
    /// Closure does not implement the feature — so `expected.stdout` is empty
    /// because there was nothing to emit, not because the program compiles to
    /// nothing. Without this the parity predicate cannot tell those apart.
    upstream_exit: i32,
    /// For rungs where `closurec` itself produces no stdout, the stable prefix
    /// of its first stderr line. Exit code alone says only "it failed"; this
    /// pins *how*, so a regression that moves the failure from one stage to
    /// another cannot stay green.
    closurec_stderr_starts_with: Option<String>,
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
                    upstream_exit: i32::try_from(
                        body.get("upstream_exit")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| {
                                panic!("{fixture}: ledger entry missing `upstream_exit`")
                            }),
                    )
                    .expect("upstream_exit fits in i32"),
                    // Absent is allowed (a rung that produces stdout needs no
                    // stage pin), but present-and-not-a-string is a typo or a
                    // type error. `map` keeps absent as `None` while the panic
                    // inside rejects a present non-string, where the obvious
                    // `.and_then(|v| v.as_str())` would degrade it to `None`
                    // and silently disarm the guard.
                    closurec_stderr_starts_with: body.get("closurec_stderr_starts_with").map(|v| {
                        v.as_str()
                            .unwrap_or_else(|| {
                                panic!("{fixture}: `closurec_stderr_starts_with` must be a string")
                            })
                            .to_string()
                    }),
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
        .filter_map(|e| e.ok())
        // `file_type` is lstat-based and does not follow symlinks, which is what
        // `discover_fixture_directories` in `oracle_manifest.rs` uses. `Path::is_dir`
        // DOES follow them, and that asymmetry is exploitable: a committed symlink
        // `tests/diff/ladder_x_simple -> elsewhere` would be run by this gate as a
        // real rung while staying invisible to the reviewed fixture inventory, so
        // neither the manifest nor the 782 tripwire would move — and the rung's
        // input bytes would live outside the repository, unreviewable in a diff.
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
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

    // `ladder_fixtures` uses lstat so a symlinked rung DIRECTORY cannot slip past
    // the reviewed inventory. That closes the asymmetry one level up; this closes
    // it at the file level, which is where the invariant below actually lives.
    //
    // The argv check pins the `--js` path STRING. It does not pin the bytes at
    // that path. A committed symlink `input/a.js -> ../../ladder_t1_empty_simple/
    // input/a.js` would re-point a hard rung at a trivial program while the
    // manifest, the reviewed inventory and the argv assertion all stay put, and
    // paired with a rewritten `expected.stdout` it goes green while measuring a
    // different program than its name claims. `symlink_metadata` does not follow
    // links, so a symlink is not a regular file and fails here.
    for path in [
        dir.join("flags.txt"),
        dir.join("expected.stdout"),
        dir.join("input").join("a.js"),
    ] {
        let meta =
            std::fs::symlink_metadata(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            meta.is_file(),
            "{}: a rung's inputs must be regular files committed in this repository, \
             not symlinks — otherwise the bytes a rung measures are not the bytes \
             under review",
            path.display()
        );
    }

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
fn run_closurec(dir: &Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(BINARY)
        .args(closurec_args(dir))
        .output()
        .expect("run closurec");
    (
        out.status.code().unwrap_or(-1),
        out.stdout,
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
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
    stderr: &str,
    known: Option<&Divergence>,
) -> Verdict {
    // Parity requires that BOTH compilers succeeded. Where upstream refused the
    // input, `expected` is empty because there was nothing to emit — so an
    // `actual` that is also empty is not agreement, it is two different
    // outcomes that happen to share a byte string. Without this guard a
    // regression to "emit nothing, exit 0" on such a rung would be reported as
    // NOW MATCHES UPSTREAM, and a maintainer following that instruction would
    // delete the entry and leave the rung passing vacuously forever.
    let upstream_succeeded = known.is_none_or(|k| k.upstream_exit == 0);
    let agrees = actual == expected && code == 0 && upstream_succeeded;
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
        // A recorded gap that still produces no stdout must still fail for the
        // same reason. Exit code alone cannot distinguish a parse-stage refusal
        // from a bridge-stage one, so a regression that moves the failure
        // between stages would otherwise stay green as a known divergence.
        Some(k)
            if actual.is_empty()
                && k.closurec_stderr_starts_with
                    .as_deref()
                    .is_some_and(|want| !stderr.starts_with(want)) =>
        {
            Verdict::Fail(format!(
                "{name}: still fails, but differently than recorded.\n    \
                 ledger expected stderr to start: {:?}\n    actual stderr: {:?}\n    \
                 Tracked in {}. Update the ledger only if this change was intended.",
                k.closurec_stderr_starts_with.as_deref().unwrap_or(""),
                stderr.lines().next().unwrap_or(""),
                k.issue
            ))
        }
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
        let (code, actual, stderr) = run_closurec(&dir);
        match verdict(&name, &expected, &actual, code, &stderr, ledger.get(&name)) {
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

/// The rules an entry must satisfy to be written down at all, as a pure
/// predicate returning `Some(complaint)` for a rejection and `None` for an
/// acceptable entry.
///
/// This is deliberately separate from [`verdict`]. `verdict` decides what a
/// recorded entry *means* for one run; these rules decide what may be recorded.
/// A weakness here is the more dangerous of the two, because it does not make a
/// rung report the wrong answer — it makes a rung quietly stop asking the
/// question. Keeping it pure is what lets the rejections be tested; asserting
/// inline would leave every rule exercised only in its passing direction,
/// against a ledger that satisfies it by construction.
fn well_formedness_error(fixture: &str, d: &Divergence) -> Option<String> {
    let tail = d
        .issue
        .strip_prefix("https://github.com/")
        .and_then(|r| r.split("/issues/").nth(1));
    if !tail.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
        return Some(format!(
            "{fixture}: issue must be a .../issues/<number> URL, got {:?}",
            d.issue
        ));
    }
    if d.reason.trim().is_empty() {
        return Some(format!("{fixture}: divergence must say why it diverges"));
    }

    // `upstream_exit` is the only ledger field nothing else corroborates.
    // `upstream_stdout` is cross-checked against the committed `expected.stdout`
    // bytes on every run, but upstream's exit status is recorded in no fixture,
    // so this number is believed on the ledger's own word. And it is
    // load-bearing: `verdict` requires `upstream_exit == 0` before it will
    // report that a rung NOW MATCHES UPSTREAM, so any non-zero value
    // permanently disables staleness detection for that rung — it stops being a
    // parity check against the oracle and becomes a golden of our own output,
    // passing forever so long as we keep producing what we once produced.
    //
    // Pin it to the only state that can justify it, which is the state its own
    // doc comment describes: upstream refused the input, so it emitted nothing.
    if d.upstream_exit != 0 && !d.upstream_stdout.is_empty() {
        return Some(format!(
            "{fixture}: upstream_exit {} says upstream refused this input, but the ledger \
             also records upstream output {:?}. A non-zero upstream_exit stops this rung \
             ever reporting that the gap has closed, so it is legitimate only where \
             upstream emitted nothing.",
            d.upstream_exit, d.upstream_stdout
        ));
    }

    // Our own two fields must agree about what happened. Across all 63 entries
    // there are exactly two shapes: we compiled and emitted something (exit 0,
    // non-empty stdout), or we refused and emitted nothing (non-zero exit,
    // empty stdout). The mixed shapes are the dangerous ones.
    //
    // "exit 0 and no output" in particular is precisely what a maintainer would
    // write down after accepting a regression to "emit nothing, successfully"
    // on a rung upstream refuses — where `expected.stdout` is empty too, so the
    // entry sits one `upstream_exit` edit away from comparing empty against
    // empty forever. `verdict` already refuses to call that a match; rejecting
    // it here means it cannot be recorded in the first place.
    if d.closurec_stdout.is_empty() != (d.closurec_exit != 0) {
        return Some(format!(
            "{fixture}: closurec_exit {} and closurec_stdout {:?} disagree about what \
             happened. Either we compiled it (exit 0, some output) or we refused it \
             (non-zero exit, no output) — a mixed shape is either a mis-recorded entry \
             or a regression being written down as if it were normal.",
            d.closurec_exit, d.closurec_stdout
        ));
    }

    // A rung we refuse otherwise asserts only "it fails somehow". Nothing else
    // requires the stage pin, so deleting the key, nulling it, or setting it to
    // "" would each silently restore that weakness — and `starts_with("")` is
    // vacuously true, so emptiness must be rejected explicitly rather than
    // merely presence checked.
    //
    // Non-emptiness alone is still not enough. The pin exists to say WHICH
    // stage a rung dies at, and every real pin has the shape
    // `"<LEVEL> compilation failed at <stage> stage:"`. A pin truncated before
    // the stage name — `"SIMPLE compilation failed at"`, or just `"S"` — is
    // non-empty, looks plausible in review, and matches every stage's message
    // equally, which is the same vacuous guard one notch along. Requiring the
    // pin to reach `stage:` forces it past the discriminating word.
    //
    // Two things about this rule that are easy to get wrong later.
    //
    // It keys on `closurec_exit != 0`, not on empty stdout, and those are the
    // same set only because the shape rule above already forced
    // `empty <=> non-zero` and returned early. Relaxing the shape rule would
    // silently narrow this one's coverage.
    //
    // And `"...stage:"` is the grammar of `CompilerError::TypedPipeline`, which
    // is the only variant exiting 1. A failure exiting 2 (a `Minify` error, for
    // instance, which is what a WHITESPACE_ONLY rung would fail with) carries no
    // stage at all and so cannot be recorded here. That is fail-closed and fine
    // today, since all 14 recorded failures are SIMPLE/ADVANCED exit-1. If a
    // rung ever fails that way, do NOT relax this back to "non-empty" — that
    // reopens the exact hole it was written to close. Key the requirement on the
    // diagnostic family instead, and demand a pin equally discriminating for
    // that family's message grammar.
    if d.closurec_exit != 0 {
        let pin = d.closurec_stderr_starts_with.as_deref();
        if pin.is_none_or(|p| p.trim().is_empty()) {
            return Some(format!(
                "{fixture}: an entry that records a failure must pin the stage it fails \
                 at via a non-empty `closurec_stderr_starts_with`, or the gate only \
                 asserts that it failed somehow"
            ));
        }
        if !pin.is_some_and(|p| p.trim_end().ends_with("stage:")) {
            return Some(format!(
                "{fixture}: the stage pin must run through the stage name and end at \
                 `stage:`, or it matches every stage alike and the gate is back to \
                 asserting only that it failed somehow, got {pin:?}"
            ));
        }
    }

    let suffix = match d.level.as_str() {
        "WHITESPACE_ONLY" => "_ws",
        "SIMPLE" => "_simple",
        "ADVANCED" => "_advanced",
        other => return Some(format!("{fixture}: unknown level {other:?}")),
    };
    if !fixture.ends_with(suffix) {
        return Some(format!(
            "{fixture}: ledger says level {} but the fixture name says otherwise",
            d.level
        ));
    }
    None
}

/// Every divergence must say where it is tracked, and describe itself
/// consistently. A gap with no issue is a gap nobody is going to close.
#[test]
fn every_divergence_is_well_formed() {
    for (fixture, d) in load_ledger() {
        if let Some(complaint) = well_formedness_error(&fixture, &d) {
            panic!("{complaint}");
        }
    }
}

/// (g) The well-formedness rules are only ever run against a ledger that
/// satisfies them, so on their own they are exercised in the passing direction
/// and nowhere else. These drive each rule into its rejecting direction.
///
/// The `upstream_exit` case is the important one. That field is the only thing
/// in the ledger nothing else corroborates, and setting it non-zero switches a
/// rung's staleness detection off permanently — a one-token edit to a data file
/// that no other test in this suite can see.
#[test]
fn well_formedness_rejects_every_way_of_switching_a_rung_off() {
    let accepted = |d: &Divergence| well_formedness_error("rung_simple", d).is_none();
    let rejected_for = |d: &Divergence, want: &str| {
        let got = well_formedness_error("rung_simple", d);
        assert!(
            got.as_deref().is_some_and(|m| m.contains(want)),
            "expected a rejection mentioning {want:?}, got {got:?}"
        );
    };

    // The baselines must be accepted, or every assertion below is vacuous.
    assert!(accepted(&fake_divergence()), "the plain fake must be legal");
    assert!(
        accepted(&fake_hard_failure()),
        "the hard-failure fake must be legal"
    );

    // upstream_exit: non-zero is legitimate only where upstream emitted nothing,
    // because that is the only story it can tell. Anything else is a rung being
    // exempted from ever reporting that its gap has closed.
    rejected_for(
        &Divergence {
            upstream_exit: 2,
            ..fake_divergence()
        },
        "upstream refused this input",
    );

    // ... and the same value WITH no upstream output is the legitimate shape,
    // so the rule discriminates rather than banning the field outright.
    assert!(
        accepted(&Divergence {
            upstream_exit: 2,
            upstream_stdout: String::new(),
            ..fake_divergence()
        }),
        "a rung upstream genuinely refused must still be recordable"
    );

    // The outcome shape: exit 0 with no output is what accepting an
    // "emit nothing, successfully" regression looks like written down.
    rejected_for(&fake_upstream_declined(), "disagree about what happened");
    rejected_for(
        &Divergence {
            closurec_stdout: "OURS".into(),
            ..fake_hard_failure()
        },
        "disagree about what happened",
    );

    // The stage pin: every way of disarming it must be rejected. `""` matters
    // most — `starts_with("")` is vacuously true, so an empty pin is
    // indistinguishable from no pin at the comparison site.
    for pin in [None, Some(String::new()), Some("   ".to_string())] {
        rejected_for(
            &Divergence {
                closurec_stderr_starts_with: pin.clone(),
                ..fake_hard_failure()
            },
            "must pin the stage it fails at",
        );
    }

    // ... and a pin truncated before the stage name is non-empty, survives the
    // rule above, looks plausible in review, and matches every stage alike.
    for pin in ["S", "SIMPLE compilation failed at", "SIMPLE compilation "] {
        rejected_for(
            &Divergence {
                closurec_stderr_starts_with: Some(pin.to_string()),
                ..fake_hard_failure()
            },
            "must run through the stage name",
        );
    }

    // The pre-existing rules, so the refactor into a pure predicate did not
    // drop one on the way.
    rejected_for(
        &Divergence {
            issue: "https://example.com/nope".into(),
            ..fake_divergence()
        },
        "issues/<number>",
    );
    rejected_for(
        &Divergence {
            reason: "   ".into(),
            ..fake_divergence()
        },
        "must say why it diverges",
    );
    rejected_for(
        &Divergence {
            level: "ADVANCED".into(),
            ..fake_divergence()
        },
        "the fixture name says otherwise",
    );
    rejected_for(
        &Divergence {
            level: "SUPER".into(),
            ..fake_divergence()
        },
        "unknown level",
    );
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
        60,
        "reviewed divergence ledger changed — see the note on this test"
    );
}

/// The rungs recorded as refused by upstream, pinned **by name**.
///
/// Such a rung does not participate in staleness detection: `verdict` can never
/// report that its gap has closed, because there is no upstream behaviour to
/// converge on. That is correct for these three, where Closure v20260915
/// rejects private class elements outright. It is also the cheapest way to
/// switch a rung off entirely.
///
/// Pinning the *count* would not be enough, and the distinction is the whole
/// point of this constant. A count is satisfied by any swap. Un-exempting one
/// of these three is free: their recorded output is non-empty while
/// `expected.stdout` is empty, so `agrees` is false either way and the entry
/// still lands on `KnownDivergence`. Exempting some other rung in its place
/// needs only that rung's `expected.stdout` blanked to match a blanked
/// `upstream_stdout`. Four coordinated edits, no test moves, and that rung is
/// exempt forever. Naming the set closes the swap: any change to *which* rungs
/// are exempt fails here, and has to be argued for in review.
const UPSTREAM_REFUSED: [&str; 3] = [
    "ladder_t7_private_field_advanced",
    "ladder_t7_private_field_simple",
    "ladder_t7_private_field_ws",
];

#[test]
fn reviewed_upstream_refusal_cohort_is_pinned() {
    let refused: Vec<String> = load_ledger()
        .iter()
        .filter(|(_, d)| d.upstream_exit != 0)
        .map(|(fixture, _)| fixture.clone())
        .collect();
    assert_eq!(
        refused, UPSTREAM_REFUSED,
        "the SET of rungs exempt from staleness detection changed - see the note on \
         `UPSTREAM_REFUSED`"
    );
}

/// The other half of the same lock: which rungs have an empty `expected.stdout`.
///
/// `well_formedness_error` never touches the filesystem, and `verdict` only
/// checks that the ledger's `upstream_stdout` *equals* the fixture bytes, so
/// blanking both sides at once is self-consistent and invisible to every other
/// test. The reviewed fixture inventory does not help either: it counts
/// fixtures, and `expected.stdout` only has to exist. So "exactly these three
/// rungs have empty oracle output" is a load-bearing fact that nothing
/// asserted, and it is the enabling condition for exempting an arbitrary rung.
///
/// That this is the same three rungs as `UPSTREAM_REFUSED` is not a
/// coincidence: upstream emitted nothing *because* it refused.
#[test]
fn reviewed_empty_oracle_output_cohort_is_pinned() {
    let empty: Vec<String> = ladder_fixtures()
        .iter()
        .filter(|dir| std::fs::metadata(dir.join("expected.stdout")).is_ok_and(|m| m.len() == 0))
        .map(|dir| fixture_name(dir))
        .collect();
    assert_eq!(
        empty, UPSTREAM_REFUSED,
        "the set of rungs whose oracle output is empty changed. Blanking a fixture is how \
         an arbitrary rung gets exempted from staleness detection, so it is a reviewed act \
         - see the note on `UPSTREAM_REFUSED`"
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
        upstream_exit: 0,
        closurec_stderr_starts_with: None,
    }
}

/// A rung where upstream REFUSED the input, so `expected.stdout` is empty
/// because nothing was emitted — not because the program compiles to nothing.
fn fake_upstream_declined() -> Divergence {
    Divergence {
        upstream_stdout: String::new(),
        // Deliberately empty: this models the state AFTER someone accepted a
        // regression to "emit nothing, exit 0" into the ledger. That is the only
        // arrangement in which the trap bites — while the ledger still recorded
        // our old non-empty output, the output-changed branch catches it first.
        closurec_stdout: String::new(),
        closurec_exit: 0,
        issue: "https://github.com/adhithyan15/coding-adventures/issues/15860".into(),
        level: "SIMPLE".into(),
        reason: "upstream declines this feature".into(),
        upstream_exit: 2,
        closurec_stderr_starts_with: None,
    }
}

/// A rung where WE produce no stdout, pinned by the stage we fail at.
fn fake_hard_failure() -> Divergence {
    Divergence {
        upstream_stdout: "UP".into(),
        closurec_stdout: String::new(),
        closurec_exit: 1,
        issue: "https://github.com/adhithyan15/coding-adventures/issues/15837".into(),
        level: "SIMPLE".into(),
        reason: "closurec refuses valid JavaScript".into(),
        upstream_exit: 0,
        closurec_stderr_starts_with: Some("SIMPLE compilation failed at parse stage:".into()),
    }
}

/// (a) A rung that differs from upstream with nothing recorded must fail.
#[test]
fn gate_fires_on_an_unrecorded_divergence() {
    let v = verdict("rung_simple", b"UP", b"OURS", 0, "", None);
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
    let v = verdict("rung_simple", b"UP", b"UP", 0, "", Some(&fake_divergence()));
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
        "",
        Some(&fake_divergence()),
    );
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("output changed")),
        "expected a shifted-divergence failure, got {v:?}"
    );
    // An unchanged stdout but a changed exit code is also a shift.
    let v = verdict(
        "rung_simple",
        b"UP",
        b"OURS",
        1,
        "",
        Some(&fake_divergence()),
    );
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
        "",
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
    assert_eq!(
        verdict("r_simple", b"UP", b"UP", 0, "", None),
        Verdict::Matches
    );
    assert_eq!(
        verdict("r_simple", b"UP", b"OURS", 0, "", Some(&fake_divergence())),
        Verdict::KnownDivergence
    );
}

/// (e) The trap this guard exists for. On a rung where upstream REFUSED the
/// input, `expected.stdout` is empty. If `closurec` regressed to emitting
/// nothing at exit 0, a predicate comparing only bytes and our exit code would
/// call that a match and instruct a maintainer to delete the ledger entry —
/// after which the rung would pass forever while comparing nothing. Empty
/// output is not parity with a compiler that refused.
#[test]
fn gate_does_not_call_empty_output_a_match_when_upstream_refused() {
    let v = verdict(
        "rung_simple",
        b"",
        b"",
        0,
        "",
        Some(&fake_upstream_declined()),
    );
    assert_eq!(
        v,
        Verdict::KnownDivergence,
        "a rung upstream refused must stay a known divergence, not become a match"
    );

    // Prove the guard is what does the work. The same inputs with upstream
    // recorded as having SUCCEEDED are exactly the case where an empty match is
    // genuine, and there the gate must say so.
    let succeeded = Divergence {
        upstream_exit: 0,
        ..fake_upstream_declined()
    };
    let v = verdict("rung_simple", b"", b"", 0, "", Some(&succeeded));
    assert!(
        matches!(&v, Verdict::Fail(m) if m.contains("NOW MATCHES UPSTREAM")),
        "with upstream succeeding, an empty match IS a match and must demand the \
         ledger entry be deleted, got {v:?}"
    );
}

/// (f) A recorded hard failure that starts failing at a different stage must
/// fail the gate. Exit code alone says only "it failed"; without the stderr
/// fingerprint a parse-stage regression could hide behind a bridge-stage entry.
#[test]
fn gate_fires_when_a_recorded_failure_changes_stage() {
    let k = fake_hard_failure();
    assert_eq!(
        verdict(
            "rung_simple",
            b"UP",
            b"",
            1,
            "SIMPLE compilation failed at parse stage: unexpected token\n",
            Some(&k),
        ),
        Verdict::KnownDivergence,
        "the recorded failure stage should still be a known divergence"
    );
    let moved = verdict(
        "rung_simple",
        b"UP",
        b"",
        1,
        "SIMPLE compilation failed at typed AST bridge stage: unsupported syntax\n",
        Some(&k),
    );
    assert!(
        matches!(&moved, Verdict::Fail(m) if m.contains("fails, but differently")),
        "a failure that moved to another stage must fail the gate, got {moved:?}"
    );
}
