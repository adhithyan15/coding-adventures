//! # Assurance Report Checker & Gate Validator (VER01 / VER00 §9)
//!
//! Fail-closed independent verification of `assurance_report.json`.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::evidence::label_rank;
use crate::matrix::WeaknessStatus;
use crate::registry::WeaknessRegistry;
use crate::report::AssuranceReport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckVerdict {
    Pass,
    ReportOnly(String),
}

#[derive(Debug)]
pub enum CheckError {
    InvalidPackage(String),
    IncompleteMatrix(String),
    InvalidExclusion(String),
    OpenRow(String),
    IllegalPassVerdict(String),
    InvalidRungStatus(String),
    FailingClaim(String),
    MissingEvidence(String),
    MismatchedExemption(String),
    CapabilitiesViolation(String),
    UncoveredSymbols(Vec<String>),
    CorruptReport(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::InvalidPackage(e) => write!(f, "Invalid package identifier: {e}"),
            CheckError::IncompleteMatrix(e) => write!(f, "Incomplete weakness matrix: {e}"),
            CheckError::InvalidExclusion(e) => write!(f, "Invalid weakness exclusion: {e}"),
            CheckError::OpenRow(e) => write!(f, "Open weakness row: {e}"),
            CheckError::IllegalPassVerdict(e) => write!(f, "Illegal pass verdict: {e}"),
            CheckError::InvalidRungStatus(e) => write!(f, "Invalid rung status: {e}"),
            CheckError::FailingClaim(e) => write!(f, "Claim failure: {e}"),
            CheckError::MissingEvidence(e) => write!(f, "Missing evidence for discharged row: {e}"),
            CheckError::MismatchedExemption(e) => write!(f, "Mismatched exemption state: {e}"),
            CheckError::CapabilitiesViolation(e) => write!(f, "Capabilities summary violation: {e}"),
            CheckError::UncoveredSymbols(syms) => {
                write!(f, "Public exported symbols without claims: {:?}", syms)
            }
            CheckError::CorruptReport(e) => write!(f, "Corrupt report: {e}"),
        }
    }
}

impl std::error::Error for CheckError {}

/// The registry's floor label for CWE-208 (gate-level netlist proof).
const GATE_LEVEL_LABEL: &str = "proved-to-gate-level";

/// A report may carry the labels the evidence layer ranks (`proved`,
/// `bounded(K)`) and the registry's gate-level floor label.  `unknown`,
/// `falsified`, `unchecked` and anything unparseable have no rank.
fn report_label_ranked(label: &str) -> bool {
    label_rank(label).is_some() || label == GATE_LEVEL_LABEL
}

/// Verify an assurance report fail-closed against the canonical weakness registry.
pub fn verify_assurance_report(
    report: &AssuranceReport,
    registry: &WeaknessRegistry,
) -> Result<CheckVerdict, CheckError> {
    // 1. Package format
    if !report.package.starts_with("rust/") {
        return Err(CheckError::InvalidPackage(format!(
            "'{}' must start with 'rust/'",
            report.package
        )));
    }

    // 2. Matrix completeness: all registry classes accounted for
    let mut seen_cwes = HashSet::new();
    let mut exemption_map = HashMap::new();

    for row in &report.weaknesses {
        if !seen_cwes.insert(row.cwe) {
            return Err(CheckError::IncompleteMatrix(format!(
                "duplicate CWE-{} in report weakness matrix",
                row.cwe
            )));
        }

        let reg_class = registry.find_class(row.cwe).ok_or_else(|| {
            CheckError::IncompleteMatrix(format!(
                "CWE-{} in report is not recognized in registry",
                row.cwe
            ))
        })?;

        // 3. Excluded rows validity
        if row.status == WeaknessStatus::Excluded {
            if reg_class.safe_subsets.is_empty() {
                return Err(CheckError::InvalidExclusion(format!(
                    "CWE-{} ({}) has no safe subset and cannot be excluded by construction",
                    row.cwe, row.name
                )));
            }
            if row.reason.trim().is_empty() {
                return Err(CheckError::InvalidExclusion(format!(
                    "CWE-{} marked excluded without exclusion rationale",
                    row.cwe
                )));
            }
        }

        // 4. Discharged rows validity
        if row.status == WeaknessStatus::Discharged {
            if row.evidence.is_empty() || row.reason.trim().is_empty() {
                return Err(CheckError::MissingEvidence(format!(
                    "CWE-{} marked discharged without supporting evidence or reason",
                    row.cwe
                )));
            }

            // Check floor enforcement for CWE-208: requires proved-to-gate-level
            if row.cwe == 208 {
                let has_gate_level_proof = report.claims.iter().any(|c| {
                    c.proof
                        .as_ref()
                        .map(|p| p.label == "proved-to-gate-level")
                        .unwrap_or(false)
                });
                if !has_gate_level_proof {
                    return Err(CheckError::MissingEvidence(format!(
                        "CWE-208 requires minimum proof label '{}' to be discharged",
                        reg_class.minimum_proof_label
                    )));
                }
            }
        }

        // 5. Exemption recording
        if row.status == WeaknessStatus::Exemption {
            let approved = row.approved.unwrap_or(false);
            if !approved && report.verdict == "pass" {
                return Err(CheckError::IllegalPassVerdict(format!(
                    "report claims verdict 'pass' while CWE-{} is an unapproved exemption",
                    row.cwe
                )));
            }
            exemption_map.insert(row.cwe, approved);
        }
    }

    for class in &registry.classes {
        if !seen_cwes.contains(&class.cwe) {
            return Err(CheckError::IncompleteMatrix(format!(
                "registry CWE-{} ({}) is missing from report weakness matrix",
                class.cwe, class.name
            )));
        }
    }

    // 6. Cross-validate exemptions between matrix and exemptions list
    for ex in &report.exemptions {
        match exemption_map.get(&ex.cwe) {
            Some(&row_approved) => {
                if row_approved != ex.approved {
                    return Err(CheckError::MismatchedExemption(format!(
                        "CWE-{} approval mismatch: matrix says {}, exemptions list says {}",
                        ex.cwe, row_approved, ex.approved
                    )));
                }
            }
            None => {
                return Err(CheckError::MismatchedExemption(format!(
                    "exemption for CWE-{} exists in exemptions list but not marked as Exemption in weakness matrix",
                    ex.cwe
                )));
            }
        }
    }

    for &cwe in exemption_map.keys() {
        if !report.exemptions.iter().any(|ex| ex.cwe == cwe) {
            return Err(CheckError::MismatchedExemption(format!(
                "CWE-{} marked as Exemption in matrix but missing from exemptions list",
                cwe
            )));
        }
    }

    // 7. Symbol claim coverage
    if !report.public_symbols_without_claims.is_empty() {
        return Err(CheckError::UncoveredSymbols(
            report.public_symbols_without_claims.clone(),
        ));
    }

    // 8. Claim evidence integrity
    for claim in &report.claims {
        if claim.example != "pass" {
            return Err(CheckError::FailingClaim(format!(
                "claim '{}' example status is '{}', expected 'pass'",
                claim.id, claim.example
            )));
        }
        if claim.property.status != "pass" {
            return Err(CheckError::FailingClaim(format!(
                "claim '{}' property status is '{}', expected 'pass'",
                claim.id, claim.property.status
            )));
        }
        if let Some(fuzz) = &claim.fuzz {
            if fuzz.status != "pass" {
                return Err(CheckError::FailingClaim(format!(
                    "claim '{}' fuzz status is '{}', expected 'pass'",
                    claim.id, fuzz.status
                )));
            }
        }
        if let Some(ex) = &claim.exhaustive {
            if ex.status != "pass" {
                return Err(CheckError::FailingClaim(format!(
                    "claim '{}' exhaustive status is '{}', expected 'pass'",
                    claim.id, ex.status
                )));
            }
        }
        if let Some(proof) = &claim.proof {
            if !report_label_ranked(&proof.label) {
                return Err(CheckError::FailingClaim(format!(
                    "claim '{}' proof label '{}' has no rank (expected 'proved', 'bounded(K)' or '{GATE_LEVEL_LABEL}')",
                    claim.id, proof.label
                )));
            }
            // A `pass` verdict may not rest on obligations the prover did not
            // discharge; under `report-only` they stay visible as assumptions.
            // Catalogue ids spell them `wir.bound.*-unchecked`; the legacy
            // `<kind>:unchecked` spelling is refused too.
            if report.verdict == "pass" {
                if let Some(a) = proof
                    .assumptions
                    .iter()
                    .find(|a| a.ends_with(":unchecked") || a.ends_with("-unchecked"))
                {
                    return Err(CheckError::FailingClaim(format!(
                        "claim '{}' has the unchecked assumption '{a}', which a 'pass' verdict cannot carry",
                        claim.id
                    )));
                }
            }
        }
    }

    // 9. Rungs assessment
    let rungs = [
        ("E0", report.rungs.e0.as_str()),
        ("E1", report.rungs.e1.as_str()),
        ("E2", report.rungs.e2.as_str()),
        ("E3", report.rungs.e3.as_str()),
        ("E4", report.rungs.e4.as_str()),
        ("E5", report.rungs.e5.as_str()),
    ];

    for (name, status) in &rungs {
        if *status != "pass" && *status != "not-live" {
            return Err(CheckError::InvalidRungStatus(format!(
                "rung '{name}' has invalid status '{status}', must be 'pass' or 'not-live'"
            )));
        }
    }

    let non_live_rungs: Vec<&str> = rungs
        .iter()
        .filter_map(|(name, status)| if *status == "not-live" { Some(*name) } else { None })
        .collect();

    if !non_live_rungs.is_empty() && report.verdict == "pass" {
        return Err(CheckError::IllegalPassVerdict(format!(
            "report claims verdict 'pass' but rungs {:?} are 'not-live'",
            non_live_rungs
        )));
    }

    // 10. Unapproved exemptions check
    let unapproved_exemptions: Vec<&str> = report
        .exemptions
        .iter()
        .filter(|ex| !ex.approved)
        .map(|ex| ex.id.as_str())
        .collect();

    if !unapproved_exemptions.is_empty() && report.verdict == "pass" {
        return Err(CheckError::IllegalPassVerdict(format!(
            "report claims verdict 'pass' but contains unapproved exemptions: {:?}",
            unapproved_exemptions
        )));
    }

    // 11. Capabilities summary check when verdict is pass
    if report.verdict == "pass" {
        if report.capabilities.analyzer != "pass" {
            return Err(CheckError::CapabilitiesViolation(format!(
                "capabilities analyzer status is '{}', expected 'pass'",
                report.capabilities.analyzer
            )));
        }
        if report.capabilities.certificate != "proved" {
            return Err(CheckError::CapabilitiesViolation(format!(
                "capabilities certificate status is '{}', expected 'proved'",
                report.capabilities.certificate
            )));
        }
    }

    // 12. Final verdict determination
    if !non_live_rungs.is_empty() || !unapproved_exemptions.is_empty() {
        if report.verdict != "report-only" {
            return Err(CheckError::IllegalPassVerdict(format!(
                "expected verdict 'report-only' due to non-live rungs {:?} and unapproved exemptions {:?}, found '{}'",
                non_live_rungs, unapproved_exemptions, report.verdict
            )));
        }
        Ok(CheckVerdict::ReportOnly(format!(
            "report-only: rungs {:?} not-live; exemptions {:?} unapproved",
            non_live_rungs, unapproved_exemptions
        )))
    } else if report.verdict == "pass" {
        Ok(CheckVerdict::Pass)
    } else if report.verdict == "report-only" {
        Ok(CheckVerdict::ReportOnly("enforced report-only mode".into()))
    } else {
        Err(CheckError::CorruptReport(format!(
            "unknown verdict '{}'",
            report.verdict
        )))
    }
}
