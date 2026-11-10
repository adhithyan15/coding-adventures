//! # Analysis Report & Diagnostics Formatter (VER08a)
//!
//! Structures violations and formats diagnostics for developers and CI gates.

use std::collections::BTreeSet;

/// Specific violation types defined in Spec 13 / VER08a.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    /// CAP001: Undeclared capability access.
    UndeclaredCapability {
        capability: String,
        file: String,
        line: usize,
        col: usize,
        evidence: String,
    },
    /// CAP002: Banned language construct without valid exception.
    BannedConstruct {
        construct: String,
        file: String,
        line: usize,
        col: usize,
        evidence: String,
    },
}

impl Violation {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndeclaredCapability { .. } => "CAP001",
            Self::BannedConstruct { .. } => "CAP002",
        }
    }

    pub fn file(&self) -> &str {
        match self {
            Self::UndeclaredCapability { file, .. } | Self::BannedConstruct { file, .. } => file,
        }
    }

    pub fn line(&self) -> usize {
        match self {
            Self::UndeclaredCapability { line, .. } | Self::BannedConstruct { line, .. } => *line,
        }
    }

    pub fn col(&self) -> usize {
        match self {
            Self::UndeclaredCapability { col, .. } | Self::BannedConstruct { col, .. } => *col,
        }
    }

    pub fn format_line(&self) -> String {
        match self {
            Self::UndeclaredCapability {
                capability,
                file,
                line,
                col,
                evidence,
            } => {
                format!(
                    "CAP001: Undeclared capability '{capability}' detected at {file}:{line}:{col} ('{evidence}')"
                )
            }
            Self::BannedConstruct {
                construct,
                file,
                line,
                col,
                evidence,
            } => {
                format!(
                    "CAP002: Banned construct '{construct}' detected at {file}:{line}:{col} ('{evidence}')"
                )
            }
        }
    }
}

/// Comprehensive report produced by the capability analyzer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisReport {
    pub package: String,
    pub manifest_present: bool,
    pub violations: Vec<Violation>,
    pub detected_capabilities: BTreeSet<String>,
    pub scanned_files_count: usize,
    pub total_bytes_scanned: u64,
}

impl AnalysisReport {
    /// True if there are zero capability or banned construct violations.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    /// Stable exit code: 0 for clean, 1 for violations.
    pub fn exit_code(&self) -> i32 {
        if self.is_clean() {
            0
        } else {
            1
        }
    }

    /// Render human-readable diagnostic text.
    pub fn format_text(&self, verbose: bool) -> String {
        let mut out = String::new();

        if self.is_clean() {
            out.push_str(&format!(
                "PASS: package '{}' capability check succeeded ({} files scanned, {} bytes)\n",
                self.package, self.scanned_files_count, self.total_bytes_scanned
            ));
            if verbose {
                if self.detected_capabilities.is_empty() {
                    out.push_str("  No ambient OS capabilities detected (pure computation).\n");
                } else {
                    out.push_str("  Declared capabilities exercised:\n");
                    for cap in &self.detected_capabilities {
                        out.push_str(&format!("    - {cap}\n"));
                    }
                }
            }
        } else {
            out.push_str(&format!(
                "FAIL: package '{}' has {} capability violation(s):\n",
                self.package,
                self.violations.len()
            ));
            for v in &self.violations {
                out.push_str(&format!("  {}\n", v.format_line()));
            }
        }

        out
    }
}
