//! End-to-end tests for **NUM-6v — `adj-verify` re-checks the precision/format
//! narrowings** (`ADJ-NUMERIC-SUBSTRATE.md` §4.3, §6, §7), driven through the
//! built binary.
//!
//! The rest of `adj-verify` re-executes the *logic* of a trail — unification,
//! log-odds, negation, quotes. The compute derivation trees carry the
//! *arithmetic*, and every `round_to`/`round_sig`/`to_scientific`/`to_percent`/
//! `to_currency` narrowing had, until now, one part a checker took on faith: the
//! rounded number (and the boundary string) it printed. These tests confirm that
//! `adj-verify` now re-rounds each narrowing's recorded exact source under the
//! recorded mode and confirms the rendered result — so a rounding is evidence,
//! not testimony (the same standard the logic re-checks already meet).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("num6v_{tag}_{}", std::process::id()));
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

// ---------------------------------------------------------------------------
// (1) Every narrowing kind is re-rounded and confirmed.
// ---------------------------------------------------------------------------

#[test]
fn each_narrowing_is_rechecked_from_its_exact_source() {
    // One derived value per narrowing kind, each an exact rational so the
    // re-round is well-defined. All must re-check, and the run must stay green.
    let dir = scratch("all");
    let src = "let a = round_to(10 / 3, 2)\n\
         let b = round_sig(31459, 3)\n\
         let c = to_scientific(1 / 3, 4)\n\
         let d = to_percent(1 / 3, 2)\n\
         let e = to_currency(100 / 3, usd, 2)\n\
     ? a\n";
    let p = write(&dir, "narrow.adj", src);
    let (ok, out) = verify(&p);

    assert!(ok, "a sound set of narrowings must exit 0:\n{out}");
    assert!(out.contains("\"verified\":true"), "{out}");
    // Five derived narrowings, all re-rounded from their exact source.
    assert!(
        out.contains("\"narrowings_rechecked\":5"),
        "every narrowing must be independently re-rounded:\n{out}"
    );
    assert!(
        out.contains("\"narrowings_mismatched\":0"),
        "and none may disagree with its exact source:\n{out}"
    );
    // The per-value section names WHICH bindings were checked and how.
    assert!(
        out.contains("\"status\":\"rechecked\""),
        "the report must show the per-narrowing verdict:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// (2) A plain arithmetic formula has nothing to re-check — and says so.
// ---------------------------------------------------------------------------

#[test]
fn a_formula_with_no_narrowing_reports_zero_and_still_verifies() {
    // A bare division carries no narrowing node. The narrowing counters are zero,
    // honestly — not silently "all checked" over an empty set.
    let dir = scratch("none");
    let src = "let q = 10 / 3\n? q\n";
    let p = write(&dir, "plain.adj", src);
    let (ok, out) = verify(&p);

    assert!(ok, "{out}");
    assert!(out.contains("\"verified\":true"), "{out}");
    assert!(out.contains("\"narrowings_rechecked\":0"), "{out}");
    assert!(out.contains("\"narrowings_mismatched\":0"), "{out}");
}

// ---------------------------------------------------------------------------
// (3) A nested narrowing (round_to over to_percent) re-checks both levels.
// ---------------------------------------------------------------------------

#[test]
fn a_nested_narrowing_rechecks_the_outer_and_inner_steps() {
    // `round_to(to_percent(1/3, 4), 2)`: the outer round and the inner percentage
    // are both narrowings, and the tree walk must find and re-round BOTH.
    let dir = scratch("nested");
    let src = "let r = round_to(to_percent(1 / 3, 4), 2)\n? r\n";
    let p = write(&dir, "nested.adj", src);
    let (ok, out) = verify(&p);

    assert!(ok, "{out}");
    assert!(
        out.contains("\"narrowings_rechecked\":2"),
        "both the outer round and the inner to_percent must be re-checked:\n{out}"
    );
    assert!(out.contains("\"narrowings_mismatched\":0"), "{out}");
}
