//! # coding_adventures_ca_capability_analyzer
//!
//! Static capability analyzer, deny-by-default name allowlist, generated capability
//! modules (VER23) and effect proof certificates (VER24) for Rust packages.
//!
//! Implements Layers 3 and 4 of the repository capability security system (Spec 13, D21, VER00 §8):
//! - Reads and validates `required_capabilities.json`;
//! - Defaults to pure computation (`capabilities = []`) if manifest is absent;
//! - Traverses Rust source files while enforcing resource quotas and refusing symlinks;
//! - Detects ambient operating-system calls and compares against declared capabilities (`CAP001`);
//! - Detects banned constructs (`unsafe`, inline assembly, foreign linkage) and verifies exceptions (`CAP002`);
//! - Generates sealed `capabilities.rs` modules exposing only declared capabilities (VER23);
//! - Emits and independently verifies canonical effect proof certificates with small checker (VER24).

#![deny(unsafe_code)]

pub mod cargo;
pub mod certificate;
pub mod generator;
pub mod manifest;
pub mod parser;
pub mod report;
pub mod rules;
pub mod scanner;
pub mod sha256;

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

pub use certificate::{
    generate_certificate, parse_certificate, verify_certificate, CertVerificationResult,
    EffectCertificate, EntryPointCert, CERT_SCHEMA_VERSION,
};
pub use generator::{
    capabilities_module_path, check_module_on_disk, generate_module_source, write_module_to_disk,
};
pub use manifest::{load_manifest, Capability, ManifestData, ManifestError};
pub use cargo::{analyze_cargo_toml, CargoFacts};
pub use parser::{
    scan_source, scan_sources, scan_sources_with_edition, Edition, ScanResult, SourceBannedConstruct, SourceDetection,
    SourceUnanalyzable,
};
pub use report::{AnalysisReport, ConstructStatus, ExceptedConstruct, Violation};
pub use scanner::{discover_sources, ScanError};
pub use sha256::{sha256, sha256_hex, Sha256};

/// Overall error returned by the analyzer CLI / library.
#[derive(Debug)]
pub enum AnalyzerError {
    Manifest(ManifestError),
    Scan(ScanError),
    Parse(String),
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(e) => write!(f, "{e}"),
            Self::Scan(e) => write!(f, "{e}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for AnalyzerError {}

impl From<ManifestError> for AnalyzerError {
    fn from(e: ManifestError) -> Self {
        Self::Manifest(e)
    }
}

impl From<ScanError> for AnalyzerError {
    fn from(e: ScanError) -> Self {
        Self::Scan(e)
    }
}

/// Analyze a Rust package located in `package_dir`.
///
/// Pipeline (VER08a): manifest -> source discovery -> package-wide lexical scan
/// -> `Cargo.toml` facts -> compare against the manifest.  Every `.rs` file the
/// scanner returns is scanned, including the generated `src/capabilities.rs`
/// (it used to be skipped, which let an edited generated file hide anything).
pub fn analyze_package(
    package_dir: &Path,
    _verbose: bool,
) -> Result<AnalysisReport, AnalyzerError> {
    // 1. Load and validate manifest
    let manifest = load_manifest(package_dir)?;

    // 2. Discover and read all Rust source files
    let sources = discover_sources(package_dir)?;

    // 3. Scan all files as one package so `use` bindings are shared across files
    let files: Vec<(&str, &str)> = sources
        .iter()
        .map(|s| (s.rel_path.as_str(), s.content.as_str()))
        .collect();
    let total_bytes: u64 = sources.iter().map(|s| s.size_bytes).sum();

    // 4. Cargo.toml facts (proc-macro, escaping paths, dependencies, edition).
    //    Read before the scan: the edition decides how `cr#"..."#` is lexed.
    let cargo = analyze_cargo_toml(package_dir).map_err(AnalyzerError::Parse)?;
    let scan_result = scan_sources_with_edition(&files, cargo.edition).map_err(AnalyzerError::Parse)?;

    let mut violations = Vec::new();
    let mut excepted_constructs = Vec::new();
    let mut detected_capabilities = BTreeSet::new();

    // A. Capability checks (CAP001)
    for det in scan_result.detections {
        let cap_str = format!("{}:{}:*", det.category, det.action);
        detected_capabilities.insert(cap_str.clone());

        if !manifest.covers_capability(det.category, det.action) {
            violations.push(Violation::UndeclaredCapability {
                capability: cap_str,
                file: det.file,
                line: det.line,
                col: det.col,
                evidence: det.evidence,
            });
        }
    }

    // B. Banned construct checks (CAP002); waived constructs stay visible.
    for banned in scan_result.banned {
        if manifest.is_construct_exempt(banned.construct) {
            excepted_constructs.push(ExceptedConstruct {
                construct: banned.construct.to_string(),
                file: banned.file,
                line: banned.line,
                col: banned.col,
                evidence: banned.evidence,
            });
        } else {
            violations.push(Violation::BannedConstruct {
                construct: banned.construct.to_string(),
                file: banned.file,
                line: banned.line,
                col: banned.col,
                evidence: banned.evidence,
            });
        }
    }

    // C. Unanalysable constructs (CAP003): never waivable.
    for u in scan_result.unanalyzable {
        violations.push(Violation::Unanalyzable {
            reason: u.reason,
            file: u.file,
            line: u.line,
            col: u.col,
            evidence: u.evidence,
        });
    }
    for f in cargo.findings {
        violations.push(Violation::Unanalyzable {
            reason: f.reason,
            file: "Cargo.toml".to_string(),
            line: f.line,
            col: 0,
            evidence: f.evidence,
        });
    }

    // Deterministic sort by file, line, col
    violations.sort();
    excepted_constructs.sort();

    Ok(AnalysisReport {
        package: manifest.package,
        manifest_present: manifest.is_explicit,
        violations,
        detected_capabilities,
        scanned_files_count: sources.len(),
        total_bytes_scanned: total_bytes,
        excepted_constructs,
        unanalyzed_dependencies: cargo.dependencies,
    })
}
