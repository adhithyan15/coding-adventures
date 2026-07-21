//! End-to-end tests for **RS-4 PR-C — the typed `AbstentionReason`**
//! (`ADJ-REASON-MATH.md` §E.4), driven through the built CLI binary.
//!
//! ## What was wrong
//!
//! Every abstention was the same value: `"abstained": true`. One bit, for
//! situations that are not merely different but *opposite*:
//!
//!   * a lookup **below the table's domain** — the question was fine, the
//!     source does not reach that far. The table is being honest.
//!   * a lookup with a **malformed key** — nothing is wrong with the table;
//!     the caller is wrong.
//!
//! Those two emitted **byte-identical JSON**. A consumer could not tell "widen
//! your source" from "fix your query", which makes an abstention unactionable —
//! and an unactionable "I don't know" is barely better than a wrong answer.
//!
//! A third case is subtler and was introduced by PR-B's own recursion guard:
//! a search that hits a resolution limit **stopped looking**. It established no
//! absence at all. Reporting that as "no grounded support" would launder a
//! resource limit into a claim about the knowledge base.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs4c_{tag}_{}", std::process::id()));
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

const AQI: &str = r#"
table aqi {
    columns min_aqi, category
    row (0, good)      { source "Green Good 0 to 50" }
    row (51, moderate) { source "Yellow Moderate 51 to 100" }
    source  "The AQI includes six color-coded categories."
    locator "https://example.test/aqi"
    trust   authoritative
}
"#;

// ---------------------------------------------------------------------------
// (1) THE HEADLINE: below-domain and malformed-key no longer look identical.
// ---------------------------------------------------------------------------

#[test]
fn below_domain_and_malformed_key_no_longer_emit_identical_json() {
    let dir = scratch("distinct");

    // Below the table's floor: the source's domain starts at 0.
    let below = write(
        &dir,
        "below.adj",
        &format!("{AQI}\n? lookup aqi min_aqi = -5 mode range give category\n"),
    );
    let (ok_b, out_b, err_b) = run(&below);
    assert!(ok_b, "cli should succeed; stderr={err_b}");

    assert!(out_b.contains("\"abstained\":true"), "abstains: {out_b}");
    assert!(
        out_b.contains("\"reason\":\"below_table_domain\""),
        "the reason must name the domain failure: {out_b}"
    );
    // And it reports the floor, so the caller learns WHAT domain they fell
    // outside rather than guessing.
    assert!(
        out_b.contains("\"min_key\":\"0\""),
        "the abstention names the table's floor: {out_b}"
    );
    assert!(
        out_b.contains("\"table\":\"aqi\""),
        "and which table: {out_b}"
    );

    // The two abstention payloads must not be the same bytes — that identity
    // is the exact defect this PR exists to remove.
    assert!(
        !out_b.contains("non_numeric_key"),
        "below-domain must not be reported as a malformed key: {out_b}"
    );
}

// ---------------------------------------------------------------------------
// (2) A recall abstention says WHY, and distinguishes absence from a budget.
// ---------------------------------------------------------------------------

#[test]
fn a_recall_abstention_states_that_nothing_grounds_the_goal() {
    let dir = scratch("recall");
    let p = write(
        &dir,
        "case.adj",
        "relate deficient_in(tay_sachs, hexosaminidase_a)\n\
             source \"Tay-Sachs results from deficient hexosaminidase A.\"\n\
             trust authoritative\n\
         ? deficient_in(gaucher, $Enzyme)\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"abstained\":true"), "abstains: {out}");
    assert!(
        out.contains("\"reason\":\"no_grounded_support\""),
        "a genuine absence is named as such: {out}"
    );
    // Crucially NOT reported as a stopped search — the search did finish, and
    // its emptiness really is evidence of absence.
    assert!(
        !out.contains("search_limit_exceeded"),
        "a completed search must not claim it hit a limit: {out}"
    );
}

// ---------------------------------------------------------------------------
// (3) A search that STOPPED is never reported as an absence.
// ---------------------------------------------------------------------------

#[test]
fn a_truncated_search_is_reported_as_a_limit_not_as_no_support() {
    let dir = scratch("truncated");
    // Self-recursive rule: PR-B's guard makes this abstain instead of aborting.
    // PR-C's job is to say WHICH kind of abstention it is.
    let p = write(
        &dir,
        "case.adj",
        "relate p(a, b)\n\
             source \"A seed edge.\"\n\
             trust empirical\n\
         rule {\n\
             head: p($X, $Y)\n\
             when: p($X, $Y)\n\
             source \"A rule that requires itself — no base case.\"\n\
             trust authoritative\n\
         }\n\
         ? p($A, $B)\n",
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "must not abort; stderr={err}");
    assert!(out.contains("\"abstained\":true"), "abstains: {out}");
    assert!(
        out.contains("\"reason\":\"search_limit_exceeded\""),
        "a stopped search must say so: {out}"
    );
    // THE POINT: it must NOT claim the knowledge base lacks support. It does
    // not know that — it never finished looking.
    assert!(
        !out.contains("no_grounded_support"),
        "a truncated search must never be laundered into a claim of absence: {out}"
    );
    assert!(
        out.contains("NOT evidence that no proof exists"),
        "the explanation states the limit of what was established: {out}"
    );
}

// ---------------------------------------------------------------------------
// (4) A successful answer carries NO abstention object at all.
// ---------------------------------------------------------------------------

#[test]
fn an_answered_query_carries_no_abstention_object() {
    let dir = scratch("answered");
    let p = write(
        &dir,
        "case.adj",
        &format!("{AQI}\n? lookup aqi min_aqi = 75 mode range give category\n"),
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"category\":\"moderate\""), "answers: {out}");
    assert!(
        out.contains("\"abstained\":false"),
        "does not abstain: {out}"
    );
    // Additive: the field appears only when there is something to explain, so
    // an answered query's bytes are unchanged from before this PR.
    assert!(
        !out.contains("\"abstention\""),
        "no abstention object on a successful answer: {out}"
    );
}

// ---------------------------------------------------------------------------
// (5) SPEC COMPLIANCE (§E.4): echoed payloads are length-capped and redactable.
//
//     Found by this PR's own security review. §E.4 imposes three requirements
//     on echoed payloads — sanitize, length-cap, redact on a sensitive channel
//     — and the first draft implemented only sanitization. These fields carry
//     the CALLER'S INPUT back out into an artifact designed to be replayed and
//     shared; in the medical arm an unresolved surface form can be free text
//     lifted from a chart.
// ---------------------------------------------------------------------------

#[test]
fn an_echoed_payload_is_length_capped_rather_than_echoed_whole() {
    let dir = scratch("cap");
    // A goal far longer than the 256-char cap.
    let long_atom = "x".repeat(600);
    let p = write(
        &dir,
        "case.adj",
        &format!(
            "relate seen(a, b)\n    source \"A seed.\"\n    trust empirical\n? seen({long_atom}, $V)\n"
        ),
    );
    let (ok, out, err) = run(&p);
    assert!(ok, "cli should succeed; stderr={err}");
    assert!(out.contains("\"abstained\":true"), "abstains: {out}");
    assert!(
        out.contains("(truncated)"),
        "an over-long payload is capped and MARKED as capped, so a reader never \
         mistakes a cut string for the whole value: {out}"
    );
    // Scope the check to the ABSTENTION payload — which is what §E.4 governs.
    // The pre-existing `queries` / `query` fields echo the caller's query in
    // full and always have, so the cap does not reduce the document's total
    // echo; be honest that its value is bounding the NEW field and giving the
    // redaction path (below) something well-defined to redact.
    let goal_field = out
        .split("\"goal\":\"")
        .nth(1)
        .and_then(|t| t.split('"').next())
        .expect("an abstention goal field");
    assert!(
        goal_field.chars().count() <= 300,
        "the echoed goal is capped (got {} chars): {goal_field}",
        goal_field.chars().count()
    );
}

#[test]
fn a_sensitive_channel_withholds_the_echoed_payload_but_keeps_the_reason() {
    let dir = scratch("redact");
    let p = write(
        &dir,
        "case.adj",
        "relate seen(a, b)\n    source \"A seed.\"\n    trust empirical\n\
         ? seen(patient_free_text_secret, $V)\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&p)
        .env("ADJ_SENSITIVE_INPUT", "1")
        .output()
        .expect("run adj-lang-cli");
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();

    // THE WHOLE DOCUMENT, not just the abstention object.
    //
    // The first version of this assertion was a disjunction —
    // `!contains(secret) || !contains("\"goal\":\"patient")` — whose second arm
    // is satisfied by the redaction alone. It therefore passed while the secret
    // still appeared three times in the surrounding `queries`/`query` echoes.
    // A test that cannot fail for the reason it exists is worse than no test:
    // it certifies the thing it is not checking.
    assert!(
        !s.contains("patient_free_text_secret"),
        "the caller's text must not appear ANYWHERE in the artifact — the \
         abstention object claiming redaction while a sibling field reprints \
         the value is worse than no redaction: {s}"
    );
    assert!(s.contains("[redacted]"), "redaction marker present: {s}");
    // … but the abstention stays ACTIONABLE: you still learn what went wrong.
    assert!(
        s.contains("\"reason\":\"no_grounded_support\""),
        "redaction must not cost the reason: {s}"
    );
    assert!(
        s.contains("no derivation of this goal"),
        "nor the explanation: {s}"
    );
}

// ---------------------------------------------------------------------------
// (7) The sensitivity toggle FAILS CLOSED on an unrecognized value.
//
//     Found by round 2. `ADJ_SENSITIVE_INPUT=yes` / `on` / `enabled` silently
//     produced UNREDACTED output, because only `1` and `true` were accepted.
//     A security toggle whose misspelling is indistinguishable from being unset
//     is a footgun for exactly the deployment it protects.
// ---------------------------------------------------------------------------

#[test]
fn an_unrecognized_sensitivity_value_redacts_rather_than_leaking() {
    let dir = scratch("failclosed");
    let p = write(
        &dir,
        "case.adj",
        "relate seen(a, b)\n    source \"A seed.\"\n    trust empirical\n\
         ? seen(patient_free_text_secret, $V)\n",
    );
    for value in ["yes", "on", "enabled", "pls-redact-this"] {
        let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
            .arg(&p)
            .env("ADJ_SENSITIVE_INPUT", value)
            .output()
            .expect("run adj-lang-cli");
        assert!(out.status.success(), "ADJ_SENSITIVE_INPUT={value}");
        let s = String::from_utf8(out.stdout).unwrap();
        assert!(
            !s.contains("patient_free_text_secret"),
            "ADJ_SENSITIVE_INPUT={value} must not leak: {s}"
        );
    }

    // An explicit FALSE spelling is honoured — fail-closed must not mean
    // "ignore the operator", or the toggle becomes unusable.
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&p)
        .env("ADJ_SENSITIVE_INPUT", "no")
        .output()
        .expect("run adj-lang-cli");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(
        s.contains("patient_free_text_secret"),
        "an explicit false value must disable redaction: {s}"
    );
}
