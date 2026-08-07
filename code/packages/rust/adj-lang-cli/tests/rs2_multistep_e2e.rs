//! End-to-end tests for ADJ-RULE-SUBSTRATE RS-2 — MULTI-STEP formula bodies
//! (in-formula `let`-steps), driven through the built CLI binary. Four things
//! are proven:
//!
//!   (a) WORKED COMPOSITE: the shipped `cockcroft_gault.adj` — a four-`let`-step
//!       formula composing `difference`/`product`/`quotient` — computes the
//!       creatinine-clearance estimate (80 mL/min for the canonical case) on the
//!       CPU, carrying its citation.
//!   (b) STEP-REFERENCES-STEP: a self-contained inline multi-step formula whose
//!       later steps reference earlier ones, and whose final expression combines
//!       them, computes the right value — the core RS-2 semantics with no imports.
//!   (c) UNDECLARED / FORWARD reference is a clean `FormulaFreeVariable` scoping
//!       error (scope grows strictly left-to-right), never a silent 0 or a panic.
//!   (d) BACK-COMPAT: the single-expression sugar (`formula f(...) = <expr>`) is
//!       unchanged — the block form is purely additive.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs2_{tag}_{}", std::process::id()));
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
/// (creating parent dirs), preserving the directory layout so relative imports
/// (`../arithmetic/...`) resolve from an entry program at the scratch root.
fn place_at(dir: &Path, src_rel: &str, dst_rel: &str) {
    let src = stdlib().join(src_rel);
    let dst = dir.join(dst_rel);
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src_rel} -> {dst_rel}: {e}"));
}

// ---------------------------------------------------------------------------
// (a) Worked composite — cockcroft_gault, a 4-step formula, computes 80.
// ---------------------------------------------------------------------------

#[test]
fn cockcroft_gault_multistep_computes_creatinine_clearance() {
    let dir = scratch("cg");
    // Preserve the two-subdir layout so cockcroft_gault's `../arithmetic/…`
    // import stays inside the (scratch-root) import root.
    place_at(&dir, "arithmetic/arithmetic.adj", "arithmetic/arithmetic.adj");
    place_at(&dir, "clinical/cockcroft_gault.adj", "clinical/cockcroft_gault.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"clinical/cockcroft_gault.adj\"\n\
         observe age(60)\n\
         observe weight(72)\n\
         observe creatinine(1)\n\
         observe sex_factor(1)\n\
         ? cockcroft_gault(age, weight, creatinine, sex_factor)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // ((140 - 60) * 72 * 1) / (72 * 1) = 80, via the four named let-steps.
    assert!(
        s.contains("\"name\":\"cockcroft_gault\"") && s.contains("\"value\":80"),
        "cockcroft_gault(60, 72, 1, 1) = 80: {s}"
    );
    // The formula's own citation rides on the answer.
    assert!(
        s.contains("NBK555956") && s.contains("\"trust\":\"authoritative\""),
        "carries the Cockcroft-Gault citation: {s}"
    );
}

// ---------------------------------------------------------------------------
// (b) Step-references-step — self-contained inline multi-step formula.
// ---------------------------------------------------------------------------

#[test]
fn inline_multistep_body_references_earlier_steps() {
    let dir = scratch("inline");
    // (a + b) then (a - b), combined as their product — each step names an
    // intermediate the final expression reuses. No imports: pure RS-2 substrate.
    std::fs::write(
        dir.join("case.adj"),
        "formulabook demo {\n\
             formula span_product(a, b) {\n\
                 let total = a + b\n\
                 let gap = a - b\n\
                 total * gap\n\
             }\n\
                 source \"test: (a+b)*(a-b) via two named let-steps\" trust inferred\n\
         }\n\
         observe a(5)\n\
         observe b(3)\n\
         ? span_product(a, b)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // (5 + 3) * (5 - 3) = 8 * 2 = 16.
    assert!(
        s.contains("\"name\":\"span_product\"") && s.contains("\"value\":16"),
        "span_product(5, 3) = 16: {s}"
    );
}

// ---------------------------------------------------------------------------
// (c) Scoping — an undeclared / forward step reference is a clean error.
// ---------------------------------------------------------------------------

#[test]
fn step_referencing_an_undeclared_name_is_a_clean_scoping_error() {
    let dir = scratch("scope");
    // `later` is defined AFTER the step that uses it; scope grows left-to-right,
    // so this is a free variable, not a forward reference.
    std::fs::write(
        dir.join("case.adj"),
        "formulabook bad {\n\
             formula oops(a) {\n\
                 let early = later + a\n\
                 let later = a\n\
                 early\n\
             }\n\
                 source \"test: forward reference must be rejected\" trust inferred\n\
         }\n\
         observe a(1)\n\
         ? oops(a)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(!ok, "a forward/undeclared step reference must fail: {s}");
    assert!(
        s.contains("FormulaFreeVariable"),
        "the scoping check returns a clean typed error: {s}"
    );
}

// ---------------------------------------------------------------------------
// (d) Back-compat — the single-expression sugar is unchanged.
// ---------------------------------------------------------------------------

#[test]
fn single_expression_formula_still_works_alongside_the_block_form() {
    let dir = scratch("sugar");
    std::fs::write(
        dir.join("case.adj"),
        "formulabook demo {\n\
             formula twice(a) = a + a\n\
                 source \"test: doubling, single-expression form\" trust inferred\n\
         }\n\
         observe a(21)\n\
         ? twice(a)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(
        s.contains("\"name\":\"twice\"") && s.contains("\"value\":42"),
        "the single-expression sugar still computes: {s}"
    );
}
