//! # coding_adventures_ca_capability_analyzer
//!
//! Static capability analyzer and deny-by-default name allowlist for Rust packages (VER08 / VER08a / VER-P027a).
//!
//! Implements Layers 3 and 4 of the repository capability security system (Spec 13, D21, VER00 §8):
//! - Reads and validates `required_capabilities.json`;
//! - Defaults to pure computation (`capabilities = []`) if manifest is absent;
//! - Traverses Rust source files while enforcing resource quotas and refusing symlinks;
//! - Detects ambient operating-system calls and compares against declared capabilities (`CAP001`);
//! - Detects banned constructs (`unsafe`, inline assembly, foreign linkage) and verifies exceptions (`CAP002`).

#![deny(unsafe_code)]

pub mod manifest;
pub mod parser;
pub mod report;
pub mod rules;
pub mod scanner;

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

pub use manifest::{load_manifest, ManifestData, ManifestError};
pub use parser::{scan_source, ScanResult, SourceBannedConstruct, SourceDetection};
pub use report::{AnalysisReport, Violation};
pub use scanner::{discover_sources, ScanError};

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
pub fn analyze_package(
    package_dir: &Path,
    _verbose: bool,
) -> Result<AnalysisReport, AnalyzerError> {
    // 1. Load and validate manifest
    let manifest = load_manifest(package_dir)?;

    // 2. Discover and read all Rust source files
    let sources = discover_sources(package_dir)?;

    let mut violations = Vec::new();
    let mut detected_capabilities = BTreeSet::new();
    let mut total_bytes = 0;

    // 3. Scan each source file
    for source in &sources {
        total_bytes += source.size_bytes;
        let scan_result = scan_source(&source.rel_path, &source.content)
            .map_err(|e| AnalyzerError::Parse(format!("{}: {e}", source.rel_path)))?;

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

        // B. Banned construct checks (CAP002)
        for banned in scan_result.banned {
            if !manifest.is_construct_exempt(banned.construct) {
                violations.push(Violation::BannedConstruct {
                    construct: banned.construct.to_string(),
                    file: banned.file,
                    line: banned.line,
                    col: banned.col,
                    evidence: banned.evidence,
                });
            }
        }
    }

    // Deterministic sort of violations by file, line, col
    violations.sort();

    Ok(AnalysisReport {
        package: manifest.package,
        manifest_present: manifest.is_explicit,
        violations,
        detected_capabilities,
        scanned_files_count: sources.len(),
        total_bytes_scanned: total_bytes,
    })
}
