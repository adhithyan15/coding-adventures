//! # Coding Adventures Assurance Report Gate (VER01 / VER15 / VER-P028)
//!
//! Provides the machine-readable assurance report generator, weakness registry evaluator,
//! and standalone report checker with zero third-party dependencies.
//!
//! Every assumption a report states is a catalogue id of VER13
//! (`code/assumptions/catalogue-v1.json`), pinned by digest in the report and
//! checked by rules AC-1..AC-14 ([`assumptions`], [`catalogue`]).

#![deny(unsafe_code)]

pub mod assumptions;
pub mod catalogue;
pub mod checker;
pub mod evidence;
pub mod json;
pub mod lane_evidence;
pub mod matrix;
pub mod registry;
pub mod report;

pub use assumptions::{check_assumptions, AssumptionCheckError, AssumptionSummary};
pub use catalogue::{CatalogueError, CatalogueSet};
pub use checker::{verify_assurance_report, CheckError, CheckVerdict};
pub use evidence::{
    derive_claims, verify_evidence, verify_profile, Evidence, EvidenceCheckError, EvidenceError,
};
pub use matrix::{
    build_weakness_matrix, AuditEvidence, AuditProfile, MatrixError, PackageAuditContext,
    WeaknessRow, WeaknessStatus,
};
pub use registry::{RegistryError, WeaknessClass, WeaknessRegistry};
pub use report::{
    AssuranceReport, CapabilitiesSummary, CatalogueRef, ClaimEvidence, ComputedExemption,
    DischargedAssumption, ExhaustiveEvidence, FuzzEvidence, ProofEvidence, PropertyEvidence,
    ReportError, RungsSummary,
};

/// Construct the report-only assurance report for `rust/ct-compare` from
/// evidence manifests.  Every count, label, certificate digest and context flag
/// is derived from `evidence`; this function holds no evidence literal.
pub fn generate_ct_compare_report(
    commit_sha: &str,
    registry: &WeaknessRegistry,
    evidence: &Evidence,
) -> Result<AssuranceReport, MatrixError> {
    generate_package_report(
        "rust/ct-compare",
        "0.1.0",
        commit_sha,
        registry,
        evidence,
        AuditProfile::Comparison,
    )
}

/// Construct the report-only assurance report of any package whose evidence
/// manifests live in `evidence`.  `package` (for example `rust/gf256`) and
/// `version` come from the caller, which reads them from the package; every
/// count, label, certificate digest and context flag is derived from
/// `evidence`, so the function holds no evidence literal.  Obligations the
/// proof manifest records `not-live` become claims without a proof block and
/// unapproved exemptions that quote the recorded blocker.
///
/// Assumptions come from the VER13 catalogue loaded with `evidence`: each
/// claim lists every entry that applies to it (rule AC-4), the report lists
/// the entries about its weakness matrix, capability evidence and verdict, and
/// each exemption lists the ids it derives from (rule AC-13).
pub fn generate_package_report(
    package: &str,
    version: &str,
    commit_sha: &str,
    registry: &WeaknessRegistry,
    evidence: &Evidence,
    profile: AuditProfile,
) -> Result<AssuranceReport, MatrixError> {
    if package != evidence.package_id {
        return Err(MatrixError::MissingEvidence(format!(
            "asked for a report on {package} but the evidence is for {}",
            evidence.package_id
        )));
    }
    let ctx = evidence.audit_context_with_profile(package, profile);
    let weaknesses = build_weakness_matrix(registry, &ctx)?;
    let claims = derive_claims(evidence).map_err(|e| MatrixError::MissingEvidence(e.0))?;
    let rungs = evidence.rungs();

    // One exemption per unapproved-exemption row, copied from the row so the
    // two lists cannot disagree.
    let exemptions: Vec<ComputedExemption> = weaknesses
        .iter()
        .filter(|w| w.status == WeaknessStatus::Exemption)
        .map(|w| {
            let not_applicable = w.evidence.iter().any(|e| e == evidence::NO_SECRET_OPERANDS_ATTESTATION);
            let (id, kind) = if w.cwe == 208 && not_applicable {
                (evidence::NOT_APPLICABLE_208_EXEMPTION.to_string(), "not-applicable")
            } else if w.cwe == 208 {
                ("EXEMPT-CWE-208-GATE-LEVEL".to_string(), "weakness-floor")
            } else if w.cwe == 400 {
                ("EXEMPT-CWE-400-NO-STEP-PROOF".to_string(), "no-live-proof")
            } else {
                (format!("EXEMPT-CWE-{}-UNSAFE-EXCEPTION", w.cwe), "unsafe-exception")
            };
            ComputedExemption {
                id,
                kind: kind.into(),
                cwe: w.cwe,
                approved: w.approved.unwrap_or(false),
                rationale: w.reason.clone(),
                evidence: w.evidence.clone(),
                assumptions: Vec::new(),
            }
        })
        .collect();
    // Derived, never typed in: the section 3.9 table applied to what the
    // claims still assume.
    let set = &evidence.catalogue;
    let mut exemptions = exemptions;
    for x in &mut exemptions {
        x.assumptions = assumptions::derive_exemption_assumptions(set, &x.id, &claims)
            .map_err(MatrixError::MissingEvidence)?;
    }
    let unverified: std::collections::BTreeSet<String> = weaknesses
        .iter()
        .flat_map(|w| w.evidence.iter())
        .chain(claims.iter().flat_map(|c| c.evidence.iter()))
        .chain(exemptions.iter().flat_map(|x| x.evidence.iter()))
        .filter(|id| id.starts_with(evidence::UNVERIFIED_PREFIX))
        .cloned()
        .collect();
    let report_assumptions = assumptions::derive_report_assumptions(set, package, &unverified)
        .map_err(MatrixError::MissingEvidence)?;

    Ok(AssuranceReport {
        package: package.into(),
        version: version.into(),
        commit: commit_sha.into(),
        profile,
        catalogue: assumptions::catalogue_ref(set),
        assumptions: report_assumptions,
        closure: vec![package.into()],
        capabilities: CapabilitiesSummary {
            declared: evidence.cert.declared_effects.clone(),
            analyzer: if evidence.cert.label == "proved" { "pass" } else { "fail" }.into(),
            certificate: evidence.cert.label.clone(),
            new_since_last_release: vec![],
        },
        weaknesses,
        claims,
        rungs,
        public_symbols_without_claims: vec![],
        exemptions,
        verdict: "report-only".into(),
    })
}
