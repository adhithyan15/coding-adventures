//! # A package with no secret operand (PR-0, item 2; first adopter: wasm-leb128)
//!
//! CWE-208 (observable timing discrepancy) is about secrets.  The matrix used to
//! know two cases: a live constant-time proof (an unapproved gate-level
//! exemption) and a `not-live` one ("applicable and unaddressed").  A Warrant
//! document that labels every operand `public` has *no* constant-time
//! obligation at all, and fell through to `OpenRow`: the report could not be
//! generated for a varint codec, a decompressor or a checksum.
//!
//! The third case is now explicit and fail-closed:
//!
//! ```text
//!  every operand labelled public ──► attestation warrant-spec::all-operands-public
//!  (read from the spec on disk)       (exists only while that is true)
//!            +                                     │
//!  no constant-time obligation  ──────────────────►│
//!                                                  ▼
//!  CWE-208: exemption, approved=false, "not applicable: no secret operands ..."
//!           EXEMPT-CWE-208-NO-SECRET-OPERANDS (kind not-applicable)
//! ```
//!
//! An exemption and not an exclusion, because the registry gives CWE-208 no
//! safe subset (VER15 section 4, rule 2): only a registry safe subset verified
//! by the analyzer may exclude a class.  Everything else keeps its old outcome:
//! a secret label with a not-live proof stays "applicable and unaddressed", and
//! a secret label with no constant-time entry at all stays an open row.  The
//! checker re-derives the case from the spec and the proof manifest, so a
//! report cannot claim it for a package that has secrets.
//!
//! The same file pins the `varint-codec` audit profile, whose static sentences
//! (CWE-326/327/407) describe LEB128 rather than GF(2^8) or comparisons.

use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::{
    build_weakness_matrix, generate_package_report, verify_assurance_report, verify_evidence,
    AssuranceReport, AuditEvidence, AuditProfile, CheckVerdict, Evidence, EvidenceCheckError as E,
    PackageAuditContext, WeaknessRegistry, WeaknessStatus,
};

const ATTEST: &str = "warrant-spec::all-operands-public";
const NA_ID: &str = "EXEMPT-CWE-208-NO-SECRET-OPERANDS";

fn pkg_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn leb_report(ev: &Evidence) -> AssuranceReport {
    generate_package_report(
        "rust/wasm-leb128",
        "0.2.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        ev,
    )
    .expect("report derives from the wasm-leb128 manifests")
}

fn gf_report(ev: &Evidence) -> AssuranceReport {
    generate_package_report(
        "rust/gf256",
        "0.2.1",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        ev,
    )
    .expect("report derives from the gf256 manifests")
}

/// A context for the matrix builder alone: CWE-400 acknowledged not-live so
/// that only the CWE-208 row decides the outcome.
fn ctx(evidence: AuditEvidence) -> PackageAuditContext {
    PackageAuditContext {
        package_id: "rust/example".into(),
        is_safe_rust: true,
        declared_capabilities: vec![],
        has_wir_timing_proof: false,
        has_instruction_benchmarks: false,
        has_zero_alloc_proof: false,
        has_pbt_coverage: true,
        has_fuzz_coverage: true,
        evidence: AuditEvidence {
            profile: AuditProfile::load(&pkg_dir("wasm-leb128")).expect("an adopter's profile loads"),
            all_ops_step_bounded: true,
            step_not_live: vec!["warrant/m/proof/op/steps-bound".into()],
            step_unproven: vec!["warrant/m/proof/op/steps-bound".into()],
            not_live_note: "front end refused".into(),
            ..evidence
        },
    }
}

fn row208(evidence: AuditEvidence) -> Result<coding_adventures_ca_assurance_report::WeaknessRow, String> {
    build_weakness_matrix(&WeaknessRegistry::embedded(), &ctx(evidence))
        .map_err(|e| e.to_string())
        .map(|rows| rows.into_iter().find(|r| r.cwe == 208).expect("CWE-208 row"))
}

// ---------------------------------------------------------------------------
// The matrix rule
// ---------------------------------------------------------------------------

#[test]
fn no_secret_operand_and_no_ct_obligation_is_not_applicable() {
    let row = row208(AuditEvidence {
        no_secret_operand_ids: vec![ATTEST.into()],
        ..AuditEvidence::default()
    })
    .expect("the attested case is a row, not an open row");
    assert_eq!(row.status, WeaknessStatus::Exemption);
    assert_eq!(row.approved, Some(false), "no exemption is approved without a signature");
    assert!(row.reason.starts_with("not applicable: no secret operands"), "{}", row.reason);
    assert_eq!(row.evidence, vec![ATTEST.to_string()], "cites the derived attestation only");
}

#[test]
fn without_the_attestation_the_row_still_fails_closed() {
    // The default evidence has no attestation: an absent fact is never read
    // as "no secrets".
    let err = row208(AuditEvidence::default()).expect_err("no attestation, no CT evidence: open row");
    assert!(err.contains("CWE-208"), "{err}");
}

#[test]
fn a_secret_label_with_a_not_live_proof_stays_applicable_and_unaddressed() {
    // Even if an attestation id were somehow present, a constant-time
    // obligation means there is a secret: the old branch wins.
    let row = row208(AuditEvidence {
        no_secret_operand_ids: vec![ATTEST.into()],
        ct_not_live: vec!["warrant/m/proof/op/constant-time".into()],
        ct_unproven: vec!["warrant/m/proof/op/constant-time".into()],
        ..AuditEvidence::default()
    })
    .unwrap();
    assert!(row.reason.contains("applicable and unaddressed"), "{}", row.reason);
    assert!(row.evidence.is_empty());
}

#[test]
fn a_failed_constant_time_obligation_is_never_not_applicable() {
    let err = row208(AuditEvidence {
        no_secret_operand_ids: vec![ATTEST.into()],
        ct_unproven: vec!["warrant/m/proof/op/constant-time".into()],
        ..AuditEvidence::default()
    })
    .expect_err("a failed CT obligation with no live proof is an open row");
    assert!(err.contains("CWE-208"), "{err}");
}

// ---------------------------------------------------------------------------
// The attestation is derived from the spec on disk
// ---------------------------------------------------------------------------

#[test]
fn the_attestation_exists_only_for_an_all_public_document() {
    let leb = Evidence::load(&pkg_dir("wasm-leb128")).expect("wasm-leb128 evidence loads");
    assert!(leb.attestation_ids().contains(ATTEST), "every operand of wasm-leb128 is labelled public");
    let gf = Evidence::load(&pkg_dir("gf256")).unwrap();
    assert!(!gf.attestation_ids().contains(ATTEST), "gf256 labels its operands secret");
    let ct = Evidence::load(&pkg_dir("ct-compare")).unwrap();
    assert!(!ct.attestation_ids().contains(ATTEST), "ct-compare labels its operands secret");
}

/// A throw-away copy of a package's evidence inputs, for edits.
struct Copy {
    root: PathBuf,
    pkg: PathBuf,
}

impl Copy {
    fn new(name: &str, spec: &str, tag: &str) -> Copy {
        let real = pkg_dir(name);
        let root = std::env::temp_dir().join(format!("ver04h-nosecret-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("code/packages/rust").join(name);
        fs::create_dir_all(pkg.join("evidence")).unwrap();
        for f in ["test-manifest.json", "proof-manifest.json"] {
            fs::copy(real.join("evidence").join(f), pkg.join("evidence").join(f)).unwrap();
        }
        fs::create_dir_all(pkg.join("src")).unwrap();
        for entry in fs::read_dir(real.join("src")).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), pkg.join("src").join(entry.file_name())).unwrap();
            }
        }
        for f in ["effect_certificate.json", "evidence.bindings", "Cargo.toml", "assurance-profile.json"] {
            fs::copy(real.join(f), pkg.join(f)).unwrap();
        }
        fs::create_dir_all(root.join("code/specs")).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs").join(spec),
            root.join("code/specs").join(spec),
        )
        .unwrap();
        let assumptions = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assumptions");
        fs::create_dir_all(root.join("code/assumptions")).unwrap();
        for f in ["catalogue-v1.json", "evidence-kinds.json", "non-claims.json"] {
            fs::copy(assumptions.join(f), root.join("code/assumptions").join(f)).unwrap();
        }
        Copy { root, pkg }
    }
    fn edit_spec(&self, spec: &str, from: &str, to: &str) {
        let p = self.root.join("code/specs").join(spec);
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains(from), "{spec} must contain {from:?}");
        fs::write(p, s.replacen(from, to, 1)).unwrap();
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn one_secret_label_or_one_unlabelled_operand_withdraws_the_attestation() {
    let cp = Copy::new("wasm-leb128", "wasm-leb128.warrant", "secret");
    cp.edit_spec("wasm-leb128.warrant", "labels decode_signed:\n    data public", "labels decode_signed:\n    data secret");
    let ev = Evidence::load(&cp.pkg).unwrap();
    assert!(!ev.attestation_ids().contains(ATTEST), "a secret label withdraws it");

    let cp = Copy::new("wasm-leb128", "wasm-leb128.warrant", "unlabelled");
    cp.edit_spec("wasm-leb128.warrant", "    max_bits public\n  labels decode_signed_bounded:", "  labels decode_signed_bounded:");
    let ev = Evidence::load(&cp.pkg).unwrap();
    assert!(
        !ev.attestation_ids().contains(ATTEST),
        "an operand with no label row is unclassified, not public"
    );
}

// ---------------------------------------------------------------------------
// The real adopter, end to end
// ---------------------------------------------------------------------------

#[test]
fn wasm_leb128_report_derives_verifies_and_matches_the_checked_in_file() {
    let dir = pkg_dir("wasm-leb128");
    let ev = Evidence::load(&dir).expect("wasm-leb128 manifests load");
    let report = leb_report(&ev);
    verify_evidence(&report, &ev).expect("a derived report must verify");
    assert!(matches!(
        verify_assurance_report(&report, &WeaknessRegistry::embedded()),
        Ok(CheckVerdict::ReportOnly(_))
    ));
    assert_eq!(report.verdict, "report-only");
    assert_eq!(report.claims.len(), 6, "one claim per Warrant operation");

    let r208 = report.weaknesses.iter().find(|w| w.cwe == 208).unwrap();
    assert_eq!(r208.status, WeaknessStatus::Exemption);
    assert!(r208.reason.starts_with("not applicable: no secret operands"), "{}", r208.reason);
    assert_eq!(r208.evidence, vec![ATTEST.to_string()]);
    let na = report.exemptions.iter().find(|x| x.id == NA_ID).expect("not-applicable exemption");
    assert_eq!((na.cwe, na.approved, na.kind.as_str()), (208, false, "not-applicable"));
    assert!(na.assumptions.is_empty(), "an inapplicable class relies on no assumption");
    assert!(report.exemptions.iter().any(|x| x.id == "EXEMPT-CWE-400-NO-STEP-PROOF"));

    // The static rows describe a varint codec, not GF(2^8) or comparisons.
    for cwe in [326, 327, 407] {
        let row = report.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        assert!(row.reason.contains("LEB128"), "CWE-{cwe}: {}", row.reason);
        assert!(!row.reason.contains("GF(2^8)") && !row.reason.contains("comparison"), "CWE-{cwe}");
    }

    let text = fs::read_to_string(dir.join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_package_report(
        "rust/wasm-leb128",
        "0.2.0",
        &on_disk.commit,
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

// ---------------------------------------------------------------------------
// Forgeries the checker must refuse
// ---------------------------------------------------------------------------

/// gf256's report with its CWE-208 row and exemption rewritten into the
/// not-applicable form; `with_attestation` decides whether the row cites it.
fn forge_not_applicable(with_attestation: bool) -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&pkg_dir("gf256")).unwrap();
    let mut report = gf_report(&ev);
    let row = report.weaknesses.iter_mut().find(|w| w.cwe == 208).unwrap();
    row.reason = "not applicable: no secret operands".into();
    row.evidence = if with_attestation { vec![ATTEST.into()] } else { vec![] };
    let x = report.exemptions.iter_mut().find(|x| x.cwe == 208).unwrap();
    x.id = NA_ID.into();
    x.kind = "not-applicable".into();
    x.evidence = row.evidence.clone();
    x.assumptions = vec![];
    (report, ev)
}

#[test]
fn a_package_with_secrets_cannot_cite_the_attestation() {
    let (report, ev) = forge_not_applicable(true);
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::UnknownEvidence(id)) if id == ATTEST),
        "the attestation does not exist for gf256"
    );
}

#[test]
fn a_package_with_secrets_cannot_use_the_not_applicable_exemption() {
    let (report, ev) = forge_not_applicable(false);
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains(NA_ID)),
        "the exemption id is re-derived from the spec and the proof manifest"
    );
}

#[test]
fn the_boolean_logic_profile_is_a_data_file_that_names_itself() {
    // The profile is `assurance-profile.json` beside the package, not an enum variant.
    let p = AuditProfile::load(&pkg_dir("logic-gates")).unwrap();
    assert_eq!(p.name, "boolean-logic");
    assert!(!p.decompresses());
    for cwe in [326, 327, 407] {
        assert!(p.row_reason(cwe).is_some(), "CWE-{cwe}");
    }
}

#[test]
fn the_logic_gates_report_uses_the_boolean_logic_sentences_and_matches_its_manifests() {
    let ev = Evidence::load(&pkg_dir("logic-gates")).unwrap();
    let report = generate_package_report(
        "rust/logic-gates",
        "0.1.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .expect("report derives from the logic-gates manifests");
    for cwe in [326, 327, 407] {
        let row = report.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        // None of the other profiles' subjects may leak into a gate library's report.
        for foreign in ["LEB128", "GF(2^8)", "comparison", "integer encoding"] {
            assert!(!row.reason.contains(foreign), "CWE-{cwe} names {foreign}: {}", row.reason);
        }
    }
    let row = report.weaknesses.iter().find(|w| w.cwe == 407).unwrap();
    assert!(row.reason.contains("at most 16 inputs"), "{}", row.reason);
    // The CWE-208 row is the not-applicable exemption: every operand is public.
    assert!(report.exemptions.iter().any(|x| x.id == NA_ID));
    // And the checked-in report is exactly what the manifests derive.
    let text = fs::read_to_string(pkg_dir("logic-gates").join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_package_report(
        "rust/logic-gates",
        "0.1.0",
        &on_disk.commit,
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

#[test]
fn the_bit_arithmetic_profile_is_a_data_file_that_names_itself() {
    // The profile is `assurance-profile.json` beside the package, not an enum variant.
    let p = AuditProfile::load(&pkg_dir("arithmetic")).unwrap();
    assert_eq!(p.name, "bit-arithmetic");
    assert!(!p.decompresses());
    for cwe in [326, 327, 407] {
        assert!(p.row_reason(cwe).is_some(), "CWE-{cwe}");
    }
}

#[test]
fn the_arithmetic_report_uses_the_bit_arithmetic_sentences_and_matches_its_manifests() {
    let ev = Evidence::load(&pkg_dir("arithmetic")).unwrap();
    let report = generate_package_report(
        "rust/arithmetic",
        "0.1.0",
        "c0ffee",
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .expect("report derives from the arithmetic manifests");
    for cwe in [326, 327, 407] {
        let row = report.weaknesses.iter().find(|w| w.cwe == cwe).unwrap();
        // None of the other profiles' subjects may leak into an adder library's report;
        // in particular the boolean-logic sentences ("single-bit values only", a 2^n
        // decoder bound) would be false for an adder over a 16-bit operand.
        for foreign in [
            "LEB128",
            "GF(2^8)",
            "comparison",
            "integer encoding",
            "single-bit values only",
            "decoder",
            "CMOS",
        ] {
            assert!(!row.reason.contains(foreign), "CWE-{cwe} names {foreign}: {}", row.reason);
        }
    }
    let row = report.weaknesses.iter().find(|w| w.cwe == 407).unwrap();
    assert!(row.reason.contains("linear in the operand length"), "{}", row.reason);
    // The CWE-208 row is the not-applicable exemption: every operand is public.
    assert!(report.exemptions.iter().any(|x| x.id == NA_ID));
    // And the checked-in report is exactly what the manifests derive.
    let text = fs::read_to_string(pkg_dir("arithmetic").join("assurance_report.json")).unwrap();
    let on_disk = AssuranceReport::from_json_str(&text).unwrap();
    verify_evidence(&on_disk, &ev).expect("assurance_report.json must agree with the manifests");
    let fresh = generate_package_report(
        "rust/arithmetic",
        "0.1.0",
        &on_disk.commit,
        &WeaknessRegistry::embedded(),
        &ev,
    )
    .unwrap();
    assert_eq!(fresh.to_json_string(), on_disk.to_json_string(), "regenerate assurance_report.json");
}

#[test]
fn the_varint_codec_profile_is_a_data_file_that_names_itself() {
    // The profile is `assurance-profile.json` beside the package, not an enum variant.
    let p = AuditProfile::load(&pkg_dir("wasm-leb128")).unwrap();
    assert_eq!(p.name, "varint-codec");
    assert!(!p.decompresses());
    for cwe in [326, 327, 407] {
        assert!(p.row_reason(cwe).is_some(), "CWE-{cwe}");
    }
}

/// wasm-leb128's genuine report with the CWE-208 exemption relabelled to the
/// gate-level id while the row keeps citing the (really present) attestation.
/// Every individual fact still holds; what is forged is the *classification*:
/// the attestation is evidence for "not applicable" and for nothing else.
fn leb_attestation_under_another_exemption() -> (AssuranceReport, Evidence) {
    let ev = Evidence::load(&pkg_dir("wasm-leb128")).unwrap();
    let mut report = leb_report(&ev);
    let x = report.exemptions.iter_mut().find(|x| x.id == NA_ID).unwrap();
    x.id = "EXEMPT-CWE-208-GATE-LEVEL".into();
    (report, ev)
}

#[test]
fn the_attestation_cannot_be_cited_under_a_different_exemption_id() {
    let (report, ev) = leb_attestation_under_another_exemption();
    assert!(
        matches!(verify_evidence(&report, &ev), Err(E::EvidenceMismatch(m)) if m.contains(ATTEST)),
        "a CWE-208 row citing {ATTEST} must carry the not-applicable exemption"
    );
}
