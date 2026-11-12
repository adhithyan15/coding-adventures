//! VER04h: the report generator is package-agnostic.  `rust/gf256` is the
//! second package; its proof manifest records every constant-time and
//! step-bound obligation `not-live` (the WIR front end refuses the source), so
//! these tests pin what the gate does with an *acknowledged absence* of proof:
//! no proof block, an unapproved exemption that quotes the recorded blocker,
//! rung E5 `not-live`, and fail-closed behaviour when the absence is not
//! acknowledged or a number disagrees with a manifest.

use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::{
    generate_ct_compare_report, generate_package_report, verify_assurance_report, verify_evidence,
    AssuranceReport, AuditProfile, CheckVerdict, Evidence, EvidenceCheckError as E,
    WeaknessRegistry, WeaknessStatus,
};

fn gf_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../gf256")
}

fn build(ev: &Evidence) -> AssuranceReport {
    generate_package_report(
        "rust/gf256",
        "0.2.1",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        ev,
        AuditProfile::FieldArithmetic,
    )
    .expect("report derives from the gf256 manifests")
}

/// A throw-away copy of `../gf256` (manifests, spec, source, certificate) that
/// a test may corrupt without touching tracked files.
struct Copy {
    root: PathBuf,
    pkg: PathBuf,
}

impl Copy {
    fn new(tag: &str) -> Copy {
        let real = gf_dir();
        let root = std::env::temp_dir().join(format!("ver04h-gf256-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("code/packages/rust/gf256");
        fs::create_dir_all(pkg.join("evidence")).unwrap();
        for f in ["test-manifest.json", "proof-manifest.json"] {
            fs::copy(real.join("evidence").join(f), pkg.join("evidence").join(f)).unwrap();
        }
        // Copy every source file: the evidence binds the digest of all of
        // `src/**/*.rs` (e.g. the generated `capabilities.rs` too), so a copy
        // of `lib.rs` alone would read as stale before the edit under test.
        fs::create_dir_all(pkg.join("src")).unwrap();
        for entry in fs::read_dir(real.join("src")).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), pkg.join("src").join(entry.file_name())).unwrap();
            }
        }
        fs::copy(real.join("effect_certificate.json"), pkg.join("effect_certificate.json")).unwrap();
        fs::copy(real.join("evidence.bindings"), pkg.join("evidence.bindings")).unwrap();
        fs::create_dir_all(root.join("code/specs")).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/gf256.warrant"),
            root.join("code/specs/gf256.warrant"),
        )
        .unwrap();
        let assumptions = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assumptions");
        fs::create_dir_all(root.join("code/assumptions")).unwrap();
        for f in ["catalogue-v1.json", "evidence-kinds.json", "non-claims.json"] {
            fs::copy(assumptions.join(f), root.join("code/assumptions").join(f)).unwrap();
        }
        Copy { root, pkg }
    }
    fn edit(&self, rel: &str, from: &str, to: &str) {
        let p = self.pkg.join(rel);
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains(from), "{rel} must contain {from:?}");
        fs::write(p, s.replacen(from, to, 1)).unwrap();
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn gf256_report_derives_verifies_and_matches_the_checked_in_file() {
    let ev = Evidence::load(&gf_dir()).expect("gf256 manifests load");
    let report = build(&ev);
    verify_evidence(&report, &ev).expect("a derived report must verify");
    assert!(matches!(
        verify_assurance_report(&report, &WeaknessRegistry::embedded()),
        Ok(CheckVerdict::ReportOnly(_))
    ));
    assert_eq!(report.package, "rust/gf256");
    assert_eq!(report.claims.len(), 6, "one claim per Warrant operation");

    let text = fs::read_to_string(gf_dir().join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_package_report(
        "rust/gf256",
        "0.2.1",
        &on_disk.commit,
        &WeaknessRegistry::embedded(),
        &ev,
        AuditProfile::FieldArithmetic,
    )
    .unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

#[test]
fn not_live_proofs_are_acknowledged_not_labelled() {
    let ev = Evidence::load(&gf_dir()).unwrap();
    let report = build(&ev);
    for c in &report.claims {
        assert!(c.proof.is_none(), "{}: a not-live obligation must not become a proof label", c.symbol);
        assert!(
            c.evidence.iter().all(|id| !id.contains("/proof/")),
            "{}: not-live obligations are not citable evidence",
            c.symbol
        );
        assert!(c.exhaustive.as_ref().unwrap().cases > 0);
        assert!(c.fuzz.as_ref().unwrap().iterations > 0);
    }
    // Exact case counts come from the manifest, summed per operation.
    let multiply = report.claims.iter().find(|c| c.symbol == "multiply").unwrap();
    let want: u64 = ev
        .tests
        .iter()
        .filter(|t| t.kind == "exhaustive" && t.op == "multiply")
        .map(|t| t.cases)
        .sum();
    assert_eq!(multiply.exhaustive.as_ref().unwrap().cases, want);
    assert!(want >= 2 * (1 << 24), "associativity and distributivity are exhaustive over U8^3");

    assert_eq!(report.rungs.e5, "not-live");
    assert_eq!(report.rungs.e1, "pass");
    assert_eq!(report.rungs.e3, "pass");
    assert_eq!(report.rungs.e4, "pass");
    assert_eq!(report.verdict, "report-only");

    for cwe in [208, 400] {
        let row = report.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        assert_eq!(row.status, WeaknessStatus::Exemption, "CWE-{cwe}");
        assert_eq!(row.approved, Some(false), "CWE-{cwe} is unapproved");
        assert!(row.reason.contains("parse refused"), "CWE-{cwe} quotes the recorded blocker: {}", row.reason);
        assert!(row.evidence.is_empty(), "CWE-{cwe}: nothing verified is cited");
    }
    assert!(report.exemptions.iter().any(|x| x.cwe == 208 && !x.approved));
    assert!(report.exemptions.iter().any(|x| x.id == "EXEMPT-CWE-400-NO-STEP-PROOF"));
    // The profile selects honest sentences for the static rows.
    let r327 = report.weaknesses.iter().find(|w| w.cwe == 327).unwrap();
    assert!(r327.reason.contains("GF(2^8)"), "{}", r327.reason);
}

#[test]
fn the_comparison_profile_still_reproduces_ct_compare_exactly() {
    // Generalising must not move ct-compare's checked-in report.
    let pkg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ct-compare");
    let ev = Evidence::load(&pkg).unwrap();
    let on_disk =
        AssuranceReport::from_json_str(&fs::read_to_string(pkg.join("assurance_report.json")).unwrap()).unwrap();
    let fresh = generate_ct_compare_report(&on_disk.commit, &WeaknessRegistry::embedded(), &ev).unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string());
}

#[test]
fn a_spec_with_secret_labels_and_no_proof_entry_is_uncovered() {
    // Deleting the proof manifest is a load error, and erasing the
    // constant-time entries (rather than recording them not-live) is not an
    // acknowledgement: the spec still declares secret labels, so the gate
    // fails closed.
    let cp = Copy::new("noproof");
    let report = build(&Evidence::load(&gf_dir()).unwrap());
    fs::remove_file(cp.pkg.join("evidence/proof-manifest.json")).unwrap();
    assert!(Evidence::load(&cp.pkg).is_err(), "a missing proof manifest must not load");

    let cp = Copy::new("noct");
    let manifest = fs::read_to_string(cp.pkg.join("evidence/proof-manifest.json")).unwrap();
    assert!(manifest.contains("\"kind\": \"constant-time\""));
    fs::write(
        cp.pkg.join("evidence/proof-manifest.json"),
        manifest.replace("\"kind\": \"constant-time\"", "\"kind\": \"erased\""),
    )
    .unwrap();
    let ev = Evidence::load(&cp.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::UncoveredClaim(m)) if m.contains("constant-time")));
}

#[test]
fn a_stale_proof_manifest_is_rejected_even_when_every_entry_is_not_live() {
    let cp = Copy::new("stale");
    let report = build(&Evidence::load(&gf_dir()).unwrap());
    // Edit the source after the proof manifest recorded its digest.
    let mut src = fs::read_to_string(cp.pkg.join("src/lib.rs")).unwrap();
    src.push_str("\n// edited after the proof run\n");
    fs::write(cp.pkg.join("src/lib.rs"), src).unwrap();
    let ev = Evidence::load(&cp.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::StaleEvidence(m)) if m.contains("src/**/*.rs")));
}

#[test]
fn a_count_that_disagrees_with_the_test_manifest_is_rejected() {
    let cp = Copy::new("count");
    let report = build(&Evidence::load(&gf_dir()).unwrap());
    // Hand-edit one exhaustive case count in the manifest; the report no
    // longer agrees with it.
    let manifest = fs::read_to_string(cp.pkg.join("evidence/test-manifest.json")).unwrap();
    let from = "\"cases\": 16777216";
    assert!(manifest.contains(from), "the 2^24 claims record 16777216 cases");
    cp.edit("evidence/test-manifest.json", from, "\"cases\": 16777215");
    let ev = Evidence::load(&cp.pkg).unwrap();
    assert!(matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains("exhaustive")));
}

#[test]
fn promoting_a_not_live_obligation_to_pass_needs_a_certificate() {
    // Flipping a manifest verdict by hand cannot manufacture a proof: a pass
    // with no certificate has nothing for the DRAT checker to accept.
    let cp = Copy::new("promote");
    let proof = fs::read_to_string(cp.pkg.join("evidence/proof-manifest.json")).unwrap();
    let marker = "\"label\": \"not-live\"";
    assert!(proof.contains(marker));
    let promoted = proof.replace(marker, "\"label\": \"proved\"").replace("\"verdict\": \"not-live\"", "\"verdict\": \"pass\"");
    fs::write(cp.pkg.join("evidence/proof-manifest.json"), promoted).unwrap();
    let ev = Evidence::load(&cp.pkg).unwrap();
    let derived = generate_package_report(
        "rust/gf256",
        "0.2.1",
        "x",
        &WeaknessRegistry::embedded(),
        &ev,
        AuditProfile::FieldArithmetic,
    );
    assert!(derived.is_err(), "no certificates behind the forged `proved` entries");
}
