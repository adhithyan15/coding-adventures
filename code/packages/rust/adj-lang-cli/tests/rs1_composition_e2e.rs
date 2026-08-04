//! End-to-end tests for ADJ-RULE-SUBSTRATE RS-1 — a formula application is a
//! first-class sub-expression — driven through the built CLI binary against the
//! SHIPPED libraries. Four things are proven here, exactly as RS-1 requires:
//!
//!   (a) COMPOSITION: `ratio(a, b) = quotient(a, b)` — a formula CALLING a formula
//!       — computes the right value AND carries BOTH citations (ratio's primary +
//!       quotient's as a corroboration).
//!   (b) BRANCH ON A FORMULA: the `bmi_category` rulebook fires `obese` for a
//!       high-BMI case and does NOT fire it for a normal one, gating on the
//!       CPU-computed `bmi(body_mass, height)` formula directly.
//!   (c) GOLDEN PIN: the shipped `bmi.adj` still computes 22.857 with WHO's
//!       citation, and a shipped arithmetic primitive still computes its value +
//!       citation — the unification did not disturb the rung-0 surface.
//!   (d) RECURSION GUARD: a deliberately self-referential formula returns a clean
//!       `FormulaRecursionTooDeep` error, never a hang or a stack overflow.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped formula-stdlib directory.
fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs1_{tag}_{}", std::process::id()));
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
// (a) Composition — ratio calls quotient; both citations present.
// ---------------------------------------------------------------------------

#[test]
fn ratio_composes_quotient_and_carries_both_citations() {
    let dir = scratch("ratio");
    // ratio.adj imports arithmetic.adj (for `quotient`); ship both.
    place(&dir, "arithmetic/arithmetic.adj");
    place(&dir, "arithmetic/ratio.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"ratio.adj\"\n\
         observe numerator(3)\n\
         observe denominator(4)\n\
         ? ratio(numerator, denominator)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 3 / 4 = 0.75, computed via quotient on the CPU.
    assert!(
        s.contains("\"name\":\"ratio\"") && s.contains("\"value\":0.75"),
        "ratio computes 0.75: {s}"
    );
    // BOTH citations: ratio's own definition (primary source) AND quotient's
    // (a corroboration) — the composed provenance chain.
    assert!(
        s.contains("mathworld.wolfram.com/Ratio.html"),
        "primary cites the ratio definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive it composed: {s}"
    );
    assert!(
        s.contains("\"corroborations\":[{"),
        "the quotient cite rides as a corroboration on the composed value: {s}"
    );
}

// ---------------------------------------------------------------------------
// (b) Branch on a formula — the bmi_category rulebook fires off the BMI formula.
// ---------------------------------------------------------------------------

/// Run the shipped `bmi_category` rulebook for one case; return the CLI stdout.
fn run_bmi_category(tag: &str, body_mass: &str, height: &str) -> String {
    let dir = scratch(tag);
    // bmi_category.adj imports bmi.adj; ship both.
    place(&dir, "clinical/bmi.adj");
    place(&dir, "clinical/bmi_category.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!(
            "import \"bmi_category.adj\"\n\
             observe body_mass({body_mass})\n\
             observe height({height})\n\
             ? obese\n"
        ),
    )
    .unwrap();
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    s
}

#[test]
fn bmi_category_fires_obese_for_a_high_bmi_case() {
    // 100 kg / (1.7 m)² = 34.6 ≥ 30 → the predicate branch fires; posterior saturates.
    let s = run_bmi_category("obese_high", "100", "1.7");
    assert!(
        s.contains("\"kind\":\"predicate\"") && s.contains("\"slot\":\"bmi\""),
        "the branch gated on the computed BMI formula: {s}"
    );
    // Saturating LR drives the posterior far above the 0.1 base rate.
    assert!(
        s.contains("\"posterior\":0.9999910000809994"),
        "obese fires (posterior ≈ 0.99999): {s}"
    );
    // The audit shows BOTH the WHO obesity threshold (on the branch clause) and
    // the BMI definition (on the computed slot).
    assert!(
        s.contains("BMI greater than or equal to 30 is obesity"),
        "cites WHO's obesity threshold: {s}"
    );
    assert!(
        s.contains("divided by the square of height"),
        "cites the BMI definition on the computed slot: {s}"
    );
}

#[test]
fn bmi_category_does_not_fire_obese_for_a_normal_bmi_case() {
    // 65 kg / (1.75 m)² = 21.2 < 30 → the branch does NOT fire; posterior stays at
    // the 0.1 prior. The proof carries the prior only (no predicate step).
    let s = run_bmi_category("obese_normal", "65", "1.75");
    assert!(
        s.contains("\"posterior\":0.10000000000000003"),
        "obese stays at its 0.1 base rate (branch did not fire): {s}"
    );
    assert!(
        !s.contains("\"kind\":\"predicate\""),
        "no predicate step fired for a sub-threshold BMI: {s}"
    );
    // The BMI was still computed and audited (21.2), it simply did not clear 30.
    assert!(
        s.contains("\"name\":\"bmi\"") && s.contains("21.224489795918366"),
        "the BMI is computed and shown even when the branch does not fire: {s}"
    );
}

// ---------------------------------------------------------------------------
// (c) Golden pin — the shipped rung-0 surface is unchanged by the unification.
// ---------------------------------------------------------------------------

#[test]
fn golden_pin_shipped_bmi_still_computes_22_857_with_who_citation() {
    let dir = scratch("golden_bmi");
    place(&dir, "clinical/bmi.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"bmi.adj\"\n\
         observe body_mass(70)\n\
         observe height(1.75)\n\
         ? bmi(body_mass, height)\n",
    )
    .unwrap();
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 70 / 1.75² = 22.857… — the exact golden value.
    assert!(
        s.contains("\"name\":\"bmi\"") && s.contains("22.857142857142858"),
        "shipped bmi.adj still computes 22.857: {s}"
    );
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("who.int"),
        "shipped bmi.adj still carries WHO's citation: {s}"
    );
}

#[test]
fn golden_pin_shipped_arithmetic_primitive_still_computes_with_citation() {
    // A shipped leaf primitive still applies unchanged: quotient(20, 4) = 5,
    // carrying its MathWorld definition. Guards against the RS-1 Apply path
    // regressing the FL-3 primitives.
    let dir = scratch("golden_quotient");
    place(&dir, "arithmetic/arithmetic.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"arithmetic.adj\"\n\
         observe dividend(20)\n\
         observe divisor(4)\n\
         ? quotient(dividend, divisor)\n",
    )
    .unwrap();
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"quotient\"") && s.contains("\"value\":5"),
        "quotient(20, 4) = 5: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "the shipped primitive still carries its citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// (d) Recursion guard — a self-referential formula errors cleanly, never hangs.
// ---------------------------------------------------------------------------

#[test]
fn self_referential_formula_is_a_clean_recursion_error_not_a_hang() {
    let dir = scratch("recur");
    std::fs::write(
        dir.join("case.adj"),
        "formulabook loops {\n\
             formula loop(x) = loop(x)\n\
                 source \"deliberately self-referential\" trust inferred\n\
         }\n\
         observe x(1)\n\
         ? loop(x)\n",
    )
    .unwrap();
    // If the guard were missing this would overflow the stack (SIGSEGV/abort) or
    // hang; instead it must be a clean, typed error on a normal exit path.
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(!ok, "a self-referential formula must fail, not succeed: {s}");
    assert!(
        s.contains("FormulaRecursionTooDeep"),
        "the recursion guard returns a clean typed error: {s}"
    );
}
