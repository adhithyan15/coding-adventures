//! End-to-end tests for **NUM-7c — `adj-verify` re-checks the sqrt `Real`/
//! `BigDouble` audit companion** (ADJ-NUMERIC-SUBSTRATE §8), driven through the
//! built `adj-verify` binary.
//!
//! A sqrt's `Real` companion is technically a *widening* (exact source →
//! approximate `BigDouble`), not a narrowing — but it reuses the same
//! recheck machinery as NUM-6v for the identical reason: turn the engine's own
//! claim about its precision into independently re-derived evidence, not
//! testimony.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("num7c_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

fn verify(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg(program)
        .output()
        .expect("run adj-verify");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn sqrt_real_companion_is_rechecked_through_adj_verify() {
    let dir = scratch("sqrt");
    let src = "let r = latex \"$\\sqrt{2}$\"\n? r\n";
    let p = write(&dir, "sqrt.adj", src);
    let (ok, out) = verify(&p);

    assert!(ok, "a sound sqrt real companion must exit 0:\n{out}");
    assert!(out.contains("\"verified\":true"), "{out}");
    assert!(
        out.contains("\"narrowings_rechecked\":1"),
        "the sqrt's real companion must be independently re-derived and confirmed:\n{out}"
    );
    assert!(out.contains("\"narrowings_mismatched\":0"), "{out}");
}

#[test]
fn a_sqrt_narrowed_by_round_to_rechecks_both_the_round_and_the_real_companion() {
    // round_to(sqrt(2), 5): the sqrt's own Real companion re-checks (exact source
    // present), while the OUTER round_to has no exact sidecar to re-round from
    // (sqrt's returned exact rational is None — irrational) and is honestly
    // reported unverifiable, never a pass — which correctly makes the run as a
    // whole NOT fully verified (exit non-zero), the same "never a pass" honesty
    // NUM-6v already established for a transcendental's narrowing. Both audit
    // channels are independently visible in the per-narrowing breakdown, without
    // interfering with each other.
    let dir = scratch("nested");
    let src = "let r = round_to(latex \"$\\sqrt{2}$\", 5)\n? r\n";
    let p = write(&dir, "nested.adj", src);
    let (ok, out) = verify(&p);

    assert!(!ok, "an unverifiable narrowing must not exit 0:\n{out}");
    assert!(out.contains("\"verified\":false"), "{out}");
    assert!(
        out.contains("\"narrowings_rechecked\":1"),
        "the inner sqrt's real companion rechecks:\n{out}"
    );
    assert!(
        out.contains("\"narrowings_unverifiable\":1"),
        "the outer round_to has no exact source to re-round from, honestly reported:\n{out}"
    );
    assert!(out.contains("\"narrowings_mismatched\":0"), "{out}");
    assert!(
        out.contains("{\"depth\":0,\"status\":\"unverifiable\"}") && out.contains("{\"depth\":1,\"status\":\"rechecked\"}"),
        "both the outer (unverifiable) and inner (rechecked) status are individually visible:\n{out}"
    );
}
