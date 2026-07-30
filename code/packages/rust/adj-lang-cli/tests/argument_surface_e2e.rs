//! End-to-end tests for the **`argument` surface** (ADJ-ARGUMENT-IR ADR-2), driven
//! through the built `adj-lang-cli` binary.
//!
//! The whole point of the argument construct is that it DESUGARS AWAY into the
//! existing substrate — premises → provenanced facts, inferences → rules — so the
//! engine *derives* the thesis by chaining the inference rules, and the proof carries
//! every premise's byte citation. These tests prove that path works end to end (a
//! multi-step argument derives its thesis, cited), and that the two things the surface
//! cannot express — a dangling `from` reference and an unknown premise kind — are clean
//! compile errors, never a panic or a silently-dropped step.

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_arg_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(src: &str, tag: &str) -> (bool, String) {
    let dir = scratch(tag);
    let p = dir.join("case.adj");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&p)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

// A three-premise, two-step argument (the ADJ-ARGUMENT-IR §8 axle-fatigue example):
// operating stress > fatigue limit ⇒ the limit is exceeded; that + beach marks ⇒ the
// mechanism is fatigue. The thesis chains p1&p2 → s1, then s1&p3 → s2.
const AXLE: &str = "argument axle_failure {\n\
    premise p1 : extracted operating_stress(axle, 420) source \"operating stress (420 MPa)\" trust authoritative\n\
    premise p2 : extracted fatigue_limit(axle, 380) source \"its fatigue limit (380 MPa)\" trust authoritative\n\
    premise p3 : extracted shows(surface, beach_marks) source \"beach marks\" trust authoritative\n\
    infer s1 : because conclude exceeds_limit(axle) from p1, p2 source \"exceeded its fatigue limit\" trust authoritative\n\
    infer s2 : therefore conclude mechanism(axle, fatigue) from s1, p3 source \"beach marks confirm a fatigue mechanism\" trust authoritative\n\
}\n\
? mechanism(axle, $M)\n";

#[test]
fn a_multistep_argument_derives_its_thesis_from_chained_premises() {
    let (ok, out) = run(AXLE, "derive");
    assert!(ok, "a well-formed argument must compile and run:\n{out}");
    // The engine derived the thesis by chaining s2 ← (s1 ← p1,p2), p3.
    assert!(
        out.contains("\"M\":\"fatigue\""),
        "the thesis `mechanism(axle, fatigue)` must be DERIVED, not asserted:\n{out}"
    );
    // Chaining reached back through both inferences to the grounding premises, so the
    // answer carries their byte citations — the argument is auditable to its sources.
    assert!(
        out.contains("operating stress (420 MPa)") && out.contains("beach marks"),
        "the derived thesis must carry the premises' citations:\n{out}"
    );
}

#[test]
fn an_inference_referencing_an_unknown_premise_is_a_compile_error() {
    // s1 cites `px`, which no premise binds. The lowerer must reject it cleanly (a
    // dangling `from` would otherwise silently produce a rule that can never fire).
    let src = "argument bad_ref {\n\
        premise p1 : extracted a(x) source \"a\" trust authoritative\n\
        infer s1 : because conclude c(x) from p1, px source \"w\" trust authoritative\n\
    }\n\
    ? c(x)\n";
    let (_ok, out) = run(src, "badref");
    // A compile error is reported as `{"error":"Lower(...)"}`; the dangling reference
    // must be named, and no answer produced — never a rule that silently never fires.
    assert!(
        out.contains("ArgUnknownReference") && out.contains("px"),
        "a dangling `from` reference must be a named compile error:\n{out}"
    );
    assert!(
        !out.contains("\"answers\""),
        "and no answer may be produced for a program that failed to compile:\n{out}"
    );
}

#[test]
fn a_premise_with_an_unknown_kind_is_a_compile_error() {
    // `assumed` is not one of extracted | imported | inferred.
    let src = "argument bad_kind {\n\
        premise p1 : assumed a(x) source \"a\" trust authoritative\n\
    }\n\
    ? a(x)\n";
    let (_ok, out) = run(src, "badkind");
    assert!(
        out.contains("ArgUnknownPremiseKind") && out.contains("assumed"),
        "an unknown premise kind must be a named compile error:\n{out}"
    );
    assert!(
        !out.contains("\"answers\""),
        "and no answer may be produced for a program that failed to compile:\n{out}"
    );
}

#[test]
fn two_elements_sharing_a_name_is_a_compile_error() {
    // `p1` is bound twice; a `from p1` reference would then be ambiguous, so a duplicate
    // name is rejected rather than silently resolving to one of them.
    let src = "argument dup {\n\
        premise p1 : extracted a(x) source \"a\" trust authoritative\n\
        premise p1 : extracted b(x) source \"b\" trust authoritative\n\
    }\n\
    ? a(x)\n";
    let (_ok, out) = run(src, "dup");
    assert!(
        out.contains("ArgDuplicateName") && out.contains("p1"),
        "a duplicated element name must be a named compile error:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// ADR-3 — the structural grounding gate: a shipped argument must be SOURCED.
// ---------------------------------------------------------------------------

#[test]
fn an_unsourced_premise_is_a_compile_error() {
    // A premise with no `source` cite is un-grounded — it names bytes it never quotes.
    // The grounding gate (§3) rejects it, the same "must be sourced" lint table/
    // statemachine/formula enforce; it is the precondition for the ADR-4 byte-anchor.
    let src = "argument u {\n\
        premise p1 : extracted a(x)\n\
    }\n\
    ? a(x)\n";
    let (_ok, out) = run(src, "unsourced_prem");
    assert!(
        out.contains("ArgMissingProvenance") && out.contains("p1"),
        "an un-sourced premise must be a named compile error:\n{out}"
    );
}

#[test]
fn an_unwarranted_inference_is_a_compile_error() {
    // An inference with no `source` carries no WARRANT — an un-grounded reasoning step.
    let src = "argument u {\n\
        premise p1 : extracted a(x) source \"a\" trust authoritative\n\
        infer s1 : because conclude c(x) from p1\n\
    }\n\
    ? c(x)\n";
    let (_ok, out) = run(src, "unwarranted");
    assert!(
        out.contains("ArgMissingProvenance") && out.contains("s1"),
        "an un-warranted inference must be a named compile error:\n{out}"
    );
}
