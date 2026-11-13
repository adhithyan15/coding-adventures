//! # The `rust/lzw` report: CWE-409 derived from its own bound rows
//!
//! The end-to-end half of `test_decompression_bomb.rs`: the real `lzw` manifests
//! (`evidence/test-manifest.json`, `proof-manifest.json`, the effect certificate)
//! produce a report whose CWE-409 row is *discharged* from the document's `output`
//! and `heap` rows, the checked-in `assurance_report.json` is exactly what they
//! derive, generation fails closed when a manifest loses a fact, and `--check`
//! refuses a report that disagrees with the manifests (a false exclusion, a
//! discharge the manifest does not support, a partial citation, an exemption).
//!
//! Every test that mutates evidence does it on an in-memory copy; nothing on disk
//! changes.

use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::{
    generate_package_report, verify_assurance_report, verify_evidence, AssuranceReport,
    AuditProfile, CheckVerdict, Evidence, EvidenceCheckError as E, WeaknessRegistry, WeaknessStatus,
};

fn pkg_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

// ---------------------------------------------------------------------------
// The real adopter, end to end
// ---------------------------------------------------------------------------

fn lzw_report(ev: &Evidence) -> Result<AssuranceReport, String> {
    generate_package_report(
        "rust/lzw",
        "0.1.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        ev,
        AuditProfile::LosslessCompression,
    )
    .map_err(|e| e.to_string())
}

#[test]
fn the_lzw_report_discharges_cwe_409_from_its_own_bound_rows_and_matches_the_file_on_disk() {
    let dir = pkg_dir("lzw");
    let ev = Evidence::load(&dir).expect("lzw manifests load");
    let facts = ev.decompression_bounds();
    assert_eq!(facts.ops, vec!["compress".to_string(), "decompress".to_string()], "both operations declare `output`");
    assert!(facts.unproven.is_empty(), "{:?}", facts.unproven);
    assert_eq!(facts.ids.len(), 6, "exhaustive + property + fuzz for each of two operations");
    assert!(facts.max_observed_heap > 0 && facts.max_observed_heap <= facts.bound_at_max);

    let report = lzw_report(&ev).expect("report derives from the lzw manifests");
    verify_evidence(&report, &ev).expect("a derived report must verify");
    assert!(matches!(
        verify_assurance_report(&report, &WeaknessRegistry::embedded()),
        Ok(CheckVerdict::ReportOnly(_))
    ));
    let r409 = report.weaknesses.iter().find(|w| w.cwe == 409).unwrap();
    assert_eq!(r409.status, WeaknessStatus::Discharged, "never the false exclusion");
    assert_eq!(r409.evidence, facts.ids);

    let text = fs::read_to_string(dir.join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_package_report(
        "rust/lzw",
        "0.1.0",
        &on_disk.commit,
        &WeaknessRegistry::embedded(),
        &ev,
        AuditProfile::LosslessCompression,
    )
    .unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

#[test]
fn the_lzw_static_rows_describe_a_compressor() {
    let ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let report = lzw_report(&ev).unwrap();
    for cwe in [326, 327, 407] {
        let row = report.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        for foreign in ["LEB128", "GF(2^8)", "comparison", "integer encoding", "logic gates"] {
            assert!(!row.reason.contains(foreign), "CWE-{cwe} names {foreign}: {}", row.reason);
        }
    }
    let r407 = report.weaknesses.iter().find(|w| w.cwe == 407).unwrap();
    assert!(r407.reason.contains("output cap"), "{}", r407.reason);
}

// ---- generation fails closed when the manifests lose a fact ----------------

#[test]
fn dropping_an_output_bound_suite_from_the_manifest_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let before = ev.tests.len();
    ev.tests.retain(|t| !(t.kind == "fuzz" && t.op == "decompress" && t.claim == "decompress/output-bound"));
    assert!(ev.tests.len() < before, "the fixture must remove something");
    let err = lzw_report(&ev).expect_err("no fuzz output-bound suite: open row");
    assert!(err.contains("CWE-409") && err.contains("no fuzz suite"), "{err}");
}

#[test]
fn a_failing_output_bound_suite_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    for t in ev.tests.iter_mut().filter(|t| t.kind == "exhaustive" && t.claim == "decompress/output-bound") {
        t.verdict = "fail".into();
    }
    let err = lzw_report(&ev).expect_err("a failing bound is not evidence");
    assert!(err.contains("CWE-409") && err.contains("is fail"), "{err}");
}

#[test]
fn an_observed_peak_over_its_bound_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let t = ev.tests.iter_mut().find(|t| t.op == "decompress" && t.max_observed_heap.is_some()).unwrap();
    t.max_observed_heap = Some(t.bound_at_max.unwrap() + 1);
    let err = lzw_report(&ev).expect_err("a peak over the heap bound is a failure");
    assert!(err.contains("CWE-409") && err.contains("over its"), "{err}");
}

#[test]
fn an_operation_that_was_never_observed_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    for t in ev.tests.iter_mut().filter(|t| t.op == "compress") {
        t.max_observed_heap = None;
        t.bound_at_max = None;
    }
    let err = lzw_report(&ev).expect_err("a heap row nobody observed is not evidence");
    assert!(err.contains("no entry was observed"), "{err}");
}

#[test]
fn an_output_row_with_no_heap_row_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    for op in ev.spec.ops.iter_mut().filter(|o| o.name == "decompress") {
        op.has_heap_bound = false;
    }
    let err = lzw_report(&ev).expect_err("an output bound without a heap bound is half a bound");
    assert!(err.contains("no `heap` bound row"), "{err}");
}

// ---- --check refuses a report that disagrees with the manifests -------------

fn good() -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let report = lzw_report(&ev).unwrap();
    (report, ev)
}

/// lzw's report with its CWE-409 row rewritten to the old text every package used to get.
fn with_false_exclusion() -> (AssuranceReport, Evidence) {
    let (mut report, ev) = good();
    let row = report.weaknesses.iter_mut().find(|w| w.cwe == 409).unwrap();
    row.status = WeaknessStatus::Excluded;
    row.reason = "pure computation: package implements no decompression or archive decoding routines".into();
    row.evidence = vec!["unverified::ast-audit::no-compression-routines".into()];
    (report, ev)
}

#[test]
fn check_rejects_a_false_exclusion_of_cwe_409() {
    // Layer 1, the assumption rules: the old text cites an unverified attestation
    // whose catalogue entry the lzw report does not list, so it is refused before
    // any CWE-409 rule runs.
    let (report, ev) = with_false_exclusion();
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::Assumption(_))),
        "{:?}",
        verify_evidence(&report, &ev)
    );
    // Layer 2, step 6c: a forger who also adds the catalogue entry still fails,
    // because the document bounds an output and so the class is not excludable.
    let (mut report, ev) = with_false_exclusion();
    report.assumptions.push("att.ast-audit.no-compression-routines".into());
    report.assumptions.sort();
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains("CWE-409") && m.contains("excluded")),
        "{:?}",
        verify_evidence(&report, &ev)
    );
}

#[test]
fn check_rejects_a_discharge_the_manifest_does_not_support() {
    let (report, mut ev) = good();
    // The report is untouched; the manifest loses its fuzz suite for decompress.
    ev.tests.retain(|t| !(t.kind == "fuzz" && t.op == "decompress" && t.claim == "decompress/output-bound"));
    let err = verify_evidence(&report, &ev).expect_err("the report cites an id the manifest no longer holds");
    // Either the dangling id or the incomplete bound evidence: both are refusals.
    assert!(matches!(err, E::UnknownEvidence(_) | E::EvidenceMismatch(_)), "{err}");
}

#[test]
fn check_rejects_a_discharge_that_cites_only_some_of_the_suites() {
    let (mut report, ev) = good();
    let row = report.weaknesses.iter_mut().find(|w| w.cwe == 409).unwrap();
    row.evidence.pop();
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains("CWE-409 cites")),
        "a discharge must cite every output-bound suite"
    );
}

#[test]
fn check_rejects_a_discharge_when_a_suite_failed() {
    let (report, mut ev) = good();
    for t in ev.tests.iter_mut().filter(|t| t.kind == "property" && t.claim == "compress/output-bound") {
        t.verdict = "fail".into();
    }
    assert!(verify_evidence(&report, &ev).is_err(), "a failing suite cannot support a discharge");
}

#[test]
fn check_rejects_an_exemption_for_cwe_409() {
    let (mut report, ev) = good();
    let row = report.weaknesses.iter_mut().find(|w| w.cwe == 409).unwrap();
    row.status = WeaknessStatus::Exemption;
    row.approved = Some(false);
    assert!(verify_evidence(&report, &ev).is_err());
}

