//! # Coding Adventures Assurance Report Gate (VER01 / VER15 / VER-P028)
//!
//! Provides the machine-readable assurance report generator, weakness registry evaluator,
//! and standalone report checker with zero third-party dependencies.

#![deny(unsafe_code)]

pub mod checker;
pub mod evidence;
pub mod json;
pub mod matrix;
pub mod registry;
pub mod report;

pub use checker::{verify_assurance_report, CheckError, CheckVerdict};
pub use evidence::{
    derive_claims, verify_evidence, Evidence, EvidenceCheckError, EvidenceError,
};
pub use matrix::{
    build_weakness_matrix, AuditEvidence, AuditProfile, MatrixError, PackageAuditContext,
    WeaknessRow, WeaknessStatus,
};
pub use registry::{RegistryError, WeaknessClass, WeaknessRegistry};
pub use report::{
    AssuranceReport, CapabilitiesSummary, ClaimEvidence, ComputedExemption, ExhaustiveEvidence,
    FuzzEvidence,
    ProofEvidence, PropertyEvidence, ReportError, RungsSummary,
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
pub fn generate_package_report(
    package: &str,
    version: &str,
    commit_sha: &str,
    registry: &WeaknessRegistry,
    evidence: &Evidence,
    profile: AuditProfile,
) -> Result<AssuranceReport, MatrixError> {
    let ctx = evidence.audit_context_with_profile(package, profile);
    let weaknesses = build_weakness_matrix(registry, &ctx)?;
    let claims = derive_claims(evidence).map_err(|e| MatrixError::MissingEvidence(e.0))?;
    let rungs = evidence.rungs();

    // One exemption per unapproved-exemption row, copied from the row so the
    // two lists cannot disagree.
    let exemptions = weaknesses
        .iter()
        .filter(|w| w.status == WeaknessStatus::Exemption)
        .map(|w| {
            let (id, kind) = if w.cwe == 208 {
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
            }
        })
        .collect();

    Ok(AssuranceReport {
        package: package.into(),
        version: version.into(),
        commit: commit_sha.into(),
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
