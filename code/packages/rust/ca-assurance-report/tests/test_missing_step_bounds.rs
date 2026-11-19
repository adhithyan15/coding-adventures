//! VER-P048: missing declared step bounds produce derived, report-only CWE-400
//! and CWE-407 dispositions. A profile can say applicable, never approve.

mod common;

use coding_adventures_ca_assurance_report::evidence::{verify_evidence, Evidence};
use coding_adventures_ca_assurance_report::profile::AuditProfile;
use coding_adventures_ca_assurance_report::{
    build_weakness_matrix, generate_package_report, verify_rederivation, WeaknessRegistry,
    WeaknessStatus,
};
use common::Sandbox;

fn fixed_point(tag: &str) -> Sandbox {
    Sandbox::for_package(tag, "fixed-point")
}

fn row(
    rows: &[coding_adventures_ca_assurance_report::WeaknessRow],
    cwe: u32,
) -> &coding_adventures_ca_assurance_report::WeaknessRow {
    rows.iter().find(|row| row.cwe == cwe).unwrap()
}

#[test]
fn actual_unbounded_fixed_point_derives_two_unapproved_typed_exemptions() {
    let sb = fixed_point("p048-actual");
    let ev = Evidence::load(&sb.pkg).expect("real evidence inputs load");
    let ctx = ev.audit_context("rust/fixed-point");
    assert_eq!(ctx.evidence.missing_step_bound_ops.len(), 10);
    assert!(!ctx.evidence.all_ops_step_bounded);

    let report = generate_package_report(
        "rust/fixed-point",
        "0.1.0",
        "local",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .expect("missing bounds are accounted as report-only exemptions");
    for cwe in [400, 407] {
        let weakness = row(&report.weaknesses, cwe);
        assert_eq!(weakness.status, WeaknessStatus::Exemption);
        assert_eq!(weakness.approved, Some(false));
        assert!(weakness.evidence.is_empty());
        assert!(weakness.reason.contains("kind=missing-step-bounds"));
        assert!(weakness.reason.contains("q16_checked_add"));
        assert!(weakness.reason.contains("q16_to_integer"));
    }
    for (cwe, id) in [
        (400, "EXEMPT-CWE-400-MISSING-STEP-BOUNDS"),
        (407, "EXEMPT-CWE-407-MISSING-STEP-BOUNDS"),
    ] {
        let exemption = report
            .exemptions
            .iter()
            .find(|entry| entry.id == id)
            .unwrap();
        assert_eq!(exemption.cwe, cwe);
        assert_eq!(exemption.kind, "missing-step-bounds");
        assert!(!exemption.approved);
        assert!(exemption.evidence.is_empty());
    }
    assert_eq!(report.verdict, "report-only");
    verify_evidence(&report, &ev).expect("generated dispositions rederive exactly");
    verify_rederivation(&report, &ev, &WeaknessRegistry::embedded())
        .expect("matrix and computed exemptions rederive exactly");

    let mut forged = report.clone();
    forged
        .exemptions
        .iter_mut()
        .find(|entry| entry.id == "EXEMPT-CWE-400-MISSING-STEP-BOUNDS")
        .unwrap()
        .approved = true;
    assert!(
        verify_rederivation(&forged, &ev, &WeaknessRegistry::embedded()).is_err(),
        "approval is never author input"
    );

    let mut forged = report.clone();
    forged
        .exemptions
        .iter_mut()
        .find(|entry| entry.id == "EXEMPT-CWE-407-MISSING-STEP-BOUNDS")
        .unwrap()
        .evidence
        .push("warrant/fixed_point_q16/proof/q16_checked_add/steps-bound".into());
    assert!(
        verify_rederivation(&forged, &ev, &WeaknessRegistry::embedded()).is_err(),
        "fallback evidence must stay empty"
    );

    let mut forged = report.clone();
    forged
        .exemptions
        .retain(|entry| entry.id != "EXEMPT-CWE-407-MISSING-STEP-BOUNDS");
    assert!(
        verify_rederivation(&forged, &ev, &WeaknessRegistry::embedded()).is_err(),
        "the CWE-407 record is required"
    );
}

#[test]
fn partial_missing_coverage_precedes_otherwise_passing_cwe_400_evidence() {
    let sb = fixed_point("p048-partial");
    let ev = Evidence::load(&sb.pkg).unwrap();
    let mut ctx = ev.audit_context("rust/fixed-point");
    ctx.evidence.missing_step_bound_ops = vec!["q16_checked_sub".into()];
    ctx.evidence.all_ops_step_bounded = false;
    // Model a live step proof and a zero-allocation proof on the covered
    // operation. The uncovered operation still blocks package-level closure.
    ctx.evidence.step_bound_ids =
        vec!["warrant/fixed_point_q16/proof/q16_checked_add/steps-bound".into()];
    ctx.evidence.step_unproven.clear();
    ctx.has_zero_alloc_proof = true;

    let rows = build_weakness_matrix(&WeaknessRegistry::embedded(), &ctx).unwrap();
    for cwe in [400, 407] {
        let weakness = row(&rows, cwe);
        assert_eq!(weakness.status, WeaknessStatus::Exemption);
        assert_eq!(weakness.approved, Some(false));
        assert!(weakness.evidence.is_empty());
        assert!(weakness.reason.contains("q16_checked_sub"));
    }
}

#[test]
fn applicable_profile_requires_a_real_missing_step_bound_set() {
    let sb = fixed_point("p048-no-missing");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    for op in &mut ev.spec.ops {
        op.has_steps_bound = true;
    }
    let err = generate_package_report(
        "rust/fixed-point",
        "0.1.0",
        "local",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap_err();
    assert!(err.to_string().contains("applicable"), "{err}");

    ev.spec.ops.clear();
    assert!(generate_package_report(
        "rust/fixed-point",
        "0.1.0",
        "local",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .is_err());
}

#[test]
fn cwe_407_exclusion_with_missing_bounds_remains_invalid() {
    let sb = fixed_point("p048-excluded-gap");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    let profile_text = std::fs::read_to_string(sb.pkg.join("assurance-profile.json")).unwrap();
    ev.profile =
        AuditProfile::from_bytes(profile_text.replace("applicable", "excluded").as_bytes())
            .unwrap();
    let err = generate_package_report(
        "rust/fixed-point",
        "0.1.0",
        "local",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("CWE-407") && err.to_string().contains("steps"),
        "{err}"
    );
}

#[test]
fn manifest_note_text_cannot_select_a_missing_bounds_exemption() {
    let sb = fixed_point("p048-note-marker");
    let mut ev = Evidence::load(&sb.pkg).unwrap();
    for op in &mut ev.spec.ops {
        op.has_steps_bound = true;
    }
    let profile_text = std::fs::read_to_string(sb.pkg.join("assurance-profile.json")).unwrap();
    ev.profile =
        AuditProfile::from_bytes(profile_text.replace("applicable", "excluded").as_bytes())
            .unwrap();
    ev.proofs.push(
        coding_adventures_ca_assurance_report::evidence::ProofEntry {
            id: "warrant/fixed_point_q16/proof/q16_checked_add/steps-bound".into(),
            op: "q16_checked_add".into(),
            function: "checked_add".into(),
            kind: "steps-bound".into(),
            label: "not-live".into(),
            verdict: "not-live".into(),
            max_len: 0,
            certificates: vec![],
            obligation_sha256: String::new(),
            note: "test marker kind=missing-step-bounds; content is untrusted".into(),
        },
    );
    let report = generate_package_report(
        "rust/fixed-point",
        "0.1.0",
        "local",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .expect("the legacy not-live branch remains reportable");
    let cwe_400 = report
        .exemptions
        .iter()
        .find(|entry| entry.cwe == 400)
        .unwrap();
    assert_eq!(cwe_400.id, "EXEMPT-CWE-400-NO-STEP-PROOF");
    assert_eq!(cwe_400.kind, "no-live-proof");
    assert!(!cwe_400.approved);
    assert!(cwe_400.rationale.contains("kind=missing-step-bounds;"));
}
