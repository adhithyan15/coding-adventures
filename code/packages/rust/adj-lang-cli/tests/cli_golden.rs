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
fn compile_error_is_reported_as_json() {
    let (ok, s) = run("adjcli_bad.adj", "this is not adj-lang at all !!!\n");
    assert!(!ok, "expected non-zero exit on bad input");
    assert!(s.contains("\"error\""), "expected error JSON: {s}");
}
