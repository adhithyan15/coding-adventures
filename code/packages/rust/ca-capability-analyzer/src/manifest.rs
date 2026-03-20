//! # Manifest Loading and Capability Comparison
//!
//! This module loads a package's `required_capabilities.json` manifest and
//! compares it against the capabilities detected by the analyzer. The
//! comparison answers the question: *"Does this package use only the
//! capabilities it declared?"*
//!
//! ## Comparison Logic
//!
//! The comparison is **asymmetric**:
//!
//! - **Undeclared capabilities** (detected but not in manifest) are **ERRORS**.
//!   The code uses something it didn't declare. This is a security violation.
//!
//! - **Unused declarations** (in manifest but not detected) are **WARNINGS**.
//!   The manifest declares a capability the code doesn't use. This isn't
//!   a security issue — it's just a stale declaration that should be cleaned up.
//!
//! ```text
//! ┌────────────────────┐    ┌────────────────────┐
//! │  Detected by       │    │  Declared in        │
//! │  Analyzer          │    │  Manifest           │
//! │                    │    │                     │
//! │  ┌──────────┐      │    │  ┌──────────┐       │
//! │  │ MATCHED  │◄─────┼────┼──│ MATCHED  │       │
//! │  └──────────┘      │    │  └──────────┘       │
//! │  ┌──────────┐      │    │  ┌──────────┐       │
//! │  │ ERROR    │      │    │  │ WARNING  │       │
//! │  │(undeclrd)│      │    │  │ (unused) │       │
//! │  └──────────┘      │    │  └──────────┘       │
//! └────────────────────┘    └────────────────────┘
//! ```
//!
//! ## Default Deny
//!
//! If no `required_capabilities.json` exists, the package is treated as
//! having **zero** declared capabilities. Any detected capability is an
//! error. This is the "no manifest = block everything" principle.
//!
//! ## Target Matching
//!
//! When comparing detected targets against declared targets, we use
//! **fnmatch-style glob matching**:
//!
//! - `*` matches anything
//! - `data/*.txt` matches `data/config.txt`
//! - `config.toml` matches `config.toml` exactly
//!
//! This mirrors OpenBSD's `unveil()` path matching behavior.

use serde::{Deserialize, Serialize};

use crate::analyzer::DetectedCapability;

// ── Data Structures ──────────────────────────────────────────────────

/// A declared capability in the manifest.
///
/// Each capability has three fields that define what the package is
/// allowed to do:
///
/// - `category` — The resource kind (fs, net, proc, env, ffi)
/// - `action` — The operation (read, write, connect, exec, *, etc.)
/// - `target` — The specific resource or glob pattern ("*", "data/*.txt")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredCapability {
    pub category: String,
    pub action: String,
    pub target: String,
}

/// A parsed `required_capabilities.json` manifest.
///
/// This struct mirrors the JSON structure of the manifest file:
///
/// ```json
/// {
///   "package": "rust/my-package",
///   "capabilities": [
///     { "category": "fs", "action": "read", "target": "*.toml" }
///   ],
///   "justification": "Reads config files",
///   "banned_construct_exceptions": []
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: String,
    pub capabilities: Vec<DeclaredCapability>,
    #[serde(default)]
    pub justification: String,
    #[serde(default)]
    pub banned_construct_exceptions: Vec<serde_json::Value>,
    /// Path to the manifest file (not serialized from JSON)
    #[serde(skip)]
    pub path: Option<String>,
}

impl Manifest {
    /// True if the manifest declares zero capabilities.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// Load a manifest from a JSON file.
///
/// # Arguments
///
/// * `path` — Path to `required_capabilities.json`
///
/// # Returns
///
/// A parsed `Manifest`, or an error string.
///
/// # Errors
///
/// Returns an error if the file can't be read or isn't valid JSON.
pub fn load_manifest(path: &str) -> Result<Manifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read manifest {}: {}", path, e))?;

    let mut manifest: Manifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse manifest {}: {}", path, e))?;

    manifest.path = Some(path.to_string());
    Ok(manifest)
}

/// Create a default (empty) manifest for a package without one.
///
/// This represents the **default deny** policy: a package without a
/// `required_capabilities.json` is treated as declaring zero capabilities.
/// Any detected capability will be flagged as an error.
pub fn default_manifest(package_name: &str) -> Manifest {
    Manifest {
        package: package_name.to_string(),
        capabilities: Vec::new(),
        justification: "No manifest file — default deny (zero capabilities).".to_string(),
        banned_construct_exceptions: Vec::new(),
        path: None,
    }
}

// ── Comparison Result ────────────────────────────────────────────────

/// Result of comparing detected capabilities against a manifest.
///
/// This is the output of the core comparison logic. The CI gate uses
/// `passed` to determine whether to allow or block a package.
#[derive(Debug)]
pub struct ComparisonResult {
    /// True if all detected capabilities are covered by declarations.
    pub passed: bool,
    /// Detected capabilities not covered by any declaration (violations).
    pub errors: Vec<DetectedCapability>,
    /// Declared capabilities not matched by any detection (stale).
    pub warnings: Vec<DeclaredCapability>,
    /// Detected capabilities that matched a declaration.
    pub matched: Vec<DetectedCapability>,
}

impl ComparisonResult {
    /// Human-readable summary of the comparison result.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();

        if self.passed {
            lines.push("PASS — all detected capabilities are declared.".to_string());
        } else {
            lines.push(format!(
                "FAIL — {} undeclared capability(ies) detected.",
                self.errors.len()
            ));
        }

        if !self.errors.is_empty() {
            lines.push(String::new());
            lines.push("Undeclared capabilities (ERRORS):".to_string());
            for cap in &self.errors {
                lines.push(format!(
                    "  {}:{}: {} ({})",
                    cap.file, cap.line, cap, cap.evidence
                ));
            }
        }

        if !self.warnings.is_empty() {
            lines.push(String::new());
            lines.push("Unused declarations (WARNINGS):".to_string());
            for decl in &self.warnings {
                lines.push(format!(
                    "  {}:{}:{}",
                    decl.category, decl.action, decl.target
                ));
            }
        }

        if !self.matched.is_empty() {
            lines.push(format!("\nMatched: {} capability(ies).", self.matched.len()));
        }

        lines.join("\n")
    }
}

// ── Target Matching (fnmatch-style glob) ─────────────────────────────
//
// We implement a simple fnmatch-style glob matcher. The key patterns:
//
// - `*` matches any sequence of characters (but not path separators
//   in strict mode — we use loose mode here for simplicity)
// - `?` matches any single character
// - Literal characters match themselves
//
// We don't use an external crate to keep dependencies minimal.

/// Check if a string matches an fnmatch-style glob pattern.
///
/// Supports `*` (match any sequence) and `?` (match single char).
///
/// # Examples
///
/// ```text
/// fnmatch("*", "anything")          → true
/// fnmatch("*.txt", "data.txt")      → true
/// fnmatch("data/?.txt", "data/a.txt") → true
/// fnmatch("exact", "exact")         → true
/// fnmatch("exact", "other")         → false
/// ```
pub fn fnmatch(pattern: &str, text: &str) -> bool {
    // Convert pattern and text to char vectors for indexed access
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();

    // Dynamic programming approach:
    // dp[i][j] = true if pattern[0..i] matches text[0..j]
    //
    // This is a classic algorithm — each cell depends on:
    // - If pattern[i-1] == '*': dp[i][j] = dp[i-1][j] (skip *) OR dp[i][j-1] (consume char)
    // - If pattern[i-1] == '?' or matches text[j-1]: dp[i][j] = dp[i-1][j-1]
    // - Otherwise: dp[i][j] = false

    let p_len = pattern.len();
    let t_len = text.len();

    let mut dp = vec![vec![false; t_len + 1]; p_len + 1];
    dp[0][0] = true; // Empty pattern matches empty text

    // Handle leading '*' patterns that match empty text
    for i in 1..=p_len {
        if pattern[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=p_len {
        for j in 1..=t_len {
            if pattern[i - 1] == '*' {
                // '*' can match zero chars (dp[i-1][j]) or consume one more char (dp[i][j-1])
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if pattern[i - 1] == '?' || pattern[i - 1] == text[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }

    dp[p_len][t_len]
}

/// Check if a detected target matches a declared target pattern.
///
/// This wraps `fnmatch` with two special cases:
///
/// 1. If the declared pattern is `"*"`, it matches anything.
/// 2. If the detected target is `"*"` (unknown/wildcard), it matches
///    any declared pattern. This is conservative: we accept it rather
///    than generating a false positive, since we can't determine what
///    the actual runtime target will be.
fn target_matches(pattern: &str, actual: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if actual == "*" {
        // The detected target is a wildcard (non-literal). We can't know
        // what it will resolve to at runtime, so we accept it as matching
        // any declared pattern. This avoids false positives.
        return true;
    }
    fnmatch(pattern, actual)
}

/// Check if a detected capability matches a declared capability.
///
/// A match requires all three components to be compatible:
///
/// 1. **Category** must match exactly (fs == fs, but fs != net)
/// 2. **Action** must match exactly, or the declared action is `"*"`
/// 3. **Target** must match via glob (fnmatch)
fn capability_matches(declared: &DeclaredCapability, detected: &DetectedCapability) -> bool {
    if declared.category != detected.category {
        return false;
    }
    if declared.action != "*" && declared.action != detected.action {
        return false;
    }
    target_matches(&declared.target, &detected.target)
}

// ── Core Comparison Logic ────────────────────────────────────────────

/// Compare detected capabilities against a manifest.
///
/// This is the core comparison logic used by the CI gate. It determines
/// whether a package's source code uses only the capabilities it declared.
///
/// # Arguments
///
/// * `detected` — Capabilities found by the analyzer
/// * `manifest` — The package's declared capabilities
///
/// # Returns
///
/// A `ComparisonResult` with pass/fail status, errors, and warnings.
pub fn compare_capabilities(
    detected: &[DetectedCapability],
    manifest: &Manifest,
) -> ComparisonResult {
    let mut errors = Vec::new();
    let mut matched = Vec::new();

    // For each detected capability, check if any declaration covers it
    for cap in detected {
        let found_match = manifest
            .capabilities
            .iter()
            .any(|decl| capability_matches(decl, cap));

        if found_match {
            matched.push(cap.clone());
        } else {
            errors.push(cap.clone());
        }
    }

    // Find unused declarations (warnings)
    let warnings: Vec<DeclaredCapability> = manifest
        .capabilities
        .iter()
        .filter(|decl| !detected.iter().any(|cap| capability_matches(decl, cap)))
        .cloned()
        .collect();

    ComparisonResult {
        passed: errors.is_empty(),
        errors,
        warnings,
        matched,
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: create a DetectedCapability quickly ──

    fn det(category: &str, action: &str, target: &str) -> DetectedCapability {
        DetectedCapability {
            category: category.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            file: "test.rs".to_string(),
            line: 1,
            evidence: "test".to_string(),
        }
    }

    fn decl(category: &str, action: &str, target: &str) -> DeclaredCapability {
        DeclaredCapability {
            category: category.to_string(),
            action: action.to_string(),
            target: target.to_string(),
        }
    }

    fn manifest_with(caps: Vec<DeclaredCapability>) -> Manifest {
        Manifest {
            package: "test/pkg".to_string(),
            capabilities: caps,
            justification: "test".to_string(),
            banned_construct_exceptions: Vec::new(),
            path: None,
        }
    }

    // ── fnmatch tests ────────────────────────────────────────────────

    #[test]
    fn test_fnmatch_exact() {
        assert!(fnmatch("hello", "hello"));
        assert!(!fnmatch("hello", "world"));
    }

    #[test]
    fn test_fnmatch_star_matches_anything() {
        assert!(fnmatch("*", "anything"));
        assert!(fnmatch("*", ""));
        assert!(fnmatch("*", "multi/path/here"));
    }

    #[test]
    fn test_fnmatch_star_prefix() {
        assert!(fnmatch("*.txt", "data.txt"));
        assert!(fnmatch("*.txt", ".txt"));
        assert!(!fnmatch("*.txt", "data.csv"));
    }

    #[test]
    fn test_fnmatch_star_suffix() {
        assert!(fnmatch("data.*", "data.txt"));
        assert!(fnmatch("data.*", "data."));
        assert!(!fnmatch("data.*", "info.txt"));
    }

    #[test]
    fn test_fnmatch_star_middle() {
        assert!(fnmatch("data/*.txt", "data/config.txt"));
        assert!(fnmatch("data/*.txt", "data/x.txt"));
        assert!(!fnmatch("data/*.txt", "other/config.txt"));
    }

    #[test]
    fn test_fnmatch_question_mark() {
        assert!(fnmatch("?.txt", "a.txt"));
        assert!(!fnmatch("?.txt", "ab.txt"));
        assert!(!fnmatch("?.txt", ".txt"));
    }

    #[test]
    fn test_fnmatch_complex_pattern() {
        assert!(fnmatch("../../grammars/*.tokens", "../../grammars/python.tokens"));
        assert!(!fnmatch("../../grammars/*.tokens", "../../grammars/python.rules"));
    }

    #[test]
    fn test_fnmatch_empty() {
        assert!(fnmatch("", ""));
        assert!(!fnmatch("", "a"));
        assert!(!fnmatch("a", ""));
    }

    #[test]
    fn test_fnmatch_multiple_stars() {
        assert!(fnmatch("*/*", "a/b"));
        assert!(fnmatch("**", "anything"));
    }

    // ── target_matches tests ─────────────────────────────────────────

    #[test]
    fn test_target_matches_wildcard_pattern() {
        assert!(target_matches("*", "anything"));
        assert!(target_matches("*", ""));
    }

    #[test]
    fn test_target_matches_wildcard_actual() {
        // When detected target is "*", we accept any declared pattern
        assert!(target_matches("specific.txt", "*"));
        assert!(target_matches("*.txt", "*"));
    }

    #[test]
    fn test_target_matches_exact() {
        assert!(target_matches("data.txt", "data.txt"));
        assert!(!target_matches("data.txt", "other.txt"));
    }

    #[test]
    fn test_target_matches_glob() {
        assert!(target_matches("*.toml", "config.toml"));
        assert!(!target_matches("*.toml", "config.json"));
    }

    // ── capability_matches tests ─────────────────────────────────────

    #[test]
    fn test_capability_matches_exact() {
        let declared = decl("fs", "read", "data.txt");
        let detected = det("fs", "read", "data.txt");
        assert!(capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_wrong_category() {
        let declared = decl("fs", "read", "*");
        let detected = det("net", "read", "*");
        assert!(!capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_wrong_action() {
        let declared = decl("fs", "read", "*");
        let detected = det("fs", "write", "*");
        assert!(!capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_wildcard_action() {
        let declared = decl("fs", "*", "*");
        let detected = det("fs", "read", "data.txt");
        assert!(capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_wildcard_target() {
        let declared = decl("fs", "read", "*");
        let detected = det("fs", "read", "data.txt");
        assert!(capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_glob_target() {
        let declared = decl("fs", "read", "*.toml");
        let detected = det("fs", "read", "config.toml");
        assert!(capability_matches(&declared, &detected));
    }

    #[test]
    fn test_capability_matches_glob_target_no_match() {
        let declared = decl("fs", "read", "*.toml");
        let detected = det("fs", "read", "config.json");
        assert!(!capability_matches(&declared, &detected));
    }

    // ── compare_capabilities tests ───────────────────────────────────

    #[test]
    fn test_compare_all_matched() {
        let detected = vec![det("fs", "read", "data.txt")];
        let manifest = manifest_with(vec![decl("fs", "read", "*")]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(result.passed);
        assert!(result.errors.is_empty());
        assert_eq!(result.matched.len(), 1);
    }

    #[test]
    fn test_compare_undeclared_error() {
        let detected = vec![det("net", "connect", "evil.com:443")];
        let manifest = manifest_with(vec![decl("fs", "read", "*")]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].category, "net");
    }

    #[test]
    fn test_compare_unused_warning() {
        let detected = vec![det("fs", "read", "data.txt")];
        let manifest = manifest_with(vec![
            decl("fs", "read", "*"),
            decl("net", "connect", "*"),
        ]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(result.passed); // Unused declarations don't fail
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].category, "net");
    }

    #[test]
    fn test_compare_empty_manifest_default_deny() {
        let detected = vec![det("fs", "read", "data.txt")];
        let manifest = default_manifest("test/pkg");

        let result = compare_capabilities(&detected, &manifest);
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_compare_no_detections_passes() {
        let detected: Vec<DetectedCapability> = vec![];
        let manifest = manifest_with(vec![decl("fs", "read", "*")]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(result.passed);
        assert!(result.errors.is_empty());
        // The declaration is unused
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_compare_multiple_errors() {
        let detected = vec![
            det("fs", "read", "data.txt"),
            det("net", "connect", "api.com:443"),
            det("proc", "exec", "ls"),
        ];
        let manifest = manifest_with(vec![decl("fs", "read", "*")]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 2); // net and proc are undeclared
        assert_eq!(result.matched.len(), 1); // fs is matched
    }

    #[test]
    fn test_compare_wildcard_action_covers_all() {
        let detected = vec![
            det("fs", "read", "a.txt"),
            det("fs", "write", "b.txt"),
            det("fs", "delete", "c.txt"),
        ];
        let manifest = manifest_with(vec![decl("fs", "*", "*")]);

        let result = compare_capabilities(&detected, &manifest);
        assert!(result.passed);
        assert_eq!(result.matched.len(), 3);
    }

    // ── Manifest loading tests ───────────────────────────────────────

    #[test]
    fn test_default_manifest_is_empty() {
        let m = default_manifest("test/pkg");
        assert!(m.is_empty());
        assert_eq!(m.package, "test/pkg");
    }

    #[test]
    fn test_manifest_is_empty() {
        let m = manifest_with(vec![]);
        assert!(m.is_empty());

        let m2 = manifest_with(vec![decl("fs", "read", "*")]);
        assert!(!m2.is_empty());
    }

    #[test]
    fn test_load_manifest_from_string() {
        let json = r#"{
            "package": "rust/my-pkg",
            "capabilities": [
                { "category": "fs", "action": "read", "target": "*.toml" }
            ],
            "justification": "Reads config"
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.package, "rust/my-pkg");
        assert_eq!(manifest.capabilities.len(), 1);
        assert_eq!(manifest.capabilities[0].category, "fs");
        assert_eq!(manifest.capabilities[0].action, "read");
        assert_eq!(manifest.capabilities[0].target, "*.toml");
    }

    #[test]
    fn test_load_manifest_missing_optional_fields() {
        let json = r#"{
            "package": "rust/my-pkg",
            "capabilities": []
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.justification, "");
        assert!(manifest.banned_construct_exceptions.is_empty());
    }

    // ── Summary output tests ─────────────────────────────────────────

    #[test]
    fn test_summary_pass() {
        let result = ComparisonResult {
            passed: true,
            errors: vec![],
            warnings: vec![],
            matched: vec![det("fs", "read", "x")],
        };
        let summary = result.summary();
        assert!(summary.contains("PASS"));
    }

    #[test]
    fn test_summary_fail() {
        let result = ComparisonResult {
            passed: false,
            errors: vec![det("net", "connect", "evil.com")],
            warnings: vec![],
            matched: vec![],
        };
        let summary = result.summary();
        assert!(summary.contains("FAIL"));
        assert!(summary.contains("net:connect:evil.com"));
    }

    #[test]
    fn test_summary_with_warnings() {
        let result = ComparisonResult {
            passed: true,
            errors: vec![],
            warnings: vec![decl("proc", "exec", "*")],
            matched: vec![det("fs", "read", "x")],
        };
        let summary = result.summary();
        assert!(summary.contains("PASS"));
        assert!(summary.contains("Unused declarations"));
        assert!(summary.contains("proc:exec:*"));
    }
}
