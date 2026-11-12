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

use std::path::{Path, PathBuf};

use coding_adventures_ca_assurance_report::{
    build_weakness_matrix, generate_package_report, verify_evidence, AssuranceReport,
    AuditEvidence, AuditProfile, Evidence, EvidenceCheckError as E,
    PackageAuditContext, WeaknessRegistry, WeaknessStatus,
};

const ATTEST: &str = "warrant-spec::all-operands-public";
const NA_ID: &str = "EXEMPT-CWE-208-NO-SECRET-OPERANDS";

fn pkg_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn gf_report(ev: &Evidence) -> AssuranceReport {
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
fn the_varint_profile_parses_and_unknown_profiles_list_it() {
    assert_eq!("varint-codec".parse::<AuditProfile>(), Ok(AuditProfile::VarintCodec));
    let err = "lzw".parse::<AuditProfile>().unwrap_err();
    assert!(err.contains("varint-codec"), "{err}");
}
