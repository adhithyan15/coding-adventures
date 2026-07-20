//! End-to-end tests for **RS-4 PR-B — the unified reasoning trace**
//! (`ADJ-REASON-MATH.md` §E), driven through the built CLI binary.
//!
//! ## What was wrong
//!
//! You could ask ADJ *what* it concluded and *what it cited*, but not *how it
//! got there*. Three separate holes:
//!
//!   1. **Recall and table lookups emitted a citation BAG.** They render from
//!      `via_facts`, which is sorted by fact id and deduplicated — so the output
//!      told you which sources were involved but never in what order or through
//!      which rules. A set is not a derivation.
//!   2. **The step renderer ended in a `_ => {}` arm**, so four of six step
//!      kinds would be dropped without a trace. In shipped paths that arm was
//!      *latent* — the SLD renderer only ever saw fact/rule steps, because the
//!      likelihood-ratio kinds are emitted by a different function. It stops
//!      being latent the moment a new kind reaches this walker, which is exactly
//!      what `FromNegation` (below) does. A wildcard that silently discards
//!      reasoning is a trap set for the next person; the match is now total, so
//!      adding a step kind breaks the build instead of quietly shortening trails.
//!   3. **Negation-as-failure recorded nothing at all.** A rule that fired
//!      *because* something was absent showed no evidence of having checked.
//!   4. **The arithmetic derivation tree was computed and thrown away.** The
//!      engine builds one for every `let`; `derived_json` never read it.
//!
//! Each test below pins one of those.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs4_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

// ---------------------------------------------------------------------------
// (1) A recall answer now carries ORDERED, ADDRESSED steps — not just a bag.
// ---------------------------------------------------------------------------

#[test]
fn a_recall_answer_carries_ordered_addressed_steps_with_inline_provenance() {
    let dir = scratch("steps");
    let p = write(
        &dir,
        "case.adj",
        "relate deficient_in(tay_sachs, hexosaminidase_a)\n\
             source \"Tay-Sachs results from deficient hexosaminidase A.\"\n\
             locator \"https://example.test/ts\"\n\
             trust authoritative\n\
         ? deficient_in(tay_sachs, $Enzyme)\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");

    // The answer still carries its citation bag (existing consumers unaffected)…
    assert!(out.contains("\"citations\":["), "citations kept: {out}");
    // …and now ALSO the ordered derivation.
    assert!(out.contains("\"steps\":["), "steps emitted: {out}");
    // Addressed: index + depth are present on the step.
    assert!(out.contains("\"step\":0"), "step index: {out}");
    assert!(out.contains("\"depth\":0"), "step depth: {out}");
    assert!(out.contains("\"kind\":\"fact\""), "fact step kind: {out}");
    // Self-contained: the step inlines the RESOLVED provenance, so the trace is
    // readable without the KB that produced it.
    assert!(
        out.contains("Tay-Sachs results from deficient hexosaminidase A."),
        "step inlines its own quoted span: {out}"
    );
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "step inlines its trust tier: {out}"
    );
}

// ---------------------------------------------------------------------------
// (2) A rule-derived answer shows NESTING: the rule, then its body one deeper.
// ---------------------------------------------------------------------------

#[test]
fn a_rule_derived_answer_shows_the_rule_and_its_body_at_increasing_depth() {
    let dir = scratch("nesting");
    let p = write(
        &dir,
        "case.adj",
        "relate cultured(patient_1, listeria)\n\
             source \"Blood culture grew Listeria monocytogenes.\"\n\
             trust empirical\n\
         rule {\n\
             head: infected(patient_1, listeria)\n\
             when: cultured(patient_1, listeria)\n\
             source \"A positive blood culture establishes infection with the isolate.\"\n\
             trust authoritative\n\
         }\n\
         ? infected(patient_1, $Organism)\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"Organism\":\"listeria\""), "binds: {out}");

    // The rule fires at depth 0; the body goal it required sits at depth 1.
    // Preorder + depth is what lets a reader rebuild the tree.
    assert!(out.contains("\"kind\":\"rule\""), "rule step: {out}");
    assert!(out.contains("\"depth\":1"), "body is nested deeper: {out}");
    // Both clauses quote their own span — the rule's and the fact's.
    assert!(
        out.contains("A positive blood culture establishes infection with the isolate."),
        "rule's own citation appears: {out}"
    );
    assert!(
        out.contains("Blood culture grew Listeria monocytogenes."),
        "the supporting fact's citation appears: {out}"
    );
}

// ---------------------------------------------------------------------------
// (3) Negation-as-failure is VISIBLE. It used to record nothing at all.
// ---------------------------------------------------------------------------

#[test]
fn negation_as_failure_appears_as_its_own_step_instead_of_vanishing() {
    let dir = scratch("naf");
    let p = write(
        &dir,
        "case.adj",
        "relate indicated(penicillin, strep)\n\
             source \"Penicillin is indicated for streptococcal infection.\"\n\
             trust authoritative\n\
         rule {\n\
             head: prescribe(penicillin, strep)\n\
             when: indicated(penicillin, strep), not allergic_to(patient_1, penicillin)\n\
             source \"Prescribe an indicated agent absent a documented allergy.\"\n\
             trust authoritative\n\
         }\n\
         ? prescribe($Drug, strep)\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"Drug\":\"penicillin\""), "binds: {out}");

    // THE POINT: the guard that licensed the conclusion is in the trail.
    // Before RS-4 the reader could not distinguish "we confirmed no allergy"
    // from "nobody checked" — the step simply did not exist.
    assert!(
        out.contains("\"kind\":\"negation\""),
        "the NAF guard must appear as a step: {out}"
    );
    assert!(
        out.contains("allergic_to"),
        "the step names the goal shown to be absent: {out}"
    );
    assert!(
        out.contains("no proof exists for the negated goal"),
        "the step states WHY it held: {out}"
    );
}

// ---------------------------------------------------------------------------
// (4) A `let` now publishes the arithmetic the engine already computed.
// ---------------------------------------------------------------------------

#[test]
fn a_derived_value_publishes_its_derivation_tree_down_to_the_observed_facts() {
    let dir = scratch("derivation");
    let p = write(
        &dir,
        "case.adj",
        "observe distance(quantity(240, km))\n\
         observe time(quantity(3, h))\n\
         let speed = distance / time\n\
         ? speed\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"derived\":["), "derived section: {out}");

    // The tree the engine built on every `let` is finally emitted.
    assert!(
        out.contains("\"derivation\":"),
        "the derivation tree is published: {out}"
    );
    assert!(out.contains("\"node\":\"op\""), "the operation node: {out}");
    assert!(out.contains("\"op\":\"/\""), "the operator applied: {out}");
    // The leaves resolve to the observed facts' own citations — this is the
    // compute-to-bytes bridge, which existed in the engine and was never
    // reachable from outside the process.
    assert!(out.contains("\"node\":\"leaf\""), "operand leaves: {out}");
    // Each leaf names the slot it read and the magnitude it contributed, and
    // resolves through its FactId to that fact's provenance block. An `observe`
    // is a case datum rather than a library claim, so its span is empty here —
    // but the CHANNEL is present, which is what lets a leaf grounded in a cited
    // library fact carry that citation all the way into the arithmetic.
    assert!(
        out.contains("\"slot\":\"distance\""),
        "numerator leaf: {out}"
    );
    assert!(out.contains("\"slot\":\"time\""), "denominator leaf: {out}");
    assert!(out.contains("\"value\":240"), "the 240 it read: {out}");
    assert!(out.contains("\"value\":3"), "the 3 it read: {out}");
}

// ---------------------------------------------------------------------------
// (5) A table RANGE lookup carries steps too, and still cites the row it
//     selected (the RS-5e guarantee this trace is built on).
// ---------------------------------------------------------------------------

#[test]
fn a_range_lookup_answer_carries_steps_citing_the_row_it_selected() {
    let dir = scratch("lookup");
    let p = write(
        &dir,
        "case.adj",
        "table aqi {\n\
             columns min_aqi, category\n\
             row (0, good)     { source \"Green Good 0 to 50\" }\n\
             row (51, moderate){ source \"Yellow Moderate 51 to 100\" }\n\
             source  \"The AQI includes six color-coded categories.\"\n\
             locator \"https://example.test/aqi\"\n\
             trust   authoritative\n\
         }\n\
         ? lookup aqi min_aqi = 75 mode range give category\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"category\":\"moderate\""), "band: {out}");
    assert!(out.contains("\"steps\":["), "lookup carries steps: {out}");
    // The step quotes the SELECTED row, not the table's framing sentence.
    assert!(
        out.contains("Yellow Moderate 51 to 100"),
        "the step cites the selected row: {out}"
    );
}

// ---------------------------------------------------------------------------
// (6) Likelihood-ratio steps survive. These are the four kinds the old
//     `_ => {}` arm discarded without a trace.
// ---------------------------------------------------------------------------

#[test]
fn likelihood_ratio_steps_are_rendered_instead_of_being_silently_dropped() {
    let dir = scratch("lr");
    let p = write(
        &dir,
        "case.adj",
        "prior 0.20 for bacterial_meningitis\n\
           source \"Baseline prevalence among presenting patients.\" trust empirical\n\
         contributes 4.0 from symptom(stiff_neck) to bacterial_meningitis\n\
           source \"Nuchal rigidity raises the likelihood of bacterial meningitis.\" trust authoritative\n\
         observe symptom(stiff_neck)\n\
         ? bacterial_meningitis\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");

    // REGRESSION GUARD. The likelihood-ratio steps are rendered by a *different*
    // function (`proof_json`) than the SLD walker this PR rewrote, so they were
    // never the ones being dropped — the `_ => {}` was latent, not lossy. Pin
    // them anyway: making the SLD walker total is exactly the kind of change
    // that could disturb a neighbouring renderer, and "the audit trail still
    // says everything it used to" is the property worth holding onto.
    assert!(out.contains("\"kind\":\"prior\""), "prior step: {out}");
    assert!(
        out.contains("\"kind\":\"contribution\""),
        "contribution step: {out}"
    );
    // And each carries the inline log-odds an auditor re-checks, under the
    // field name this renderer has always used (`logit`) — pinned here so a
    // future trace unification cannot rename it without a deliberate decision.
    assert!(
        out.contains("\"kind\":\"prior\",\"logit\":"),
        "prior logit: {out}"
    );
    assert!(
        out.contains("\"evidence\":\"symptom(stiff_neck)\""),
        "the contribution names its evidence: {out}"
    );
    assert!(
        out.contains("Nuchal rigidity raises the likelihood of bacterial meningitis."),
        "the contribution's cited span: {out}"
    );
}
