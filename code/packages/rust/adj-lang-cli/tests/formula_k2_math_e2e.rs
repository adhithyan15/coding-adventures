//! End-to-end tests for the K-2 math trio — `mathematics/number-sequence.adj`,
//! `mathematics/comparison.adj`, `mathematics/cardinality.adj` — driven through
//! the built CLI binary against the SHIPPED stdlib. `number-sequence.adj` and
//! `cardinality.adj` compose the elementary `sum`/`difference` primitives from
//! `arithmetic/arithmetic.adj` (a cross-directory import, like
//! `clinical/cockcroft_gault.adj`); `comparison.adj` is the first shipped
//! library to exercise ADJ-FORMULA-LIBRARIES FL-8 (a `formula` body that ends
//! in a comparison, `a > b`, instead of arithmetic).

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_k2math_{tag}_{}", std::process::id()));
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

/// Copy a shipped `.adj` file into `dir` at the given relative destination
/// (creating parent dirs), preserving the directory layout so relative
/// imports (`../arithmetic/…`) resolve from an entry program at the scratch
/// root — the same technique `rs2_multistep_e2e.rs` uses for
/// `cockcroft_gault.adj`.
fn place_at(dir: &Path, src_rel: &str, dst_rel: &str) {
    let src = stdlib().join(src_rel);
    let dst = dir.join(dst_rel);
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src_rel} -> {dst_rel}: {e}"));
}

// ---------------------------------------------------------------------------
// number-sequence — next/previous compose arithmetic.adj's sum/difference.
// ---------------------------------------------------------------------------

#[test]
fn number_sequence_next_and_previous_compose_arithmetic() {
    let dir = scratch("seq");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/number-sequence.adj",
        "mathematics/number-sequence.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/number-sequence.adj\"\n\
         observe n(5)\n\
         ? next(n)\n\
         ? previous(n)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // `previous` is queried second, so it's the one the JSON's per-name
    // `derived` view retains (see `comparison.adj`'s own header note on this
    // same shadowing behavior) — 5 - 1 = 4.
    assert!(
        s.contains("\"name\":\"previous\"") && s.contains("\"value\":4"),
        "previous(5) = 4: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/CountingNumber.html"),
        "carries the Counting Number citation: {s}"
    );
    // The composed `difference` primitive's own citation corroborates.
    assert!(
        s.contains("mathworld.wolfram.com/Difference.html"),
        "corroborated by the composed arithmetic.adj primitive's citation: {s}"
    );
}

#[test]
fn number_sequence_next_alone_computes_via_sum() {
    let dir = scratch("seq_next");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/number-sequence.adj",
        "mathematics/number-sequence.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/number-sequence.adj\"\n\
         observe n(5)\n\
         ? next(n)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        s.contains("\"name\":\"next\"") && s.contains("\"value\":6"),
        "next(5) = 6: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Sum.html"),
        "corroborated by the composed arithmetic.adj sum primitive's citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// comparison — FL-8: a formula body that ends in `a > b`, not arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn comparison_greater_than_is_true_with_citation() {
    let dir = scratch("cmp_true");
    place_at(&dir, "mathematics/comparison.adj", "comparison.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparison.adj\"\n\
         observe a(5)\n\
         observe b(3)\n\
         ? greater_than(a, b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"greater_than\"") && s.contains("\"value\":1"),
        "greater_than(5, 3) = 1 (true): {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"")
            && s.contains("mathworld.wolfram.com/Greater.html"),
        "carries the MathWorld Greater citation: {s}"
    );
}

#[test]
fn comparison_greater_than_is_false_when_swapped() {
    let dir = scratch("cmp_false");
    place_at(&dir, "mathematics/comparison.adj", "comparison.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparison.adj\"\n\
         observe a(5)\n\
         observe b(3)\n\
         ? greater_than(b, a)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // less_than(a, b) is asked by swapping arguments to greater_than(b, a);
    // 3 > 5 is false.
    assert!(
        s.contains("\"name\":\"greater_than\"") && s.contains("\"value\":0"),
        "greater_than(3, 5) = 0 (false) — this IS \"3 < 5\": {s}"
    );
}

// ---------------------------------------------------------------------------
// cardinality — total_cardinality composes arithmetic.adj's sum.
// ---------------------------------------------------------------------------

#[test]
fn cardinality_total_composes_arithmetic_sum() {
    let dir = scratch("card");
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(
        &dir,
        "mathematics/cardinality.adj",
        "mathematics/cardinality.adj",
    );
    std::fs::write(
        dir.join("case.adj"),
        "import \"mathematics/cardinality.adj\"\n\
         observe count_a(5)\n\
         observe count_b(3)\n\
         ? total_cardinality(count_a, count_b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"total_cardinality\"") && s.contains("\"value\":8"),
        "total_cardinality(5, 3) = 8: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/CardinalNumber.html"),
        "carries the MathWorld Cardinal Number citation: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Sum.html"),
        "corroborated by the composed arithmetic.adj sum primitive's citation: {s}"
    );
}
