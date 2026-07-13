//! End-to-end tests for ADJ-FORMULA-LIBRARIES FL-4 — the `percent.adj` and
//! `average.adj` elementary libraries — driven through the built CLI binary
//! against the SHIPPED stdlib. Each proves the FL-4 invariant: the new library
//! COMPOSES the cited `arithmetic.adj` primitives (it re-derives no arithmetic),
//! computes the exact value on the CPU, and carries BOTH its own citation and the
//! primitives' as corroborations — write-once-use-many all the way down.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fl4_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Copy a shipped `.adj` file (by relative path under the stdlib) into `dir`
/// under its basename, so a consumer's relative `import` resolves.
fn place(dir: &Path, rel: &str) {
    let src = stdlib().join(rel);
    let name = Path::new(rel).file_name().unwrap();
    std::fs::copy(&src, dir.join(name)).unwrap_or_else(|e| panic!("copy {rel}: {e}"));
}

// ---------------------------------------------------------------------------
// percent — a share per hundred, composing quotient.
// ---------------------------------------------------------------------------

#[test]
fn percent_composes_quotient_and_carries_both_citations() {
    let dir = scratch("percent");
    place(&dir, "arithmetic/arithmetic.adj");
    place(&dir, "arithmetic/percent.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"percent.adj\"\n\
         observe part(30)\n\
         observe whole(40)\n\
         ? percent(part, whole)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 30 / 40 * 100 = 75, via quotient on the CPU.
    assert!(
        s.contains("\"name\":\"percent\"") && s.contains("\"value\":75"),
        "percent(30, 40) = 75: {s}"
    );
    // BOTH citations: percent's own definition (primary) AND the quotient
    // primitive it composed (corroboration).
    assert!(
        s.contains("mathworld.wolfram.com/Percent.html"),
        "primary cites the percent definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive it composed: {s}"
    );
}

#[test]
fn percent_change_composes_difference_and_quotient() {
    let dir = scratch("pctchg");
    place(&dir, "arithmetic/arithmetic.adj");
    place(&dir, "arithmetic/percent.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"percent.adj\"\n\
         observe old_amount(200)\n\
         observe new_amount(250)\n\
         ? percent_change(old_amount, new_amount)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (250 - 200) / 200 * 100 = 25, composing difference then quotient.
    assert!(
        s.contains("\"name\":\"percent_change\"") && s.contains("\"value\":25"),
        "percent_change(200, 250) = 25: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/PercentageChange.html"),
        "primary cites the percentage-change definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Difference.html"),
        "cites the difference primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// average — the arithmetic mean, composing sum then quotient.
// ---------------------------------------------------------------------------

#[test]
fn mean_two_composes_sum_and_quotient() {
    let dir = scratch("mean2");
    place(&dir, "arithmetic/arithmetic.adj");
    place(&dir, "arithmetic/average.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"average.adj\"\n\
         observe value_one(4)\n\
         observe value_two(6)\n\
         ? mean_two(value_one, value_two)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (4 + 6) / 2 = 5, composing sum then quotient.
    assert!(
        s.contains("\"name\":\"mean_two\"") && s.contains("\"value\":5"),
        "mean_two(4, 6) = 5: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/ArithmeticMean.html"),
        "primary cites the arithmetic-mean definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Sum.html"),
        "cites the sum primitive it composed: {s}"
    );
}

#[test]
fn mean_three_pools_three_values_over_three() {
    let dir = scratch("mean3");
    place(&dir, "arithmetic/arithmetic.adj");
    place(&dir, "arithmetic/average.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"average.adj\"\n\
         observe value_one(3)\n\
         observe value_two(5)\n\
         observe value_three(10)\n\
         ? mean_three(value_one, value_two, value_three)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (3 + 5 + 10) / 3 = 6, composing sum twice then quotient.
    assert!(
        s.contains("\"name\":\"mean_three\"") && s.contains("\"value\":6"),
        "mean_three(3, 5, 10) = 6: {s}"
    );
}
