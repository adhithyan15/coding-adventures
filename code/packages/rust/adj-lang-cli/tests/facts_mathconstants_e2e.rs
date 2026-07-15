//! End-to-end test for the mathematics CONSTANTS facts library
//! (`adj-facts-stdlib/mathematics/constants.adj`) driven through the built CLI:
//! a native `table` of named constant → decimal value resolves a binding-query
//! recall with the source's citation, and abstains on a name that is not a row —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

#[test]
fn mathematics_constants_recall_binds_values_with_citation() {
    let dir = scratch("mathconstants");
    // Copy the shipped mathematics constants table beside the entry program and
    // import it.
    let src = facts_stdlib().join("mathematics/constants.adj");
    std::fs::copy(&src, dir.join("constants.adj")).expect("copy shipped constants.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"constants.adj\"\n\
         ? math_constant(pi, $V)\n\
         ? math_constant(e, $V)\n\
         ? math_constant(golden_ratio, $V)\n\
         ? math_constant(unicorn_constant, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // pi and e bind their MathWorld decimal values to EVERY published digit — the
    // exact-numbers arc (ADJ-EXACT-NUMBERS NX-1..4) is now complete, so a recall
    // binding is stored exactly as the `.adj` literal writes it and renders in full,
    // NOT truncated to the ~16 significant digits an f64 carries. Before this arc the
    // binding came back as `3.141592653589793`; now the whole 39-digit string does.
    assert!(
        out.contains("\"V\":\"3.141592653589793238462643383279502884197\""),
        "pi binds ALL 39 published decimal places, not the f64-truncated ~16: {out}"
    );
    assert!(
        out.contains("\"V\":\"2.718281828459045235360287471352662497757\""),
        "e binds ALL 39 published decimal places exactly: {out}"
    );
    // golden_ratio's literal ends in a trailing zero (…117720), which the canonical
    // decimal form drops — every SIGNIFICANT digit is still bound exactly.
    assert!(
        out.contains("\"V\":\"1.61803398874989484820458683436563811772\""),
        "golden_ratio binds its full exact decimal: {out}"
    );

    // The answer carries the Wolfram MathWorld citation, at the authoritative
    // trust tier, as its proof.
    assert!(
        out.contains("mathworld.wolfram.com/Pi.html")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld source citation: {out}"
    );
    // The `source` span quotes the verbatim text that carries pi's digits.
    assert!(
        out.contains("pi has decimal expansion given by pi=3.141592653589793238462643"),
        "source span quotes the verbatim digit text: {out}"
    );

    // `unicorn_constant` is not a row — honest abstention, never a fabricated
    // value.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown constant abstains: {out}"
    );
}

/// Path to the shipped elementary-arithmetic formula library (a sibling stdlib).
fn shipped_arithmetic_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/arithmetic/arithmetic.adj")
        .canonicalize()
        .expect("shipped arithmetic.adj must exist")
}

#[test]
fn high_precision_constant_computes_exactly_through_shipped_formula() {
    // The CLOSING proof of the exact-numbers arc (ADJ-EXACT-NUMBERS NX-4): a stored
    // high-precision constant does not just BIND exactly (test above) — it COMPUTES
    // exactly and RENDERS the computed result to every digit. We take pi's stored
    // 39-digit value (copied verbatim from `mathematics/constants.adj`), feed it as an
    // operand into the shipped `product` formula, and double it. Before the arc this
    // came back as the f64-truncated `6.283185307179586`; now the CLI's `derived.value`
    // field carries all 39 fractional digits, because NX-3 ingests the literal into an
    // exact rational (no f64 hop) and NX-4 renders the exact terminating decimal.
    let dir = scratch("compute_exact");
    let lib = std::fs::read_to_string(shipped_arithmetic_lib()).unwrap();
    std::fs::write(dir.join("arithmetic.adj"), lib).unwrap();
    // pi to 39 places — the exact value the stdlib ships and stores.
    std::fs::write(
        dir.join("case.adj"),
        "import \"arithmetic.adj\"\n\
         observe factor_one(3.141592653589793238462643383279502884197)\n\
         observe factor_two(2)\n\
         ? product(factor_one, factor_two)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"derived\":["), "has a derived section: {out}");

    // 2 · pi = 6.283185307179586476925286766559005768394 — a terminating decimal, so
    // the exact result renders in FULL, not the ~16-significant-digit f64 export.
    assert!(
        out.contains("\"value\":6.283185307179586476925286766559005768394"),
        "computed 2*pi renders ALL 39 exact digits, not the f64-truncated value: {out}"
    );
    // The exact rational sidecar corroborates it: pi's exact 40-digit mantissa over 5·10^38.
    assert!(
        out.contains("\"num\":\"3141592653589793238462643383279502884197\""),
        "the exact-rational sidecar carries pi's full mantissa: {out}"
    );
    // The applied primitive still carries its cited provenance — an auditable answer.
    assert!(
        out.contains("mathworld.wolfram.com/Product.html")
            && out.contains("\"trust\":\"authoritative\""),
        "the computed answer carries the formula's cited provenance: {out}"
    );
}
