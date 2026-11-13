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
    generate_package_report, verify_assurance_report, verify_evidence, verify_profile, AssuranceReport,
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

// ---- a suite that ran nothing, or that omits the bound it was measured against ----
//
// CWE-409 review finding 2: `decompression_bounds` read manifest fields only, so a suite with
// `cases = 0`, or an `output-bound` entry with no `max_observed_heap` / `bound_at_max`
// (silently skipped), still counted as passing evidence.  Each of these must stop the report.

#[test]
fn an_output_bound_suite_that_ran_zero_cases_stops_the_report() {
    for kind in ["exhaustive", "property"] {
        let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
        let t = ev
            .tests
            .iter_mut()
            .find(|t| t.kind == kind && t.claim == "decompress/output-bound")
            .unwrap();
        t.cases = 0;
        let err = lzw_report(&ev).expect_err("a suite that ran nothing is not evidence");
        assert!(err.contains("CWE-409") && err.contains("ran no cases"), "{kind}: {err}");
    }
    // A fuzz suite counts its `iterations`, not `cases`.
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let t = ev.tests.iter_mut().find(|t| t.kind == "fuzz" && t.claim == "compress/output-bound").unwrap();
    t.iterations = 0;
    t.cases = 0;
    let err = lzw_report(&ev).expect_err("a fuzz suite with zero iterations is not evidence");
    assert!(err.contains("CWE-409") && err.contains("ran no cases"), "{err}");
}

#[test]
fn an_output_bound_entry_missing_its_observed_peak_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let t = ev.tests.iter_mut().find(|t| t.kind == "property" && t.claim == "decompress/output-bound").unwrap();
    t.max_observed_heap = None;
    let err = lzw_report(&ev).expect_err("an unobserved output-bound entry is not evidence");
    assert!(err.contains("CWE-409") && err.contains("max_observed_heap"), "{err}");
}

#[test]
fn an_output_bound_entry_missing_its_bound_stops_the_report() {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    let t = ev.tests.iter_mut().find(|t| t.kind == "fuzz" && t.claim == "compress/output-bound").unwrap();
    t.bound_at_max = None;
    let err = lzw_report(&ev).expect_err("a peak with no bound to compare to is not evidence");
    assert!(err.contains("CWE-409") && err.contains("bound_at_max"), "{err}");
}

#[test]
fn the_facts_that_step_6c_re_derives_flag_both_holes() {
    // `verify_evidence` step 6c reads these same facts, so a discharge is refused whenever they
    // are unproven.  (An earlier rung/claim comparison also notices a changed case count, so the
    // facts are asserted directly rather than through the first error `verify_evidence` hits.)
    let (_, mut ev) = good();
    ev.tests.iter_mut().find(|t| t.kind == "exhaustive" && t.claim == "compress/output-bound").unwrap().cases = 0;
    let facts = ev.decompression_bounds();
    assert!(facts.unproven.iter().any(|u| u.contains("ran no cases")), "{:?}", facts.unproven);
    let (_, mut ev) = good();
    ev.tests.iter_mut().find(|t| t.kind == "fuzz" && t.claim == "decompress/output-bound").unwrap().bound_at_max = None;
    let facts = ev.decompression_bounds();
    assert!(facts.unproven.iter().any(|u| u.contains("bound_at_max")), "{:?}", facts.unproven);
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

// ---- the audit profile is recorded in the report and checked ----------------
//
// CWE-409 review finding 1: the profile was not in the report, so a decompressor whose Warrant
// document has no `output` row, generated with the default `comparison` profile, produced and
// verified an `Excluded` CWE-409 row; nothing tied "this package decompresses" to the profile.

/// lzw's evidence with the `output` rows removed: a decompressor whose document forgot them.
fn lzw_without_output_rows() -> Evidence {
    let mut ev = Evidence::load(&pkg_dir("lzw")).unwrap();
    for op in ev.spec.ops.iter_mut() {
        op.has_output_bound = false;
    }
    ev
}

fn generate_with(ev: &Evidence, profile: AuditProfile) -> AssuranceReport {
    generate_package_report("rust/lzw", "0.1.0", "c0ffee", &WeaknessRegistry::embedded(), ev, profile)
        .expect("report generates")
}

#[test]
fn the_report_records_its_profile_and_the_json_round_trips_it() {
    let (report, _) = good();
    assert_eq!(report.profile, AuditProfile::LosslessCompression);
    let json = report.to_json_string();
    assert!(json.contains("\"profile\": \"lossless-compression\""), "{json}");
    let back = AssuranceReport::from_json_str(&json).unwrap();
    assert_eq!(back.profile, AuditProfile::LosslessCompression);
    assert_eq!(back.to_json_string(), json, "byte-stable");
}

#[test]
fn a_report_with_no_or_an_unknown_profile_is_refused() {
    let (report, _) = good();
    let json = report.to_json_string();
    let missing = json.replace("  \"profile\": \"lossless-compression\",\n", "");
    assert_ne!(missing, json, "the fixture must remove the line");
    assert!(AssuranceReport::from_json_str(&missing).is_err(), "a pre-profile report cannot be checked");
    let unknown = json.replace("lossless-compression\",", "lossless-compressionx\",");
    assert!(AssuranceReport::from_json_str(&unknown).is_err());
}

#[test]
fn check_rejects_an_excluded_cwe_409_row_under_the_lossless_compression_profile() {
    // Control: the honest comparison-profile report of the output-row-less document is excluded and
    // verifies (that is the old hole: nothing says the package decompresses)...
    let ev = lzw_without_output_rows();
    let mut report = generate_with(&ev, AuditProfile::Comparison);
    let row = report.weaknesses.iter().find(|w| w.cwe == 409).unwrap();
    assert_eq!(row.status, WeaknessStatus::Excluded);
    verify_evidence(&report, &ev).expect("a comparison-profile exclusion is internally consistent");
    // ...but the same row in a report recorded as lossless-compression is refused.
    report.profile = AuditProfile::LosslessCompression;
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains("CWE-409") && m.contains("lossless-compression")),
        "{:?}",
        verify_evidence(&report, &ev)
    );
}

#[test]
fn check_rejects_a_recorded_profile_that_differs_from_the_requested_one() {
    // The generator under the default profile records `comparison`; BUILD checks with
    // `--profile lossless-compression`, so this is the mismatch it must catch.
    let ev = lzw_without_output_rows();
    let report = generate_with(&ev, AuditProfile::Comparison);
    assert!(matches!(
        verify_profile(&report, Some(AuditProfile::LosslessCompression)),
        Err(E::EvidenceMismatch(m)) if m.contains("comparison") && m.contains("lossless-compression")
    ));
    assert!(verify_profile(&report, Some(AuditProfile::Comparison)).is_ok());
    assert!(verify_profile(&report, None).is_ok(), "no profile requested: the recorded one stands");
}
