//! Pure, bounded extra-CI toolchain decisions over caller-supplied snapshots.
//!
//! This module deliberately has no filesystem, environment, process, Git,
//! clock, randomness, or network access. BUILD fronts are inert UTF-8 data.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BUILD_BYTES: usize = 65_536;
pub const MAX_BUILD_LINES: usize = 4_096;
pub const MAX_AGGREGATE_BUILD_BYTES: usize = 1_048_576;

pub const CANONICAL_TOOLCHAINS: [&str; 16] = [
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
];

const DECLARATION_PREFIX: &str = "# needs-toolchain:";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackageSnapshot {
    pub name: String,
    pub language: String,
    pub build_files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ToolchainDiagnostic {
    pub code: String,
    pub severity: String,
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainEvaluation {
    pub outcome: String,
    pub toolchains: BTreeMap<String, bool>,
    pub diagnostics: Vec<ToolchainDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    UnsupportedPlatform,
    PerFileByteLimit,
    PerFileLineLimit,
    AggregateByteLimit,
    ForceFullRequiresAllPackages,
}

fn logical_line_count(content: &str) -> usize {
    content.bytes().filter(|byte| *byte == b'\n').count() + 1
}

fn meter_front(content: &str) -> Result<usize, SnapshotError> {
    let bytes = content.len();
    if bytes > MAX_BUILD_BYTES {
        return Err(SnapshotError::PerFileByteLimit);
    }
    if logical_line_count(content) > MAX_BUILD_LINES {
        return Err(SnapshotError::PerFileLineLimit);
    }
    Ok(bytes)
}

fn is_canonical_toolchain(name: &str) -> bool {
    CANONICAL_TOOLCHAINS.binary_search(&name).is_ok()
}

fn trim_ascii_space_and_tab(value: &str) -> &str {
    value.trim_matches([' ', '\t'])
}

/// Parse exact inert declarations from one already supplied BUILD front.
///
/// This public helper retains the same per-file bounds as the top-level
/// evaluator so callers cannot bypass metering by invoking it directly.
pub fn parse_extra_toolchains(content: &str) -> Result<Vec<String>, SnapshotError> {
    meter_front(content)?;

    let last_index = logical_line_count(content) - 1;
    let mut seen = BTreeSet::new();
    let mut declarations = Vec::new();
    for (index, raw_line) in content.split('\n').enumerate() {
        let line = if index < last_index {
            raw_line.strip_suffix('\r').unwrap_or(raw_line)
        } else {
            raw_line
        };
        let line = trim_ascii_space_and_tab(line);
        let Some(suffix) = line.strip_prefix(DECLARATION_PREFIX) else {
            continue;
        };
        if !suffix.starts_with([' ', '\t']) {
            continue;
        }
        let name = trim_ascii_space_and_tab(suffix);
        if !is_canonical_toolchain(name) || !seen.insert(name) {
            continue;
        }
        declarations.push(name.to_string());
    }
    Ok(declarations)
}

fn front_precedence(platform: &str) -> Result<&'static [&'static str], SnapshotError> {
    match platform {
        "windows" => Ok(&["BUILD_windows", "BUILD"]),
        "darwin" => Ok(&["BUILD_mac", "BUILD_mac_and_linux", "BUILD"]),
        "linux" => Ok(&["BUILD_linux", "BUILD_mac_and_linux", "BUILD"]),
        _ => Err(SnapshotError::UnsupportedPlatform),
    }
}

fn selected_front<'a>(build_files: &'a BTreeMap<String, String>, precedence: &[&str]) -> &'a str {
    for front in precedence {
        if let Some(content) = build_files.get(*front) {
            return content;
        }
    }
    ""
}

fn toolchain_for_language(language: &str) -> Option<&str> {
    match language {
        "c" | "cpp" => Some("cpp"),
        "csharp" | "fsharp" | "dotnet" => Some("dotnet"),
        "wasm" => Some("rust"),
        canonical if is_canonical_toolchain(canonical) => Some(canonical),
        _ => None,
    }
}

fn fresh_toolchain_map(enabled: bool) -> BTreeMap<String, bool> {
    CANONICAL_TOOLCHAINS
        .into_iter()
        .map(|name| (name.to_string(), enabled))
        .collect()
}

fn unsupported(package: Option<&str>) -> ToolchainEvaluation {
    ToolchainEvaluation {
        outcome: "error".to_string(),
        toolchains: BTreeMap::new(),
        diagnostics: vec![ToolchainDiagnostic {
            code: "TOOLCHAIN_UNSUPPORTED".to_string(),
            severity: "error".to_string(),
            package: package.map(str::to_string),
        }],
    }
}

/// Evaluate a complete caller-owned toolchain snapshot without host access.
pub fn evaluate_snapshot(
    platform: &str,
    force_full: bool,
    packages: &[PackageSnapshot],
    scheduled_packages: Option<&[String]>,
    forced_toolchains: &[String],
) -> Result<ToolchainEvaluation, SnapshotError> {
    let precedence = front_precedence(platform)?;

    let mut aggregate_bytes = 0usize;
    for package in packages {
        for content in package.build_files.values() {
            let bytes = meter_front(content)?;
            aggregate_bytes = aggregate_bytes
                .checked_add(bytes)
                .ok_or(SnapshotError::AggregateByteLimit)?;
            if aggregate_bytes > MAX_AGGREGATE_BUILD_BYTES {
                return Err(SnapshotError::AggregateByteLimit);
            }
        }
    }

    if force_full && scheduled_packages.is_some() {
        return Err(SnapshotError::ForceFullRequiresAllPackages);
    }

    let scheduled: Option<BTreeSet<&str>> =
        scheduled_packages.map(|names| names.iter().map(String::as_str).collect());
    let mut selected = Vec::new();
    for package in packages.iter().filter(|package| {
        scheduled
            .as_ref()
            .is_none_or(|names| names.contains(package.name.as_str()))
    }) {
        let Some(toolchain) = toolchain_for_language(&package.language) else {
            return Ok(unsupported(Some(&package.name)));
        };
        selected.push((package, toolchain));
    }

    for forced in forced_toolchains {
        if !is_canonical_toolchain(forced) {
            return Ok(unsupported(None));
        }
    }

    let mut toolchains = fresh_toolchain_map(force_full);
    if !force_full {
        for (package, language_toolchain) in selected {
            toolchains.insert(language_toolchain.to_string(), true);
            for extra in parse_extra_toolchains(selected_front(&package.build_files, precedence))? {
                toolchains.insert(extra, true);
            }
        }
    }
    for forced in forced_toolchains {
        toolchains.insert(forced.clone(), true);
    }

    Ok(ToolchainEvaluation {
        outcome: "ok".to_string(),
        toolchains,
        diagnostics: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    struct FixtureCase {
        id: String,
        input: FixtureInput,
        expected: FixtureExpected,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureInput {
        options: FixtureOptions,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureOptions {
        platform: String,
        force_full: bool,
        packages: Vec<PackageSnapshot>,
        scheduled_packages: Option<Vec<String>>,
        forced_toolchains: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpected {
        outcome: String,
        #[serde(default)]
        result: FixtureResult,
        diagnostics: Vec<ToolchainDiagnostic>,
    }

    #[derive(Debug, Default, Deserialize)]
    struct FixtureResult {
        #[serde(default)]
        toolchains: BTreeMap<String, bool>,
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repository root")
            .join("code/specs/fixtures/build-tool-v1/cases")
    }

    fn package(
        name: &str,
        language: &str,
        fronts: impl IntoIterator<Item = (&'static str, String)>,
    ) -> PackageSnapshot {
        PackageSnapshot {
            name: name.to_string(),
            language: language.to_string(),
            build_files: fronts
                .into_iter()
                .map(|(front, content)| (front.to_string(), content))
                .collect(),
        }
    }

    #[test]
    fn consumes_every_language_neutral_toolchain_fixture() {
        let mut fixture_paths: Vec<PathBuf> = fs::read_dir(fixture_root())
            .expect("fixture directory")
            .map(|entry| entry.expect("fixture entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("toolchain-detection-") && name.ends_with(".json")
                    })
            })
            .collect();
        fixture_paths.sort();

        let names: Vec<&str> = fixture_paths
            .iter()
            .filter_map(|path| path.file_name()?.to_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "toolchain-detection-affected-only.json",
                "toolchain-detection-crlf-grammar.json",
                "toolchain-detection-declarations.json",
                "toolchain-detection-empty.json",
                "toolchain-detection-force-full.json",
                "toolchain-detection-null-all.json",
                "toolchain-detection-platform-darwin.json",
                "toolchain-detection-platform-linux.json",
                "toolchain-detection-platform-windows.json",
                "toolchain-detection-shared.json",
                "toolchain-detection-unsupported.json",
            ]
        );

        for path in fixture_paths {
            let case: FixtureCase = serde_json::from_slice(&fs::read(&path).expect("read fixture"))
                .expect("parse fixture");
            let options = case.input.options;
            let actual = evaluate_snapshot(
                &options.platform,
                options.force_full,
                &options.packages,
                options.scheduled_packages.as_deref(),
                &options.forced_toolchains,
            )
            .unwrap_or_else(|error| panic!("{}: {error:?}", case.id));
            assert_eq!(actual.outcome, case.expected.outcome, "{}", case.id);
            assert_eq!(
                actual.toolchains, case.expected.result.toolchains,
                "{}",
                case.id
            );
            assert_eq!(actual.diagnostics, case.expected.diagnostics, "{}", case.id);
        }
    }

    #[test]
    fn parser_enforces_utf8_byte_and_logical_line_limits_before_splitting() {
        let ascii_at_limit = "x".repeat(MAX_BUILD_BYTES);
        assert_eq!(
            parse_extra_toolchains(&ascii_at_limit).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            parse_extra_toolchains(&(ascii_at_limit + "x")),
            Err(SnapshotError::PerFileByteLimit)
        );

        let multibyte_at_limit = "é".repeat(MAX_BUILD_BYTES / 2);
        assert_eq!(multibyte_at_limit.len(), MAX_BUILD_BYTES);
        assert!(parse_extra_toolchains(&multibyte_at_limit).is_ok());
        assert_eq!(
            parse_extra_toolchains(&(multibyte_at_limit + "é")),
            Err(SnapshotError::PerFileByteLimit)
        );

        let at_line_limit = "x\n".repeat(MAX_BUILD_LINES - 1);
        assert!(parse_extra_toolchains(&at_line_limit).is_ok());
        assert_eq!(
            parse_extra_toolchains(&(at_line_limit + "\n")),
            Err(SnapshotError::PerFileLineLimit)
        );
    }

    #[test]
    fn declaration_grammar_is_byte_exact_and_stably_deduplicated() {
        let content = concat!(
            " # needs-toolchain: python \r\n",
            "# needs-toolchain:\tjava\t\n",
            "# needs-toolchain: python\n",
            "# needs-toolchain: ruby\r",
            "# needs-toolchain: lua\r  \n",
            "# needs-toolchain: perl\r\t\n",
            "# needs-toolchain: swift\r\r\n",
            "# needs-toolchain:python\n",
            "# Needs-toolchain: kotlin\n",
            "# needs-toolchain: zig\n",
            "# needs-toolchain: kotlin trailing\n",
        );
        assert_eq!(
            parse_extra_toolchains(content).unwrap(),
            vec!["python".to_string(), "java".to_string()]
        );
    }

    #[test]
    fn meters_every_front_and_the_aggregate_before_selection() {
        let unselected_oversized = package(
            "rust/app",
            "rust",
            [
                ("BUILD", String::new()),
                ("BUILD_windows", "x".repeat(MAX_BUILD_BYTES + 1)),
            ],
        );
        assert_eq!(
            evaluate_snapshot("linux", false, &[unselected_oversized], None, &[]),
            Err(SnapshotError::PerFileByteLimit)
        );

        let exact: Vec<PackageSnapshot> = (0..16)
            .map(|index| {
                package(
                    &format!("rust/exact-{index}"),
                    "rust",
                    [("BUILD", "x".repeat(MAX_BUILD_BYTES))],
                )
            })
            .collect();
        assert!(evaluate_snapshot("linux", false, &exact, Some(&[]), &[]).is_ok());

        let over: Vec<PackageSnapshot> = (0..17)
            .map(|index| {
                package(
                    &format!("rust/over-{index}"),
                    "rust",
                    [("BUILD", "x".repeat(MAX_BUILD_BYTES))],
                )
            })
            .collect();
        assert_eq!(
            evaluate_snapshot("linux", false, &over, Some(&[]), &[]),
            Err(SnapshotError::AggregateByteLimit)
        );
    }

    #[test]
    fn present_empty_platform_front_wins_over_generic_declarations() {
        let snapshot = package(
            "rust/app",
            "rust",
            [
                ("BUILD", "# needs-toolchain: python\n".to_string()),
                ("BUILD_linux", String::new()),
            ],
        );
        let actual = evaluate_snapshot("linux", false, &[snapshot], None, &[]).unwrap();
        assert!(actual.toolchains["rust"]);
        assert!(!actual.toolchains["python"]);
    }

    #[test]
    fn null_and_empty_schedules_are_distinct() {
        let snapshots = [package("python/app", "python", [("BUILD", String::new())])];
        let all = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        let none = evaluate_snapshot("linux", false, &snapshots, Some(&[]), &[]).unwrap();
        assert!(all.toolchains["python"]);
        assert!(none.toolchains.values().all(|enabled| !enabled));
    }

    #[test]
    fn aliases_forced_toolchains_and_force_full_are_closed() {
        let snapshots = [
            package("c/app", "c", [("BUILD", String::new())]),
            package("fsharp/app", "fsharp", [("BUILD", String::new())]),
            package("wasm/app", "wasm", [("BUILD", String::new())]),
        ];
        let selected =
            evaluate_snapshot("windows", false, &snapshots, None, &["kotlin".to_string()]).unwrap();
        assert!(selected.toolchains["cpp"]);
        assert!(selected.toolchains["dotnet"]);
        assert!(selected.toolchains["rust"]);
        assert!(selected.toolchains["kotlin"]);

        let full = evaluate_snapshot("windows", true, &snapshots, None, &[]).unwrap();
        assert_eq!(full.toolchains.len(), CANONICAL_TOOLCHAINS.len());
        assert!(full.toolchains.values().all(|enabled| *enabled));
    }

    #[test]
    fn selected_package_error_precedes_forced_toolchain_error_even_when_full() {
        let snapshots = [package("zig/app", "zig", [("BUILD", String::new())])];
        let actual =
            evaluate_snapshot("linux", true, &snapshots, None, &["zig".to_string()]).unwrap();
        assert_eq!(actual.outcome, "error");
        assert_eq!(actual.toolchains, BTreeMap::new());
        assert_eq!(
            actual.diagnostics,
            vec![ToolchainDiagnostic {
                code: "TOOLCHAIN_UNSUPPORTED".to_string(),
                severity: "error".to_string(),
                package: Some("zig/app".to_string()),
            }]
        );
    }

    #[test]
    fn unsupported_platform_fails_before_full_or_empty_schedule_shortcuts() {
        assert_eq!(
            evaluate_snapshot("solaris", true, &[], None, &[]),
            Err(SnapshotError::UnsupportedPlatform)
        );
        assert_eq!(
            evaluate_snapshot("solaris", false, &[], Some(&[]), &[]),
            Err(SnapshotError::UnsupportedPlatform)
        );
    }

    #[test]
    fn force_full_requires_the_null_schedule_shape() {
        assert_eq!(
            evaluate_snapshot("linux", true, &[], Some(&[]), &[]),
            Err(SnapshotError::ForceFullRequiresAllPackages)
        );
    }

    #[test]
    fn results_are_fresh_sorted_maps_and_inputs_are_unchanged() {
        let snapshots = [package(
            "rust/app",
            "rust",
            [("BUILD", "# needs-toolchain: python\n".to_string())],
        )];
        let original = snapshots.clone();
        let mut first = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        let second = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        first.toolchains.insert("rust".to_string(), false);
        assert!(second.toolchains["rust"]);
        assert_eq!(snapshots, original);
        assert_eq!(
            second
                .toolchains
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            CANONICAL_TOOLCHAINS
        );
    }
}
