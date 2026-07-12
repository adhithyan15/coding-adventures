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
fn let_derived_value_reports_its_inferred_dimension() {
    // ADJ-LADDER rung-4 (dimensional): a `let` over typed quantities surfaces
    // the engine-INFERRED unit in a `"derived"` section. `quantity(240, km) /
    // quantity(3, h)` is 80 — but the engine reports it as 80 **km/h**, a tag
    // it formed by Dimension::combine at the division (the model never wrote
    // "km/h"). This is the audit channel a grader uses to reject a
    // numerically-right-but-unit-wrong answer.
    let (ok, s) = run(
        "adjcli_dim_speed.adj",
        "observe distance(quantity(240, km))\n\
         observe time(quantity(3, h))\n\
         let speed = distance / time\n\
         ? speed\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"derived\":["), "expected a derived section: {s}");
    assert!(s.contains("\"name\":\"speed\""), "{s}");
    assert!(s.contains("\"value\":80"), "{s}");
    assert!(s.contains("\"dim\":\"km/h\""), "expected inferred km/h: {s}");
    // Exact integer arithmetic is preserved alongside the f64.
    // NUM-5: the exact value is an arbitrary-precision BigRational, so num/den are emitted as
    // JSON strings (they can exceed JSON's safe integer range) rather than bare numbers.
    assert!(s.contains("\"exact\":{\"num\":\"80\",\"den\":\"1\"}"), "{s}");
}

#[test]
fn same_unit_division_cancels_to_scalar_in_derived() {
    // A ratio of like quantities is dimensionless: mg / mg → scalar. The
    // derived section reports `"dim":"scalar"`, so a grader knows the answer is
    // a pure number (a wrong-unit option would be rejected).
    let (ok, s) = run(
        "adjcli_dim_ratio.adj",
        "observe num(quantity(60, mg))\n\
         observe den(quantity(20, mg))\n\
         let ratio = num / den\n\
         ? ratio\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"name\":\"ratio\""), "{s}");
    assert!(s.contains("\"value\":3"), "{s}");
    assert!(s.contains("\"dim\":\"scalar\""), "expected scalar: {s}");
}

#[test]
fn dimension_mismatch_is_an_error_not_a_silent_number() {
    // The whole point of carrying dimensions: km + h is a category error. The
    // engine refuses to compute it rather than returning a meaningless 8.
    let (ok, s) = run(
        "adjcli_dim_mismatch.adj",
        "observe a(quantity(5, km))\n\
         observe b(quantity(3, h))\n\
         let bad = a + b\n\
         ? bad\n",
    );
    assert!(!ok, "expected a non-zero exit on a dimension mismatch: {s}");
    assert!(s.contains("DimensionMismatch"), "{s}");
}

#[test]
fn programs_without_a_let_omit_the_derived_section() {
    // Byte-stability: a plain rulebook/recall program binds nothing, so the
    // output carries no `"derived"` key at all (existing goldens unchanged).
    let (ok, s) = run(
        "adjcli_no_let.adj",
        "prior 0.10 for acs\n  source \"x\" trust empirical\n\
         contributes 2.5 from symptom(pressure) to acs\n  source \"y\" trust empirical\n\
         observe symptom(pressure)\n? acs\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"derived\""), "no let ⇒ no derived section: {s}");
}

#[test]
fn predicate_rhs_expression_matches_fraction_result() {
    // Rung-1 hardening: the option predicate can compare a computed answer
    // against another ADJ arithmetic expression, so a model can emit
    // `answer == 3 / 10` instead of relying on a decimal literal.
    let (ok, s) = run(
        "adjcli_fraction_rhs.adj",
        "let answer = 1 / 10 + 2 / 10\n\
         prior 0.10 for opt_a\n\
         contributes 1000000 from answer == 3 / 10 to opt_a\n\
         ? opt_a\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"kind\":\"predicate\""), "{s}");
    assert!(s.contains("\"slot\":\"answer\""), "{s}");
    assert!(
        s.contains("\"posterior\":0.99") || s.contains("\"posterior\":1"),
        "fraction expression predicate should fire: {s}"
    );
}

#[test]
fn rule_derived_evidence_drives_contribution() {
    // The model can emit observations + a Horn rule; the engine proves the
    // derived evidence term and uses that proof to license the LR contribution.
    let (ok, s) = run(
        "adjcli_derived_evidence.adj",
        "prior 0.10 for bacterial\n  source \"base rate\" trust empirical\n\
         contributes 10 from infection_present to bacterial\n  source \"clinical LR\" trust authoritative\n\
         observe fever\n\
         observe positive_culture\n\
         rule { head: infection_present when: fever, positive_culture\n\
                source \"case decomposition\" trust inferred }\n\
         ? bacterial\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"kind\":\"contribution\""), "{s}");
    assert!(s.contains("\"evidence\":\"infection_present\""), "{s}");
    assert!(s.contains("\"evidence_proof\""), "{s}");
    assert!(s.contains("\"kind\":\"rule\""), "{s}");
    assert!(s.contains("\"goal\":\"infection_present\""), "{s}");
    assert!(s.contains("\"source\":\"case decomposition\""), "{s}");
    // prior odds 1/9, LR 10 => posterior 10/19 ≈ 0.526.
    assert!(s.contains("\"posterior\":0.526"), "{s}");
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

// ---- constraint solving in the CLI (ADJ constraints track B2b) ----

#[test]
fn solve_for_emits_solved_values() {
    // x + y = 10 ; x - y = 2 → x = 6, y = 4.
    let (ok, s) = run(
        "adjcli_solve.adj",
        "symbol x : scalar\n\
             symbol y : scalar\n\
             constrain x + y = 10\n\
             constrain x - y = 2\n\
             solve for { x, y }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"solve\":{"), "expected a solve section: {s}");
    assert!(s.contains("\"outcome\":\"solved\""), "{s}");
    assert!(s.contains("\"name\":\"x\",\"value\":6"), "{s}");
    assert!(s.contains("\"name\":\"y\",\"value\":4"), "{s}");
    assert!(s.contains("\"from_constraints\":[0,1]"), "{s}");
}

#[test]
fn solve_emits_roots_for_a_nonlinear_equation() {
    // x*x = 4 → real roots {-2, 2}.
    let (ok, s) = run(
        "adjcli_quad.adj",
        "symbol x : scalar\nconstrain x * x = 4\nsolve for { x }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"solved_roots\""), "{s}");
    assert!(s.contains("\"var\":\"x\""), "{s}");
    assert!(s.contains("\"roots\":[-2,2]"), "{s}");
}

#[test]
fn solve_emits_roots_for_native_latex_equation() {
    // The ADJ language owns LaTeX math input: the constraint parses through
    // the LaTeX MathFrontend, lowers x^2 to the solver expression, and the
    // existing native solver returns the roots.
    let (ok, s) = run(
        "adjcli_latex_quad.adj",
        "symbol x : scalar\nconstrain latex \"$x^2 = 4$\"\nsolve for { x }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"solved_roots\""), "{s}");
    assert!(s.contains("\"var\":\"x\""), "{s}");
    assert!(s.contains("\"roots\":[-2,2]"), "{s}");
}

#[test]
fn solve_emits_roots_for_native_latex_factored_equation() {
    // Adjacent LaTeX factors stay on the native parser path; the solver expands
    // the polynomial and returns the finite real roots.
    let (ok, s) = run(
        "adjcli_latex_factored.adj",
        "symbol x : scalar\nconstrain latex \"$(x + 2)(x - 3)(x - 6) = 0$\"\nsolve for { x }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"solved_roots\""), "{s}");
    assert!(s.contains("\"var\":\"x\""), "{s}");
    assert!(s.contains("\"roots\":[-2,3,6]"), "{s}");
}

#[test]
fn solve_substitutes_observed_facts() {
    // base_rate is observed (not an unknown) → substituted as a constant, so
    // premium = base_rate + 300 = 1500. This is the realistic mixed case.
    let (ok, s) = run(
        "adjcli_solve_subst.adj",
        "symbol premium : money(usd)\n\
         observe base_rate(1200)\n\
         constrain premium = base_rate + 300\n\
         solve for { premium }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"solved\""), "{s}");
    assert!(s.contains("\"name\":\"premium\",\"value\":1500"), "{s}");
}

#[test]
fn unsupported_constraint_reports_a_reason_not_an_answer() {
    // An inequality is out of this slice's scope → unsupported, never a fake value.
    let (ok, s) = run(
        "adjcli_unsupported.adj",
        "symbol x : scalar\nconstrain x <= 10\nsolve for { x }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"unsupported\""), "{s}");
    assert!(s.contains("\"reason\""), "{s}");
}

// ---- feasibility / check in the CLI (ADJ constraints B2c) ----

#[test]
fn check_reports_sat_with_a_witness_assignment() {
    // x >= 3 ; x <= 5 is jointly satisfiable → sat, with an integer witness.
    let (ok, s) = run(
        "adjcli_check_sat.adj",
        "symbol x : scalar\n\
             constrain x >= 3\n\
             constrain x <= 5\n\
             check\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"check\":{"), "expected a check section: {s}");
    assert!(s.contains("\"outcome\":\"sat\""), "{s}");
    assert!(s.contains("\"name\":\"x\""), "{s}");
}

#[test]
fn check_reports_unsat_with_a_conflicting_core() {
    // x >= 5 ; x <= 3 cannot both hold → unsat, citing the conflicting clauses.
    let (ok, s) = run(
        "adjcli_check_unsat.adj",
        "symbol x : scalar\n\
             constrain x >= 5\n\
             constrain x <= 3\n\
             check\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"unsat\""), "{s}");
    assert!(s.contains("\"core\":["), "{s}");
}

// ---- QF_LRA real feasibility in the CLI (ADJ constraints C1) ----

#[test]
fn check_reports_sat_real_for_a_fractional_system() {
    // 0.25 <= x <= 0.75 has no integer point but is real-feasible → the CLI
    // emits a `sat_real` verdict with a rational witness.
    let (ok, s) = run(
        "adjcli_check_satreal.adj",
        "symbol x : scalar\n\
             constrain x >= 0.25\n\
             constrain x <= 0.75\n\
             check\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"check\":{"), "expected a check section: {s}");
    assert!(s.contains("\"outcome\":\"sat_real\""), "{s}");
    assert!(s.contains("\"name\":\"x\""), "{s}");
}

#[test]
fn check_reports_sat_real_when_integer_infeasible() {
    // 2x = 1 is integer-infeasible but real-feasible at x = 0.5 → sat_real.
    let (ok, s) = run(
        "adjcli_check_2x1.adj",
        "symbol x : scalar\nconstrain 2 * x = 1\ncheck\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"sat_real\""), "{s}");
    assert!(s.contains("\"value\":0.5"), "expected x = 0.5 witness: {s}");
}

#[test]
fn no_check_keyword_emits_no_check_section() {
    // A solve-only constraint system requests no feasibility verdict.
    let (ok, s) = run(
        "adjcli_nocheck.adj",
        "symbol x : scalar\nconstrain x + 1 = 4\nsolve for { x }\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        !s.contains("\"check\""),
        "no check keyword → no check key: {s}"
    );
}

#[test]
fn a_pure_rulebook_emits_no_solve_section() {
    let (ok, s) = run(
        "adjcli_nosolve.adj",
        "prior 0.10 for acs\n  source \"x\" trust empirical\n? acs\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        !s.contains("\"solve\""),
        "no constraint system → no solve key: {s}"
    );
}

// ---- linear optimization in the CLI (ADJ constraints C2) ----

#[test]
fn maximize_emits_an_optimal_value_and_witness() {
    // The classic LP: max 3x + 2y s.t. x+y<=4, x<=3, x,y>=0 → 11 at (3,1).
    let (ok, s) = run(
        "adjcli_lp.adj",
        "symbol x : scalar\nsymbol y : scalar\n\
             constrain x + y <= 4\nconstrain x <= 3\n\
             constrain x >= 0\nconstrain y >= 0\n\
             maximize 3 * x + 2 * y\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        s.contains("\"optimize\":{"),
        "expected an optimize section: {s}"
    );
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(s.contains("\"value\":11"), "expected optimum 11: {s}");
    assert!(s.contains("\"binding\":["), "{s}");
}

#[test]
fn minimize_emits_its_optimum() {
    // min x + y s.t. x>=2, y>=3 → 5.
    let (ok, s) = run(
        "adjcli_min.adj",
        "symbol x : scalar\nsymbol y : scalar\n\
             constrain x >= 2\nconstrain y >= 3\nminimize x + y\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(s.contains("\"value\":5"), "expected optimum 5: {s}");
}

#[test]
fn an_unbounded_objective_is_reported_in_the_cli() {
    let (ok, s) = run(
        "adjcli_unbounded.adj",
        "symbol x : scalar\nconstrain x >= 0\nmaximize x\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"unbounded\""), "{s}");
}

#[test]
fn no_objective_emits_no_optimize_section() {
    let (ok, s) = run(
        "adjcli_noopt.adj",
        "symbol x : scalar\nconstrain x >= 1\ncheck\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        !s.contains("\"optimize\""),
        "no objective → no optimize key: {s}"
    );
}

// ---- feed-a-verdict: constraint outcome drives the differential (E2) ----

#[test]
fn an_infeasible_check_feeds_a_verdict_into_the_differential() {
    // The schedule is contradictory (design ≥ 28 forces build ≥ 48 > 45). The
    // `check` returns unsat; the engine injects `infeasible`, which fires
    // `contributes from infeasible to schedule_broken` in the SAME differential.
    let (ok, s) = run(
        "adjcli_feedv_unsat.adj",
        "prior 0.10 for schedule_ok\n  source \"x\" trust empirical\n\
         prior 0.10 for schedule_broken\n  source \"x\" trust empirical\n\
         contributes 1000000 from infeasible to schedule_broken\n  source \"sched\" trust authoritative\n\
         symbol d : scalar\nsymbol b : scalar\n\
         constrain d >= 28\nconstrain b >= d + 20\nconstrain b <= 45\ncheck\n\
         ? schedule_ok\n? schedule_broken\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"unsat\""), "{s}");
    // The constraint verdict drove the differential to schedule_broken.
    assert!(s.contains("\"leader\":\"schedule_broken\""), "{s}");
}

#[test]
fn a_feasible_check_drives_the_other_verdict() {
    // Same clauses, looser deadline (design ≥ 25) → feasible → `feasible` fires
    // `contributes from feasible to schedule_ok`, so schedule_ok leads instead.
    let (ok, s) = run(
        "adjcli_feedv_sat.adj",
        "prior 0.10 for schedule_ok\n  source \"x\" trust empirical\n\
         prior 0.10 for schedule_broken\n  source \"x\" trust empirical\n\
         contributes 1000000 from feasible to schedule_ok\n  source \"sched\" trust authoritative\n\
         contributes 1000000 from infeasible to schedule_broken\n  source \"sched\" trust authoritative\n\
         symbol d : scalar\nsymbol b : scalar\n\
         constrain d >= 25\nconstrain b >= d + 20\nconstrain b <= 45\ncheck\n\
         ? schedule_ok\n? schedule_broken\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"sat\""), "{s}");
    assert!(s.contains("\"leader\":\"schedule_ok\""), "{s}");
}

#[test]
fn an_infeasible_lp_feeds_a_verdict() {
    // An infeasible `maximize` injects `infeasible` just like `check`.
    let (ok, s) = run(
        "adjcli_feedv_lp.adj",
        "prior 0.10 for plan_ok\n  source \"x\" trust empirical\n\
         prior 0.10 for plan_overcommitted\n  source \"x\" trust empirical\n\
         contributes 1000000 from infeasible to plan_overcommitted\n  source \"p\" trust authoritative\n\
         symbol x : scalar\nconstrain x >= 5\nconstrain x <= 1\nmaximize x\n\
         ? plan_ok\n? plan_overcommitted\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"infeasible\""), "{s}");
    assert!(s.contains("\"leader\":\"plan_overcommitted\""), "{s}");
}

#[test]
fn no_status_clause_means_constraints_dont_disturb_the_differential() {
    // A program with constraints but NO `contributes from <status>` clause: the
    // injected status fact is inert (nothing references it), so the differential
    // is unchanged — the prior leads.
    let (ok, s) = run(
        "adjcli_feedv_inert.adj",
        "prior 0.30 for acs\n  source \"x\" trust empirical\n\
         symbol x : scalar\nconstrain x >= 5\nconstrain x <= 1\ncheck\n? acs\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"outcome\":\"unsat\""), "{s}");
    assert!(s.contains("\"posterior\":0.3"), "prior unchanged: {s}");
}

// ---- FromSolve: the solver certificate renders under the verdict step (E3) ----

#[test]
fn the_verdict_proof_step_carries_the_iis_core() {
    // The contribution that fired from `infeasible` now embeds the minimal
    // infeasibility certificate (IIS core) right under the verdict's proof step,
    // so the whole adjudication is one auditable tree.
    let (ok, s) = run(
        "adjcli_fromsolve_iis.adj",
        "prior 0.10 for schedule_broken\n  source \"x\" trust empirical\n\
         contributes 1000000 from infeasible to schedule_broken\n  source \"sched\" trust authoritative\n\
         symbol d : scalar\nsymbol b : scalar\n\
         constrain d >= 28\nconstrain b >= d + 20\nconstrain b <= 45\ncheck\n\
         ? schedule_broken\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    // the proof step embeds the solver cert with the IIS core
    assert!(
        s.contains("\"solver\":{"),
        "expected a solver field on the step: {s}"
    );
    assert!(s.contains("\"outcome\":\"unsat\""), "{s}");
    assert!(
        s.contains("\"core\":[0,1,2]"),
        "expected the IIS core under the step: {s}"
    );
}

#[test]
fn the_verdict_proof_step_carries_the_solved_assignment() {
    // A `solve` that fires a verdict via `solved` embeds the assignment cert.
    let (ok, s) = run(
        "adjcli_fromsolve_solved.adj",
        "prior 0.10 for priced\n  source \"x\" trust empirical\n\
         contributes 1000000 from solved to priced\n  source \"calc\" trust authoritative\n\
         symbol p : scalar\nconstrain p * 1000 = 8000\nsolve for { p }\n? priced\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"solver\":{"), "{s}");
    assert!(
        s.contains("\"name\":\"p\",\"value\":8"),
        "expected solved p=8 under the step: {s}"
    );
}

#[test]
fn the_verdict_proof_step_carries_the_optimum() {
    // An `optimize` that fires a verdict via `optimal` embeds the value + binding.
    let (ok, s) = run(
        "adjcli_fromsolve_opt.adj",
        "prior 0.10 for allocated\n  source \"x\" trust empirical\n\
         contributes 1000000 from optimal to allocated\n  source \"lp\" trust authoritative\n\
         symbol x : scalar\nconstrain x <= 5\nconstrain x >= 0\nmaximize x\n? allocated\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(s.contains("\"solver\":{"), "{s}");
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(
        s.contains("\"value\":5"),
        "expected optimum 5 under the step: {s}"
    );
}

#[test]
fn a_non_status_contribution_has_no_solver_field() {
    // An ordinary contribution (not from a constraint status) carries no solver.
    let (ok, s) = run(
        "adjcli_fromsolve_none.adj",
        "prior 0.10 for acs\n  source \"x\" trust empirical\n\
         contributes 2.5 from symptom(chest_pain) to acs\n  source \"y\" trust empirical\n\
         observe symptom(chest_pain)\n? acs\n",
    );
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(
        !s.contains("\"solver\":"),
        "no constraint status → no solver field: {s}"
    );
}
