//! # Analysis Report & Diagnostics Formatter (VER08a)
//!
//! Structures violations and formats diagnostics for developers and CI gates.

use std::collections::BTreeSet;

/// Specific violation types defined in Spec 13 / VER08a.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// CAP003: a construct the analyzer can see but cannot analyse (never waivable).
    Unanalyzable {
        reason: String,
        file: String,
        line: usize,
        col: usize,
        evidence: String,
    },
}

/// Violations sort by location first (file, line, column), then code, then text.
///
/// The derived `Ord` ordered by enum variant first, so CAP002 findings were
/// listed before CAP001 ones regardless of where they were; the documented
/// intent ("sort by file, line, col") was not met.
impl Ord for Violation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let key = |v: &Violation| (v.file().to_string(), v.line(), v.col(), v.code(), v.format_line());
        key(self).cmp(&key(other))
    }
}

impl PartialOrd for Violation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Violation {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndeclaredCapability { .. } => "CAP001",
            Self::BannedConstruct { .. } => "CAP002",
            Self::Unanalyzable { .. } => "CAP003",
        }
    }

    pub fn file(&self) -> &str {
        match self {
            Self::UndeclaredCapability { file, .. }
            | Self::BannedConstruct { file, .. }
            | Self::Unanalyzable { file, .. } => file,
        }
    }

    pub fn line(&self) -> usize {
        match self {
            Self::UndeclaredCapability { line, .. }
            | Self::BannedConstruct { line, .. }
            | Self::Unanalyzable { line, .. } => *line,
        }
    }

    pub fn col(&self) -> usize {
        match self {
            Self::UndeclaredCapability { col, .. }
            | Self::BannedConstruct { col, .. }
            | Self::Unanalyzable { col, .. } => *col,
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
            Self::Unanalyzable {
                reason,
                file,
                line,
                col,
                evidence,
            } => {
                format!(
                    "CAP003: Unanalysable construct detected at {file}:{line}:{col} ('{evidence}'): {reason}"
                )
            }
        }
    }
}

/// A banned construct that a manifest exception waived.  It is *present*, not absent.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExceptedConstruct {
    pub construct: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub evidence: String,
}

/// Presence of one banned construct in a package (VER08a §6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructStatus {
    /// No occurrence anywhere in the scanned sources.
    Absent,
    /// Occurrences exist and every one is waived by a manifest exception.
    PresentExcepted(usize),
    /// Occurrences exist that are not waived (the report is not clean).
    PresentViolation(usize),
}

impl ConstructStatus {
    /// `absent`, `present-excepted(N)` or `present-violation(N)`.
    pub fn render(&self) -> String {
        match self {
            Self::Absent => "absent".to_string(),
            Self::PresentExcepted(n) => format!("present-excepted({n})"),
            Self::PresentViolation(n) => format!("present-violation({n})"),
        }
    }
}

/// The three banned construct ids, in the order printed by the SUMMARY line.
pub const BANNED_CONSTRUCTS: [&str; 3] = ["unsafe_code", "inline_assembly", "foreign_linkage"];

/// Comprehensive report produced by the capability analyzer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisReport {
    pub package: String,
    pub manifest_present: bool,
    pub violations: Vec<Violation>,
    pub detected_capabilities: BTreeSet<String>,
    pub scanned_files_count: usize,
    pub total_bytes_scanned: u64,
    /// Banned constructs that exist but were waived by an exception (never hidden).
    pub excepted_constructs: Vec<ExceptedConstruct>,
    /// Third-party dependency names from `Cargo.toml` that this analyzer did not look inside.
    pub unanalyzed_dependencies: Vec<String>,
}

impl AnalysisReport {
    /// True if there are zero capability, banned construct or unanalysable violations.
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

    /// Presence of a banned construct, distinguishing waived from absent.
    pub fn construct_status(&self, construct: &str) -> ConstructStatus {
        let excepted = self
            .excepted_constructs
            .iter()
            .filter(|e| e.construct == construct)
            .count();
        let violating = self
            .violations
            .iter()
            .filter(|v| matches!(v, Violation::BannedConstruct { construct: c, .. } if c == construct))
            .count();
        if violating > 0 {
            ConstructStatus::PresentViolation(violating + excepted)
        } else if excepted > 0 {
            ConstructStatus::PresentExcepted(excepted)
        } else {
            ConstructStatus::Absent
        }
    }

    /// Number of detected `ffi:*` capabilities (declared or not).
    pub fn ffi_present(&self) -> bool {
        self.detected_capabilities.iter().any(|c| c.starts_with("ffi:"))
    }

    /// The machine-readable last line of every report (VER08a §6.1).
    pub fn summary_line(&self) -> String {
        let mut out = String::from("SUMMARY");
        for c in BANNED_CONSTRUCTS {
            out.push_str(&format!(" {c}={}", self.construct_status(c).render()));
        }
        out.push_str(if self.ffi_present() {
            " ffi=present"
        } else {
            " ffi=absent"
        });
        out.push_str(&format!(
            " unanalyzed_dependencies={}",
            self.unanalyzed_dependencies.len()
        ));
        out
    }

    /// Render human-readable diagnostic text.
    pub fn format_text(&self, verbose: bool) -> String {
        let mut out = String::new();

        if self.is_clean() {
            out.push_str(&format!(
                "PASS: package '{}' capability check succeeded ({} files scanned, {} bytes)\n",
                self.package, self.scanned_files_count, self.total_bytes_scanned
            ));
            // Present-but-excepted constructs are always shown, never only in verbose mode.
            if !self.excepted_constructs.is_empty() {
                out.push_str(&format!(
                    "  NOTE: {} banned construct occurrence(s) are PRESENT under manifest exception (not absent):\n",
                    self.excepted_constructs.len()
                ));
                for e in &self.excepted_constructs {
                    out.push_str(&format!(
                        "    - {} at {}:{}:{} ('{}')\n",
                        e.construct, e.file, e.line, e.col, e.evidence
                    ));
                }
            }
            for d in &self.unanalyzed_dependencies {
                out.push_str(&format!(
                    "  WARN: dependency '{d}' is not analyzed by this tool (VER08a G-1)\n"
                ));
            }
            if verbose {
                if self.detected_capabilities.is_empty() {
                    if self.excepted_constructs.is_empty() {
                        out.push_str("  No ambient OS capabilities detected (pure computation).\n");
                    } else {
                        out.push_str(
                            "  No ambient OS capabilities detected; banned constructs present under exception (see NOTE).\n",
                        );
                    }
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
            for e in &self.excepted_constructs {
                out.push_str(&format!(
                    "  NOTE: excepted {} at {}:{}:{} ('{}')\n",
                    e.construct, e.file, e.line, e.col, e.evidence
                ));
            }
        }
        out.push_str(&self.summary_line());
        out.push('\n');
        out
    }
}
