//! Golden tests for `adj-lang-cli` — run the built binary on a small `.adj`
//! program and assert the JSON decision + proof DAG.

use std::process::Command;

/// Write `src` to a temp `.adj` file, run the CLI on it, return (success, stdout).
fn run(name: &str, src: &str) -> (bool, String) {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&path)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn single_hypothesis_emits_cited_proof_dag() {
    // prior 0.10 + LR 2.5 observed → posterior ≈ 0.2174 (logit(0.1)+ln(2.5)).
    let (ok, s) = run(
        "adjcli_single.adj",
        "prior 0.10 for acs\n  source \"Pope 1995\" trust authoritative\n\
         contributes 2.5 from symptom(pressure) to acs\n  source \"Panju 1998\" trust authoritative\n\
         observe symptom(pressure)\n? acs\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"hypothesis\":\"acs\""), "{s}");
    assert!(s.contains("\"kind\":\"prior\""), "{s}");
    assert!(s.contains("\"kind\":\"contribution\""), "{s}");
    assert!(s.contains("\"evidence\":\"symptom(pressure)\""), "{s}");
    assert!(s.contains("\"source\":\"Panju 1998\""), "{s}");
    assert!(s.contains("\"trust\":\"authoritative\""), "{s}");
    // posterior ≈ 0.2174
    assert!(
        s.contains("\"posterior\":0.217"),
        "expected ~0.217 posterior: {s}"
    );
    // single hypothesis ⇒ determinate, infinite margin → null
    assert!(s.contains("\"type\":\"determinate\""), "{s}");
    assert!(s.contains("\"margin_logit\":null"), "{s}");
}

#[test]
fn two_hypothesis_differential_ranks_and_decides() {
    // bacterial gets a strong observed LR, viral does not → bacterial leads.
    let (ok, s) = run(
        "adjcli_diff.adj",
        "prior 0.30 for bacterial\n  source \"x\" trust empirical\n\
         prior 0.30 for viral\n  source \"x\" trust empirical\n\
         contributes 15 from csf(neutrophilic) to bacterial\n  source \"Straus 2006\" trust authoritative\n\
         contributes 1.2 from csf(neutrophilic) to viral\n  source \"y\" trust inferred\n\
         observe csf(neutrophilic)\n? bacterial\n? viral\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    // both hypotheses ranked
    assert!(s.contains("\"hypothesis\":\"bacterial\""), "{s}");
    assert!(s.contains("\"hypothesis\":\"viral\""), "{s}");
    // a decision was produced (determinate leader bacterial, or kickback naming it)
    assert!(
        s.contains("\"leader\":\"bacterial\""),
        "expected bacterial leader: {s}"
    );
    // the proof cites the bacterial contribution's source
    assert!(s.contains("\"source\":\"Straus 2006\""), "{s}");
}

#[test]
fn predicate_gated_contribution_emits_cited_comparison_step() {
    // A DETERMINISTIC rule as a saturating LR: income at/above the filing
    // threshold ⇒ required to file. The proof step records the literal
    // comparison the engine evaluated on the CPU (slot/op/threshold/observed).
    let (ok, s) = run(
        "adjcli_predicate.adj",
        "prior 0.10 for required_to_file\n  source \"IRS Pub 501\" trust authoritative\n\
         contributes 1000000 from gross_income >= 14600 to required_to_file\n  source \"IRS Pub 501 (2024)\" trust authoritative\n\
         observe gross_income(18000)\n? required_to_file\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"kind\":\"predicate\""), "{s}");
    assert!(s.contains("\"slot\":\"gross_income\""), "{s}");
    assert!(s.contains("\"op\":\">=\""), "{s}");
    assert!(s.contains("\"threshold\":14600"), "{s}");
    assert!(s.contains("\"observed\":18000"), "{s}");
    assert!(s.contains("\"source\":\"IRS Pub 501 (2024)\""), "{s}");
    // saturating LR ⇒ posterior ≈ 1.0
    assert!(
        s.contains("\"posterior\":0.99") || s.contains("\"posterior\":1"),
        "{s}"
    );
}

#[test]
fn predicate_fires_over_typed_value_literal() {
    // Step 2: a unit-bearing typed value. The engine reads the leading
    // magnitude (18000) from `quantity(18000, usd)` for the comparison.
    let (ok, s) = run(
        "adjcli_typed.adj",
        "prior 0.10 for required_to_file\n  source \"IRS Pub 501\" trust authoritative\n\
         contributes 1000000 from gross_income >= 14600 to required_to_file\n  source \"IRS Pub 501 (2024)\" trust authoritative\n\
         observe gross_income(quantity(18000, usd))\n? required_to_file\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"kind\":\"predicate\""), "{s}");
    assert!(s.contains("\"observed\":18000"), "{s}");
    assert!(
        s.contains("\"posterior\":0.99") || s.contains("\"posterior\":1"),
        "{s}"
    );
}

#[test]
fn let_computed_value_drives_a_predicate() {
    // ADJ step 3b: a `let` formula is computed on the CPU and a predicate
    // fires over the derived value — the model wrote only the formula.
    let (ok, s) = run(
        "adjcli_let.adj",
        "prior 0.30 for bacterial\n  source \"x\" trust empirical\n\
         observe csf_glucose(40)\n\
         observe serum_glucose(100)\n\
         let csf_ratio = csf_glucose / serum_glucose\n\
         contributes 1000000 from csf_ratio <= 0.5 to bacterial\n  source \"Spanos 1989\" trust authoritative\n\
         ? bacterial\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"hypothesis\":\"bacterial\""), "{s}");
    // csf_ratio = 0.4 <= 0.5 fires the saturating rule → posterior ≈ 1.
    assert!(
        s.contains("\"posterior\":0.99") || s.contains("\"posterior\":1"),
        "derived-value predicate should fire: {s}"
    );
}

#[test]
fn predicate_below_threshold_does_not_fire() {
    // Income under the threshold: the predicate step never appears, and the
    // posterior stays at the prior.
    let (ok, s) = run(
        "adjcli_predicate_below.adj",
        "prior 0.10 for required_to_file\n\
         contributes 1000000 from gross_income >= 14600 to required_to_file\n\
         observe gross_income(9000)\n? required_to_file\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        !s.contains("\"kind\":\"predicate\""),
        "predicate should not fire: {s}"
    );
    assert!(s.contains("\"posterior\":0.1"), "{s}");
}

#[test]
fn compile_error_is_reported_as_json() {
    let (ok, s) = run("adjcli_bad.adj", "this is not adj-lang at all !!!\n");
    assert!(!ok, "expected non-zero exit on bad input");
    assert!(s.contains("\"error\""), "expected error JSON: {s}");
}

#[test]
fn malformed_numeric_clauses_do_not_panic_the_cli() {
    // Regression: a non-positive LR / out-of-range prior / overflowing
    // literal must be a clean `{"error":...}` (exit 1), never a panic.
    for src in [
        "contributes -5 from x to y\n? y\n",
        "prior 2 for x\n? x\n",
        "observe gross_income(1e400)\n? required_to_file\n",
    ] {
        let (ok, s) = run("adjcli_malformed.adj", src);
        assert!(!ok, "expected non-zero exit for {src:?}: {s}");
        assert!(
            s.contains("\"error\""),
            "expected error JSON for {src:?}: {s}"
        );
        assert!(!s.contains("panic"), "must not panic for {src:?}: {s}");
    }
}
