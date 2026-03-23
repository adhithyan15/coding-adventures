// Build plan serialization and deserialization.
//
// ==========================================================================
// Chapter 1: Why Build Plans?
// ==========================================================================
//
// A build plan is a snapshot of WHAT the build tool intends to build, in
// what ORDER, and with what COMMANDS. It serves three purposes:
//
//   1. **Auditability**: Before running a build, you can emit a plan
//      (`--emit-plan`) and review exactly what will happen. This is
//      especially useful in CI, where you want to log the build plan
//      alongside the build output for post-mortem analysis.
//
//   2. **Reproducibility**: A plan file can be saved, versioned, and
//      replayed. If a build breaks, you can examine the plan that was
//      used and compare it against the current plan to see what changed.
//
//   3. **Tooling integration**: Other tools (dashboards, notification
//      systems, build-status pages) can consume the JSON plan to display
//      what is being built without parsing log output.
//
// ==========================================================================
// Chapter 2: Schema Versioning
// ==========================================================================
//
// Build plans include a `schema_version` field. When we read a plan file,
// we check the version and reject plans from incompatible versions. This
// prevents subtle bugs when the plan format changes between build-tool
// releases.
//
// The current schema version is 1. When we make breaking changes to the
// plan format (adding required fields, changing field types, removing
// fields), we increment the version and add migration logic or a clear
// error message.
//
// ==========================================================================
// Chapter 3: Plan Structure
// ==========================================================================
//
// A build plan has this structure:
//
// ```json
// {
//   "schema_version": 1,
//   "created_at": "2026-03-22T10:30:00Z",
//   "diff_base": "origin/main",
//   "packages": [
//     {
//       "name": "python/logic-gates",
//       "language": "python",
//       "commands": ["pytest", "ruff check ."],
//       "reason": "changed"
//     },
//     {
//       "name": "python/boolean-algebra",
//       "language": "python",
//       "commands": ["pytest"],
//       "reason": "dependency_changed"
//     }
//   ]
// }
// ```
//
// The `reason` field explains WHY the package is in the build plan:
//   - `"changed"` — the package's own source files changed.
//   - `"dependency_changed"` — a transitive dependency changed.
//   - `"forced"` — the `--force` flag was used.
//   - `"cache_miss"` — no cached hash was found (first build).

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The current schema version for build plan JSON files.
///
/// Increment this when making breaking changes to the plan format.
/// The reader will reject plans with a different version, preventing
/// silent misinterpretation of fields.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single package entry in the build plan.
///
/// Each entry describes one package that will be built, including
/// its name, language, the commands that will be executed, and the
/// reason it was selected for building.
///
/// ## Fields
///
/// | Field    | Type       | Description                                    |
/// |----------|------------|------------------------------------------------|
/// | name     | String     | Qualified package name, e.g. "python/logic-gates" |
/// | language | String     | Language: "python", "go", "rust", etc.         |
/// | commands | Vec<String>| Shell commands to execute for the build         |
/// | reason   | String     | Why this package is being rebuilt               |
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageEntry {
    /// Qualified package name, e.g. "python/logic-gates".
    pub name: String,
    /// Inferred language: "python", "ruby", "go", "rust", etc.
    pub language: String,
    /// Shell commands to execute during the build.
    pub commands: Vec<String>,
    /// Reason the package is being rebuilt: "changed", "dependency_changed",
    /// "forced", or "cache_miss".
    pub reason: String,
}

/// A complete build plan describing what the build tool will execute.
///
/// The plan is a self-contained document that can be:
///   - Written to disk as JSON for auditing (`--emit-plan`).
///   - Read back for inspection or replay.
///   - Compared between runs to see what changed.
///
/// ## Schema versioning
///
/// The `schema_version` field ensures forward/backward compatibility.
/// When reading a plan, the tool checks the version and rejects
/// incompatible plans with a clear error message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildPlan {
    /// Schema version. Must match CURRENT_SCHEMA_VERSION for compatibility.
    pub schema_version: u32,
    /// ISO 8601 timestamp of when the plan was created.
    pub created_at: String,
    /// The git ref used as the diff base (e.g., "origin/main").
    pub diff_base: String,
    /// The list of packages to build, in dependency order.
    pub packages: Vec<PackageEntry>,
}

// ---------------------------------------------------------------------------
// Serialization / Deserialization
// ---------------------------------------------------------------------------

/// Write a build plan to disk as pretty-printed JSON.
///
/// Pretty-printing (with indentation) makes the plan human-readable,
/// which is important for the auditability use case. The performance
/// cost of pretty-printing is negligible for build plans (typically
/// tens of packages, not millions).
///
/// # Errors
///
/// Returns an error if:
///   - The plan cannot be serialized (shouldn't happen with valid data).
///   - The file cannot be written (permissions, disk full, etc.).
///
/// # Example
///
/// ```no_run
/// use build_tool::plan::{BuildPlan, PackageEntry, write_plan, CURRENT_SCHEMA_VERSION};
///
/// let plan = BuildPlan {
///     schema_version: CURRENT_SCHEMA_VERSION,
///     created_at: "2026-03-22T10:30:00Z".to_string(),
///     diff_base: "origin/main".to_string(),
///     packages: vec![
///         PackageEntry {
///             name: "python/logic-gates".to_string(),
///             language: "python".to_string(),
///             commands: vec!["pytest".to_string()],
///             reason: "changed".to_string(),
///         },
///     ],
/// };
///
/// write_plan(&plan, "/tmp/build-plan.json").unwrap();
/// ```
pub fn write_plan(bp: &BuildPlan, path: &str) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(bp)?;
    fs::write(Path::new(path), json)?;
    Ok(())
}

/// Read a build plan from a JSON file on disk.
///
/// After deserializing, this function checks the `schema_version` field.
/// If it does not match `CURRENT_SCHEMA_VERSION`, we return an error
/// rather than silently misinterpreting the data.
///
/// # Errors
///
/// Returns an error if:
///   - The file cannot be read (missing, permissions).
///   - The JSON is malformed.
///   - The schema version does not match.
///
/// # Example
///
/// ```no_run
/// use build_tool::plan::read_plan;
///
/// let plan = read_plan("/tmp/build-plan.json").unwrap();
/// println!("Plan has {} packages", plan.packages.len());
/// ```
pub fn read_plan(path: &str) -> Result<BuildPlan, Box<dyn Error>> {
    let data = fs::read_to_string(Path::new(path))?;
    let plan: BuildPlan = serde_json::from_str(&data)?;

    // Version gate: reject incompatible plans.
    //
    // This is intentionally strict. We could try to migrate old plans,
    // but for a build tool, clarity is more important than convenience.
    // A clear error message ("version 2, expected 1") is better than
    // silently misinterpreting fields.
    if plan.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "incompatible build plan schema version: got {}, expected {}",
            plan.schema_version, CURRENT_SCHEMA_VERSION
        )
        .into());
    }

    Ok(plan)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// These tests verify:
//   1. Round-trip serialization (write then read produces identical data).
//   2. Schema version rejection (reading a plan with wrong version fails).
//   3. Empty plans (valid edge case — nothing to build).
//   4. Malformed JSON handling.
//   5. Missing file handling.

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper to create a temporary file path unique to this test run.
    fn temp_plan_path(suffix: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_plan_{}_{}",
            std::process::id(),
            suffix
        ));
        dir.to_string_lossy().to_string()
    }

    /// Helper to build a sample plan for testing.
    fn sample_plan() -> BuildPlan {
        BuildPlan {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: "2026-03-22T10:30:00Z".to_string(),
            diff_base: "origin/main".to_string(),
            packages: vec![
                PackageEntry {
                    name: "python/logic-gates".to_string(),
                    language: "python".to_string(),
                    commands: vec![
                        "pytest".to_string(),
                        "ruff check .".to_string(),
                    ],
                    reason: "changed".to_string(),
                },
                PackageEntry {
                    name: "python/boolean-algebra".to_string(),
                    language: "python".to_string(),
                    commands: vec!["pytest".to_string()],
                    reason: "dependency_changed".to_string(),
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_round_trip() {
        let path = temp_plan_path("roundtrip.json");
        let plan = sample_plan();

        write_plan(&plan, &path).unwrap();
        let loaded = read_plan(&path).unwrap();

        assert_eq!(plan, loaded);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_round_trip_empty_packages() {
        let path = temp_plan_path("empty.json");
        let plan = BuildPlan {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: "2026-03-22T00:00:00Z".to_string(),
            diff_base: "HEAD~1".to_string(),
            packages: vec![],
        };

        write_plan(&plan, &path).unwrap();
        let loaded = read_plan(&path).unwrap();

        assert_eq!(plan, loaded);
        assert!(loaded.packages.is_empty());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_round_trip_single_package() {
        let path = temp_plan_path("single.json");
        let plan = BuildPlan {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            diff_base: "origin/main".to_string(),
            packages: vec![PackageEntry {
                name: "go/directed-graph".to_string(),
                language: "go".to_string(),
                commands: vec![
                    "go build ./...".to_string(),
                    "go test ./... -v -cover".to_string(),
                ],
                reason: "forced".to_string(),
            }],
        };

        write_plan(&plan, &path).unwrap();
        let loaded = read_plan(&path).unwrap();

        assert_eq!(plan, loaded);

        let _ = fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Schema version tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rejects_future_schema_version() {
        let path = temp_plan_path("future.json");

        // Write a plan with a future schema version directly.
        let json = r#"{
            "schema_version": 99,
            "created_at": "2026-03-22T10:30:00Z",
            "diff_base": "origin/main",
            "packages": []
        }"#;
        fs::write(&path, json).unwrap();

        let result = read_plan(&path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("incompatible"));
        assert!(err_msg.contains("99"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_rejects_zero_schema_version() {
        let path = temp_plan_path("zero.json");

        let json = r#"{
            "schema_version": 0,
            "created_at": "2026-03-22T10:30:00Z",
            "diff_base": "origin/main",
            "packages": []
        }"#;
        fs::write(&path, json).unwrap();

        let result = read_plan(&path);
        assert!(result.is_err());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_accepts_current_schema_version() {
        let path = temp_plan_path("current.json");

        let json = format!(
            r#"{{
                "schema_version": {},
                "created_at": "2026-03-22T10:30:00Z",
                "diff_base": "origin/main",
                "packages": []
            }}"#,
            CURRENT_SCHEMA_VERSION
        );
        fs::write(&path, json).unwrap();

        let result = read_plan(&path);
        assert!(result.is_ok());

        let _ = fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Error handling tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_missing_file() {
        let result = read_plan("/nonexistent/path/plan.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_malformed_json() {
        let path = temp_plan_path("malformed.json");
        fs::write(&path, "this is not json {{{").unwrap();

        let result = read_plan(&path);
        assert!(result.is_err());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_missing_required_field() {
        let path = temp_plan_path("missing_field.json");

        // JSON is valid but missing required fields.
        let json = r#"{"schema_version": 1}"#;
        fs::write(&path, json).unwrap();

        let result = read_plan(&path);
        assert!(result.is_err());

        let _ = fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Content verification tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_plan_json_is_pretty_printed() {
        let path = temp_plan_path("pretty.json");
        let plan = sample_plan();

        write_plan(&plan, &path).unwrap();
        let raw = fs::read_to_string(&path).unwrap();

        // Pretty-printed JSON has newlines and indentation.
        assert!(raw.contains('\n'));
        assert!(raw.contains("  ")); // At least 2-space indent.

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_plan_preserves_command_order() {
        let path = temp_plan_path("order.json");
        let plan = BuildPlan {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: "2026-03-22T10:30:00Z".to_string(),
            diff_base: "origin/main".to_string(),
            packages: vec![PackageEntry {
                name: "rust/parser".to_string(),
                language: "rust".to_string(),
                commands: vec![
                    "cargo build".to_string(),
                    "cargo test".to_string(),
                    "cargo clippy".to_string(),
                ],
                reason: "changed".to_string(),
            }],
        };

        write_plan(&plan, &path).unwrap();
        let loaded = read_plan(&path).unwrap();

        assert_eq!(loaded.packages[0].commands, vec![
            "cargo build",
            "cargo test",
            "cargo clippy",
        ]);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_plan_preserves_package_order() {
        let path = temp_plan_path("pkg_order.json");
        let plan = BuildPlan {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: "2026-03-22T10:30:00Z".to_string(),
            diff_base: "origin/main".to_string(),
            packages: vec![
                PackageEntry {
                    name: "z-package".to_string(),
                    language: "python".to_string(),
                    commands: vec![],
                    reason: "changed".to_string(),
                },
                PackageEntry {
                    name: "a-package".to_string(),
                    language: "go".to_string(),
                    commands: vec![],
                    reason: "forced".to_string(),
                },
            ],
        };

        write_plan(&plan, &path).unwrap();
        let loaded = read_plan(&path).unwrap();

        // Order must be preserved (z before a), not sorted.
        assert_eq!(loaded.packages[0].name, "z-package");
        assert_eq!(loaded.packages[1].name, "a-package");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_reason_values() {
        // Verify all expected reason values can round-trip.
        for reason in &["changed", "dependency_changed", "forced", "cache_miss"] {
            let path = temp_plan_path(&format!("reason_{}.json", reason));
            let plan = BuildPlan {
                schema_version: CURRENT_SCHEMA_VERSION,
                created_at: "2026-03-22T10:30:00Z".to_string(),
                diff_base: "origin/main".to_string(),
                packages: vec![PackageEntry {
                    name: "test/pkg".to_string(),
                    language: "python".to_string(),
                    commands: vec![],
                    reason: reason.to_string(),
                }],
            };

            write_plan(&plan, &path).unwrap();
            let loaded = read_plan(&path).unwrap();
            assert_eq!(loaded.packages[0].reason, *reason);

            let _ = fs::remove_file(&path);
        }
    }
}
