use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use oxixml_unicode::{case, norm};
use serde::{Deserialize, Serialize};

use crate::discovery::Package;

const CI_MANAGED_TOOLCHAIN_LANGUAGES: &[&str] = &[
    "python",
    "ruby",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
    "java",
    "kotlin",
    "haskell",
];

const TRACKED_ARTIFACT_COMPONENT_IDENTITY: &str = "node_modules";
const TRACKED_ARTIFACT_REDACTED_PATH: &str = "repository";
pub const TRACKED_ARTIFACT_UNICODE_VERSION: &str = oxixml_unicode::UNICODE_VERSION;

const ORPHAN_SCAN_ROOT: &str = "code";
const ORPHAN_LEDGER_PATH: &str = "code/BUILD-EXEMPTIONS";
const ORPHAN_BUILD_NAMES: &[&str] = &[
    "BUILD",
    "BUILD_windows",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_mac_and_linux",
];
const ORPHAN_SKIP_COMPONENTS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "vendor",
    ".venv",
    "_build",
    "deps",
    ".build",
    "dist-newstyle",
    ".cargo",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedArtifactEntry {
    pub ordinal: u32,
    pub path: String,
    pub entry_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedArtifactDiagnosticDetails {
    pub ordinal: u32,
    pub entry_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedArtifactDiagnostic {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub details: TrackedArtifactDiagnosticDetails,
}

/// Derive tracked-artifact diagnostics from caller-supplied inert path records.
///
/// This oracle does not inspect Git, the filesystem, processes, the environment,
/// or the network. Entry kinds are metadata only and grant no path authority.
pub fn validate_tracked_artifact_snapshot(
    entries: &[TrackedArtifactEntry],
) -> Vec<TrackedArtifactDiagnostic> {
    validate_tracked_artifact_snapshot_with_version(TRACKED_ARTIFACT_UNICODE_VERSION, entries)
        .expect("the compiled tracked-artifact Unicode version is valid")
}

/// Derive diagnostics while requiring the caller's closed snapshot version.
pub fn validate_tracked_artifact_snapshot_with_version(
    unicode_version: &str,
    entries: &[TrackedArtifactEntry],
) -> Result<Vec<TrackedArtifactDiagnostic>, &'static str> {
    if unicode_version != TRACKED_ARTIFACT_UNICODE_VERSION {
        return Err("tracked artifact Unicode version must be 17.0.0");
    }
    let mut diagnostics = Vec::new();

    for entry in entries {
        let details = TrackedArtifactDiagnosticDetails {
            ordinal: entry.ordinal,
            entry_kind: entry.entry_kind.clone(),
            problem: None,
        };

        let normalized_path = match normalize_tracked_artifact_path(&entry.path) {
            Ok(path) => path,
            Err(problem) => {
                diagnostics.push(TrackedArtifactDiagnostic {
                    code: "TRACKED_ARTIFACT_PATH_INVALID".to_string(),
                    severity: "error".to_string(),
                    path: TRACKED_ARTIFACT_REDACTED_PATH.to_string(),
                    details: TrackedArtifactDiagnosticDetails {
                        problem: Some(problem.to_string()),
                        ..details
                    },
                });
                continue;
            }
        };

        if normalized_path.split('/').any(|component| {
            case::fold_str(&norm::nfkc(component)) == TRACKED_ARTIFACT_COMPONENT_IDENTITY
        }) {
            diagnostics.push(TrackedArtifactDiagnostic {
                code: "TRACKED_ARTIFACT_FORBIDDEN".to_string(),
                severity: "error".to_string(),
                path: normalized_path,
                details,
            });
        }
    }

    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.path.chars().cmp(right.path.chars()))
            .then_with(|| {
                canonical_details_key(&left.details).cmp(&canonical_details_key(&right.details))
            })
    });
    Ok(diagnostics)
}

fn normalize_tracked_artifact_path(path: &str) -> Result<String, &'static str> {
    let normalized = path.replace('\\', "/");
    if normalized.is_empty() {
        return Err("EMPTY");
    }
    if normalized.chars().count() > 512 {
        return Err("TOO_LONG");
    }
    if !norm::is_nfc(&normalized) && norm::nfc(&normalized) != normalized {
        return Err("NON_NFC");
    }
    if normalized.starts_with('/') {
        return Err("ABSOLUTE");
    }
    if normalized.as_bytes().get(1) == Some(&b':') && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err("DRIVE_QUALIFIED");
    }
    if normalized.contains("//") {
        return Err("EMPTY_SEGMENT");
    }
    if normalized.ends_with('/') {
        return Err("EMPTY_SEGMENT");
    }
    if normalized.chars().any(|character| {
        character < '\u{20}' || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
    }) {
        return Err("UNSAFE_CHARACTER");
    }

    for component in normalized.split('/') {
        if component == "." || component == ".." {
            return Err("DOT_SEGMENT");
        }
        if component.ends_with([' ', '.']) {
            return Err("TRAILING_DOT_OR_SPACE");
        }
        let basename = component.split('.').next().unwrap_or_default();
        let uppercase = case::full_uppercase(basename);
        if is_windows_reserved_basename(&uppercase) {
            return Err("RESERVED_BASENAME");
        }
    }

    Ok(normalized)
}

fn is_windows_reserved_basename(basename: &str) -> bool {
    if matches!(
        basename,
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
    ) {
        return true;
    }

    let suffix = basename
        .strip_prefix("COM")
        .or_else(|| basename.strip_prefix("LPT"));
    matches!(
        suffix,
        Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
    )
}

fn canonical_details_key(details: &TrackedArtifactDiagnosticDetails) -> String {
    let mut object = serde_json::Map::new();
    object.insert(
        "entry_kind".to_string(),
        serde_json::Value::String(details.entry_kind.clone()),
    );
    object.insert(
        "ordinal".to_string(),
        serde_json::Value::Number(details.ordinal.into()),
    );
    if let Some(problem) = &details.problem {
        object.insert(
            "problem".to_string(),
            serde_json::Value::String(problem.clone()),
        );
    }
    serde_json::to_string(&object).expect("tracked-artifact details are JSON-safe")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanManifest {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanBuildFile {
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanExemption {
    pub line: u32,
    pub kind: String,
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanCrateSnapshot {
    pub directories: Vec<String>,
    pub manifests: Vec<OrphanManifest>,
    pub build_files: Vec<OrphanBuildFile>,
    pub exemptions: Vec<OrphanExemption>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanCrateDiagnosticDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanCrateDiagnostic {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub details: OrphanCrateDiagnosticDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanCrateValidationResult {
    pub valid: bool,
    pub diagnostic_codes: Vec<String>,
    pub pending_exemption_count: usize,
    pub diagnostics: Vec<OrphanCrateDiagnostic>,
}

/// Validate a caller-supplied, closed orphan-crate snapshot.
///
/// The function is deliberately inert: it performs no filesystem, Git,
/// process, environment, or network access. Snapshot construction remains an
/// adapter responsibility so every implementation language can consume the
/// same language-neutral fixtures.
pub fn validate_orphan_crate_snapshot(
    snapshot: &OrphanCrateSnapshot,
) -> OrphanCrateValidationResult {
    let manifests: Vec<&OrphanManifest> = snapshot
        .manifests
        .iter()
        .filter(|manifest| !is_orphan_artifact_path(&manifest.path))
        .collect();
    let directories: BTreeSet<&str> = snapshot.directories.iter().map(String::as_str).collect();
    let manifest_paths: BTreeSet<&str> = manifests
        .iter()
        .map(|manifest| manifest.path.as_str())
        .collect();
    let coverage: BTreeMap<&str, Option<&OrphanBuildFile>> = manifests
        .iter()
        .map(|manifest| {
            (
                manifest.path.as_str(),
                find_covering_build(&snapshot.build_files, &manifest.path, "runnable"),
            )
        })
        .collect();
    let empty_builds: BTreeMap<&str, Option<&OrphanBuildFile>> = manifests
        .iter()
        .map(|manifest| {
            (
                manifest.path.as_str(),
                find_covering_build(&snapshot.build_files, &manifest.path, "empty"),
            )
        })
        .collect();

    let mut diagnostics = Vec::new();
    let mut seen_exemption_paths = BTreeSet::new();
    let mut valid_exemptions = Vec::new();

    for exemption in &snapshot.exemptions {
        let (identity, path_problem) = if !is_portable_orphan_path(&exemption.path) {
            (None, Some("PATH_UNSAFE"))
        } else {
            let identity = orphan_path_identity(&exemption.path);
            let problem = if !is_under_orphan_scan_root(&exemption.path) {
                Some("PATH_OUTSIDE_SCAN")
            } else if is_orphan_artifact_path(&exemption.path) {
                Some("PATH_ARTIFACT")
            } else {
                None
            };
            (Some(identity), problem)
        };

        let duplicate = identity
            .as_ref()
            .is_some_and(|identity| !seen_exemption_paths.insert(identity.clone()));
        let problem = if !matches!(exemption.kind.as_str(), "EXCLUDED" | "PENDING") {
            Some("UNKNOWN_KIND")
        } else if is_blank_orphan_reason(&exemption.reason) {
            Some("REASON_MISSING")
        } else if duplicate {
            Some("DUPLICATE_PATH")
        } else {
            path_problem
        };

        if let Some(problem) = problem {
            diagnostics.push(OrphanCrateDiagnostic {
                code: "ORPHAN_EXEMPTION_INVALID".to_string(),
                severity: "error".to_string(),
                path: ORPHAN_LEDGER_PATH.to_string(),
                details: OrphanCrateDiagnosticDetails {
                    line: Some(exemption.line),
                    problem: Some(problem.to_string()),
                    ..OrphanCrateDiagnosticDetails::default()
                },
            });
        } else {
            valid_exemptions.push(exemption);
        }
    }

    let mut active_exemptions = BTreeSet::new();
    let mut pending_exemption_count = 0;
    for exemption in valid_exemptions {
        let stale_problem = if !directories.contains(exemption.path.as_str()) {
            Some("MISSING_DIRECTORY")
        } else if !manifest_paths.contains(exemption.path.as_str()) {
            Some("NO_MANIFEST")
        } else if coverage
            .get(exemption.path.as_str())
            .and_then(|build| *build)
            .is_some()
        {
            Some("COVERED")
        } else {
            None
        };

        if let Some(problem) = stale_problem {
            diagnostics.push(OrphanCrateDiagnostic {
                code: "ORPHAN_EXEMPTION_STALE".to_string(),
                severity: "error".to_string(),
                path: ORPHAN_LEDGER_PATH.to_string(),
                details: OrphanCrateDiagnosticDetails {
                    entry_path: Some(exemption.path.clone()),
                    kind: Some(exemption.kind.clone()),
                    line: Some(exemption.line),
                    problem: Some(problem.to_string()),
                    ..OrphanCrateDiagnosticDetails::default()
                },
            });
            continue;
        }

        active_exemptions.insert(exemption.path.as_str());
        if exemption.kind == "PENDING" {
            pending_exemption_count += 1;
        }
    }

    for manifest in manifests {
        if coverage
            .get(manifest.path.as_str())
            .and_then(|build| *build)
            .is_some()
            || active_exemptions.contains(manifest.path.as_str())
        {
            continue;
        }

        let empty_build = empty_builds
            .get(manifest.path.as_str())
            .and_then(|build| *build);
        diagnostics.push(match empty_build {
            Some(build) => OrphanCrateDiagnostic {
                code: "ORPHAN_CRATE_EMPTY_BUILD".to_string(),
                severity: "error".to_string(),
                path: manifest.path.clone(),
                details: OrphanCrateDiagnosticDetails {
                    build_path: Some(build.path.clone()),
                    manifest_kind: Some(manifest.kind.clone()),
                    ..OrphanCrateDiagnosticDetails::default()
                },
            },
            None => OrphanCrateDiagnostic {
                code: "ORPHAN_CRATE_UNLISTED".to_string(),
                severity: "error".to_string(),
                path: manifest.path.clone(),
                details: OrphanCrateDiagnosticDetails {
                    manifest_kind: Some(manifest.kind.clone()),
                    ..OrphanCrateDiagnosticDetails::default()
                },
            },
        });
    }

    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.path.chars().cmp(right.path.chars()))
            .then_with(|| {
                canonical_orphan_details_key(&left.details)
                    .cmp(&canonical_orphan_details_key(&right.details))
            })
    });
    let diagnostic_codes: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    OrphanCrateValidationResult {
        valid: diagnostics.is_empty(),
        diagnostic_codes,
        pending_exemption_count,
        diagnostics,
    }
}

fn find_covering_build<'a>(
    build_files: &'a [OrphanBuildFile],
    manifest_path: &str,
    state: &str,
) -> Option<&'a OrphanBuildFile> {
    build_files
        .iter()
        .filter(|build| build.state == state)
        .filter_map(|build| {
            let parent = portable_parent_path(&build.path);
            let name = portable_basename(&build.path);
            if !is_under_orphan_scan_root(parent)
                || (manifest_path != parent && !manifest_path.starts_with(&format!("{parent}/")))
            {
                return None;
            }
            let rank = ORPHAN_BUILD_NAMES
                .iter()
                .position(|candidate| *candidate == name)?;
            Some((build, parent.matches('/').count() + 1, rank))
        })
        .min_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.path.chars().cmp(right.0.path.chars()))
        })
        .map(|candidate| candidate.0)
}

fn is_portable_orphan_path(path: &str) -> bool {
    if path.is_empty() || path.chars().count() > 512 || norm::nfc(path) != path {
        return false;
    }
    if path.starts_with('/') || path.contains('\\') || path.contains("//") {
        return false;
    }
    if path.as_bytes().get(1) == Some(&b':') && path.as_bytes()[0].is_ascii_alphabetic() {
        return false;
    }
    if path.chars().any(|character| {
        character < '\u{20}' || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
    }) {
        return false;
    }

    for component in path.split('/') {
        if component.is_empty()
            || matches!(component, "." | "..")
            || component.ends_with([' ', '.'])
        {
            return false;
        }
        let basename = component.split('.').next().unwrap_or_default();
        if is_windows_reserved_basename(&case::full_uppercase(basename)) {
            return false;
        }
    }
    true
}

fn is_blank_orphan_reason(reason: &str) -> bool {
    reason.is_empty()
        || reason.chars().all(|character| {
            character.is_whitespace() || ('\u{001c}'..='\u{001f}').contains(&character)
        })
}

fn orphan_path_identity(path: &str) -> String {
    case::fold_str(&norm::nfc(path))
}

fn is_under_orphan_scan_root(path: &str) -> bool {
    path == ORPHAN_SCAN_ROOT || path.starts_with("code/")
}

fn is_orphan_artifact_path(path: &str) -> bool {
    path.split('/')
        .any(|component| ORPHAN_SKIP_COMPONENTS.contains(&component))
}

fn portable_parent_path(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn portable_basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

fn canonical_orphan_details_key(details: &OrphanCrateDiagnosticDetails) -> String {
    let mut object = serde_json::Map::new();
    if let Some(build_path) = &details.build_path {
        object.insert(
            "build_path".to_string(),
            serde_json::Value::String(build_path.clone()),
        );
    }
    if let Some(entry_path) = &details.entry_path {
        object.insert(
            "entry_path".to_string(),
            serde_json::Value::String(entry_path.clone()),
        );
    }
    if let Some(kind) = &details.kind {
        object.insert("kind".to_string(), serde_json::Value::String(kind.clone()));
    }
    if let Some(line) = details.line {
        object.insert("line".to_string(), serde_json::Value::Number(line.into()));
    }
    if let Some(manifest_kind) = &details.manifest_kind {
        object.insert(
            "manifest_kind".to_string(),
            serde_json::Value::String(manifest_kind.clone()),
        );
    }
    if let Some(problem) = &details.problem {
        object.insert(
            "problem".to_string(),
            serde_json::Value::String(problem.clone()),
        );
    }
    serde_json::to_string(&object).expect("orphan diagnostic details are JSON-safe")
}

pub fn validate_ci_full_build_toolchains(repo_root: &Path, packages: &[Package]) -> Option<String> {
    let ci_path = repo_root.join(".github").join("workflows").join("ci.yml");
    let workflow = fs::read_to_string(&ci_path).ok()?;

    if !workflow.contains("Full build on main merge") {
        return None;
    }

    let compact_workflow: String = workflow.chars().filter(|c| !c.is_whitespace()).collect();
    let mut missing_output_binding = Vec::new();
    let mut missing_main_force = Vec::new();

    for lang in languages_needing_ci_toolchains(packages) {
        let output_binding = format!("needs_{lang}:${{{{steps.toolchains.outputs.needs_{lang}}}}}");
        if !compact_workflow.contains(&output_binding) {
            missing_output_binding.push(lang.clone());
        }

        if !compact_workflow.contains(&format!("needs_{lang}=true")) {
            missing_main_force.push(lang);
        }
    }

    if missing_output_binding.is_empty() && missing_main_force.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if !missing_output_binding.is_empty() {
        parts.push(format!(
            "detect outputs for forced main full builds are not normalized through steps.toolchains for: {}",
            missing_output_binding.join(", ")
        ));
    }
    if !missing_main_force.is_empty() {
        parts.push(format!(
            "forced main full-build path does not explicitly enable toolchains for: {}",
            missing_main_force.join(", ")
        ));
    }

    Some(format!(
        "{}: {}",
        ci_path.to_string_lossy().replace('\\', "/"),
        parts.join("; ")
    ))
}

pub fn validate_build_contracts(repo_root: &Path, packages: &[Package]) -> Option<String> {
    let mut errors = Vec::new();

    if let Some(error) = validate_ci_full_build_toolchains(repo_root, packages) {
        errors.push(error);
    }
    errors.extend(validate_lua_isolated_build_files(packages));
    errors.extend(validate_perl_build_files(packages));

    if errors.is_empty() {
        None
    } else {
        Some(errors.join("\n  - "))
    }
}

fn languages_needing_ci_toolchains(packages: &[Package]) -> Vec<String> {
    let mut langs = BTreeSet::new();
    for pkg in packages {
        if CI_MANAGED_TOOLCHAIN_LANGUAGES.contains(&pkg.language.as_str()) {
            langs.insert(pkg.language.clone());
        }
    }
    langs.into_iter().collect()
}

fn validate_lua_isolated_build_files(packages: &[Package]) -> Vec<String> {
    let mut errors = Vec::new();

    for pkg in packages {
        if pkg.language != "lua" {
            continue;
        }

        let self_rock = format!(
            "coding-adventures-{}",
            pkg.path
                .file_name()
                .map(|name| name.to_string_lossy().replace('_', "-"))
                .unwrap_or_default()
        );
        let mut build_lines = std::collections::BTreeMap::new();

        for build_path in lua_build_files(&pkg.path) {
            let lines = read_build_lines(&build_path);
            if let Some(name) = build_path.file_name().and_then(|value| value.to_str()) {
                build_lines.insert(name.to_string(), lines.clone());
            }
            if lines.is_empty() {
                continue;
            }

            if let Some(foreign_remove) = first_foreign_lua_remove(&lines, &self_rock) {
                errors.push(format!(
                    "{}: Lua BUILD removes unrelated rock {}; isolated package builds should only remove the package they are rebuilding",
                    build_path.to_string_lossy().replace('\\', "/"),
                    foreign_remove
                ));
            }

            let state_machine_index =
                first_line_containing(&lines, &["../state_machine", "..\\state_machine"]);
            let directed_graph_index =
                first_line_containing(&lines, &["../directed_graph", "..\\directed_graph"]);
            if let (Some(state_machine_index), Some(directed_graph_index)) =
                (state_machine_index, directed_graph_index)
            {
                if state_machine_index < directed_graph_index {
                    errors.push(format!(
                        "{}: Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first",
                        build_path.to_string_lossy().replace('\\', "/")
                    ));
                }
            }

            if (has_guarded_local_lua_install(&lines)
                || (build_path.file_name().and_then(|value| value.to_str()) == Some("BUILD_windows")
                    && has_local_lua_sibling_install(&lines)))
                && !self_install_disables_deps(&lines, &self_rock)
            {
                errors.push(format!(
                    "{}: Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest",
                    build_path.to_string_lossy().replace('\\', "/")
                ));
            }
        }

        let missing_windows_deps = missing_lua_sibling_installs(
            build_lines.get("BUILD").map(Vec::as_slice).unwrap_or(&[]),
            build_lines
                .get("BUILD_windows")
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        if !missing_windows_deps.is_empty() {
            errors.push(format!(
                "{}: Lua BUILD_windows is missing sibling installs present in BUILD: {}",
                pkg.path.join("BUILD_windows").to_string_lossy().replace('\\', "/"),
                missing_windows_deps.join(", ")
            ));
        }
    }

    errors
}

fn validate_perl_build_files(packages: &[Package]) -> Vec<String> {
    let mut errors = Vec::new();

    for pkg in packages {
        if pkg.language != "perl" {
            continue;
        }

        for build_path in lua_build_files(&pkg.path) {
            let lines = read_build_lines(&build_path);
            if lines.iter().any(|line| {
                line.contains("cpanm")
                    && line.contains("Test2::V0")
                    && !line.contains("--notest")
            }) {
                errors.push(format!(
                    "{}: Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself",
                    build_path.to_string_lossy().replace('\\', "/")
                ));
            }
        }
    }

    errors
}

fn lua_build_files(pkg_path: &Path) -> Vec<std::path::PathBuf> {
    let mut files = match fs::read_dir(pkg_path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.starts_with("BUILD"))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

fn read_build_lines(build_path: &Path) -> Vec<String> {
    let contents = match fs::read_to_string(build_path) {
        Ok(contents) => contents,
        Err(_) => return Vec::new(),
    };

    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn first_foreign_lua_remove(lines: &[String], self_rock: &str) -> Option<String> {
    for line in lines {
        let marker = "luarocks remove --force ";
        let Some(start) = line.find(marker) else {
            continue;
        };
        let remainder = &line[start + marker.len()..];
        let target = remainder
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if !target.is_empty() && target != self_rock {
            return Some(target.to_string());
        }
    }
    None
}

fn first_line_containing(lines: &[String], needles: &[&str]) -> Option<usize> {
    lines.iter().enumerate().find_map(|(index, line)| {
        needles
            .iter()
            .any(|needle| line.contains(needle))
            .then_some(index)
    })
}

fn has_guarded_local_lua_install(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| line.contains("luarocks show ") && (line.contains("../") || line.contains("..\\")))
}

fn has_local_lua_sibling_install(lines: &[String]) -> bool {
    !lua_sibling_install_dirs(lines).is_empty()
}

fn self_install_disables_deps(lines: &[String], self_rock: &str) -> bool {
    lines.iter().any(|line| {
        line.contains("luarocks make")
            && line.contains(self_rock)
            && (line.contains("--deps-mode=none")
                || line.contains("--deps-mode none")
                || line.contains("--no-manifest"))
    })
}

fn missing_lua_sibling_installs(unix_lines: &[String], windows_lines: &[String]) -> Vec<String> {
    let windows_deps: std::collections::BTreeSet<String> =
        lua_sibling_install_dirs(windows_lines).into_iter().collect();
    lua_sibling_install_dirs(unix_lines)
        .into_iter()
        .filter(|dep| !windows_deps.contains(dep))
        .collect()
}

fn lua_sibling_install_dirs(lines: &[String]) -> Vec<String> {
    let mut dirs = BTreeSet::new();

    for line in lines {
        if !line.contains("luarocks make") {
            continue;
        }
        let Some(start) = line.find("cd ") else {
            continue;
        };
        let remainder = &line[start + 3..];
        let dep = remainder.split_whitespace().next().unwrap_or_default();
        if !(dep.starts_with("../") || dep.starts_with("..\\")) {
            continue;
        }
        dirs.insert(dep.replace('\\', "/"));
    }

    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{
        validate_build_contracts, validate_ci_full_build_toolchains,
        validate_orphan_crate_snapshot, validate_tracked_artifact_snapshot,
        validate_tracked_artifact_snapshot_with_version, OrphanBuildFile, OrphanCrateDiagnostic,
        OrphanCrateSnapshot, OrphanExemption, OrphanManifest, TrackedArtifactDiagnostic,
        TrackedArtifactEntry, TRACKED_ARTIFACT_UNICODE_VERSION,
    };
    use crate::discovery::Package;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const TRACKED_ARTIFACT_CASES: &[&str] = &[
        "validation-tracked-artifacts-clean.json",
        "validation-tracked-artifacts-forbidden.json",
        "validation-tracked-artifacts-aliases.json",
        "validation-tracked-artifacts-invalid.json",
        "validation-tracked-artifacts-unicode-boundaries.json",
    ];

    const ORPHAN_CRATE_CASES: &[&str] = &[
        "validation-orphan-crates-clean.json",
        "validation-orphan-crates-unlisted.json",
        "validation-orphan-exemptions-invalid.json",
        "validation-orphan-exemptions-stale.json",
    ];

    fn repository_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("build-tool package must be four levels below the repository root")
            .to_path_buf()
    }

    #[test]
    fn orphan_crate_validation_matches_shared_conformance_fixtures() {
        let cases_root = repository_root().join("code/specs/fixtures/build-tool-v1/cases");

        for fixture_name in ORPHAN_CRATE_CASES {
            let fixture: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(cases_root.join(fixture_name)).unwrap())
                    .unwrap();
            let snapshot: OrphanCrateSnapshot =
                serde_json::from_value(fixture["input"]["options"]["orphan_snapshot"].clone())
                    .unwrap();
            let expected_diagnostics: Vec<OrphanCrateDiagnostic> =
                serde_json::from_value(fixture["expected"]["diagnostics"].clone()).unwrap();
            let expected_result = &fixture["expected"]["result"];

            let actual = validate_orphan_crate_snapshot(&snapshot);
            assert_eq!(
                actual.diagnostics, expected_diagnostics,
                "fixture {fixture_name}"
            );
            assert_eq!(
                actual.valid,
                expected_result["valid"].as_bool().unwrap(),
                "fixture {fixture_name}"
            );
            assert_eq!(
                actual.diagnostic_codes,
                serde_json::from_value::<Vec<String>>(expected_result["diagnostic_codes"].clone())
                    .unwrap(),
                "fixture {fixture_name}"
            );
            assert_eq!(
                actual.pending_exemption_count,
                expected_result["pending_exemption_count"].as_u64().unwrap() as usize,
                "fixture {fixture_name}"
            );
        }
    }

    #[test]
    fn orphan_crate_validation_redacts_unsafe_exemption_paths() {
        for unsafe_path in [
            String::new(),
            "a".repeat(513),
            "/absolute/secret-project".to_string(),
            "C:/host/secret-project".to_string(),
            "code/packages/rust/bad<name>".to_string(),
            "code/packages/rust/trailing.".to_string(),
            "code/packages/rust/CON".to_string(),
        ] {
            let result = validate_orphan_crate_snapshot(&OrphanCrateSnapshot {
                directories: vec!["code/packages/rust/demo".to_string()],
                manifests: vec![OrphanManifest {
                    path: "code/packages/rust/demo".to_string(),
                    kind: "package".to_string(),
                }],
                build_files: vec![],
                exemptions: vec![OrphanExemption {
                    line: 7,
                    kind: "PENDING".to_string(),
                    path: unsafe_path.clone(),
                    reason: "not allowed".to_string(),
                }],
            });

            let diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == "ORPHAN_EXEMPTION_INVALID")
                .unwrap();
            assert_eq!(diagnostic.path, "code/BUILD-EXEMPTIONS");
            assert_eq!(diagnostic.details.problem.as_deref(), Some("PATH_UNSAFE"));
            if !unsafe_path.is_empty() {
                assert!(!serde_json::to_string(&result)
                    .unwrap()
                    .contains(&unsafe_path));
            }
        }
    }

    #[test]
    fn orphan_crate_validation_uses_python_whitespace_for_reasons() {
        let result = validate_orphan_crate_snapshot(&OrphanCrateSnapshot {
            directories: vec!["code/packages/rust/demo".to_string()],
            manifests: vec![OrphanManifest {
                path: "code/packages/rust/demo".to_string(),
                kind: "package".to_string(),
            }],
            build_files: vec![],
            exemptions: vec![OrphanExemption {
                line: 7,
                kind: "PENDING".to_string(),
                path: "code/packages/rust/demo".to_string(),
                reason: "\u{001c}".to_string(),
            }],
        });

        assert_eq!(result.pending_exemption_count, 0);
        assert_eq!(
            result.diagnostic_codes,
            ["ORPHAN_CRATE_UNLISTED", "ORPHAN_EXEMPTION_INVALID"]
        );
        assert_eq!(
            result.diagnostics[1].details.problem.as_deref(),
            Some("REASON_MISSING")
        );
    }

    #[test]
    fn orphan_crate_validation_chooses_closest_empty_build_then_fixed_name_order() {
        let result = validate_orphan_crate_snapshot(&OrphanCrateSnapshot {
            directories: vec!["code/packages/rust/demo/child".to_string()],
            manifests: vec![OrphanManifest {
                path: "code/packages/rust/demo/child".to_string(),
                kind: "package".to_string(),
            }],
            build_files: vec![
                OrphanBuildFile {
                    path: "code/packages/rust/BUILD".to_string(),
                    state: "empty".to_string(),
                },
                OrphanBuildFile {
                    path: "code/packages/rust/demo/BUILD_linux".to_string(),
                    state: "empty".to_string(),
                },
                OrphanBuildFile {
                    path: "code/packages/rust/demo/BUILD".to_string(),
                    state: "empty".to_string(),
                },
            ],
            exemptions: vec![],
        });

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "ORPHAN_CRATE_EMPTY_BUILD");
        assert_eq!(
            result.diagnostics[0].details.build_path.as_deref(),
            Some("code/packages/rust/demo/BUILD")
        );
    }

    #[test]
    fn orphan_crate_validation_uses_nfc_full_casefold_duplicate_identity() {
        let result = validate_orphan_crate_snapshot(&OrphanCrateSnapshot {
            directories: vec!["code/packages/rust/Straße".to_string()],
            manifests: vec![OrphanManifest {
                path: "code/packages/rust/Straße".to_string(),
                kind: "package".to_string(),
            }],
            build_files: vec![],
            exemptions: vec![
                OrphanExemption {
                    line: 7,
                    kind: "EXCLUDED".to_string(),
                    path: "code/packages/rust/Straße".to_string(),
                    reason: "first".to_string(),
                },
                OrphanExemption {
                    line: 8,
                    kind: "PENDING".to_string(),
                    path: "CODE/PACKAGES/RUST/STRASSE".to_string(),
                    reason: "duplicate".to_string(),
                },
            ],
        });

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "ORPHAN_EXEMPTION_INVALID");
        assert_eq!(
            result.diagnostics[0].details.problem.as_deref(),
            Some("DUPLICATE_PATH")
        );
    }

    #[test]
    fn tracked_artifact_validation_matches_shared_conformance_fixtures() {
        let cases_root = repository_root().join("code/specs/fixtures/build-tool-v1/cases");

        for fixture_name in TRACKED_ARTIFACT_CASES {
            let fixture: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(cases_root.join(fixture_name)).unwrap())
                    .unwrap();
            let entries: Vec<TrackedArtifactEntry> = serde_json::from_value(
                fixture["input"]["options"]["tracked_artifact_snapshot"]["entries"].clone(),
            )
            .unwrap();
            let unicode_version = fixture["input"]["options"]["tracked_artifact_snapshot"]
                ["unicode_version"]
                .as_str()
                .unwrap();
            let expected: Vec<TrackedArtifactDiagnostic> =
                serde_json::from_value(fixture["expected"]["diagnostics"].clone()).unwrap();

            assert_eq!(
                validate_tracked_artifact_snapshot_with_version(unicode_version, &entries).unwrap(),
                expected,
                "fixture {fixture_name}"
            );
        }
    }

    #[test]
    fn tracked_artifact_validation_rejects_unicode_version_drift() {
        assert_eq!(TRACKED_ARTIFACT_UNICODE_VERSION, "17.0.0");
        assert_eq!(
            validate_tracked_artifact_snapshot_with_version("15.1.0", &[]),
            Err("tracked artifact Unicode version must be 17.0.0")
        );
    }

    #[test]
    fn tracked_artifact_validation_rejects_closed_path_errors_without_echoing_input() {
        assert_eq!(TRACKED_ARTIFACT_UNICODE_VERSION, "17.0.0");
        let cases = [
            (String::new(), "EMPTY"),
            ("a".repeat(513), "TOO_LONG"),
            ("code/packages/e\u{0301}/file.rs".to_string(), "NON_NFC"),
            ("/absolute/file.rs".to_string(), "ABSOLUTE"),
            ("C:/drive/file.rs".to_string(), "DRIVE_QUALIFIED"),
            ("code//empty/file.rs".to_string(), "EMPTY_SEGMENT"),
            ("code/trailing/".to_string(), "EMPTY_SEGMENT"),
            ("code\\trailing\\".to_string(), "EMPTY_SEGMENT"),
            ("code/bad?/file.rs".to_string(), "UNSAFE_CHARACTER"),
            ("code/../traversal".to_string(), "DOT_SEGMENT"),
            (
                "code/trailing./file.rs".to_string(),
                "TRAILING_DOT_OR_SPACE",
            ),
            ("code/COM1.txt/file.rs".to_string(), "RESERVED_BASENAME"),
        ];

        for (path, expected_problem) in cases {
            let diagnostics = validate_tracked_artifact_snapshot(&[TrackedArtifactEntry {
                ordinal: 7,
                path: path.clone(),
                entry_kind: "regular".to_string(),
            }]);

            assert_eq!(diagnostics.len(), 1, "path {path:?}");
            let diagnostic = &diagnostics[0];
            assert_eq!(diagnostic.code, "TRACKED_ARTIFACT_PATH_INVALID");
            assert_eq!(diagnostic.path, "repository");
            assert_eq!(
                diagnostic.details.problem.as_deref(),
                Some(expected_problem)
            );
            if !path.is_empty() {
                assert!(!serde_json::to_string(diagnostic).unwrap().contains(&path));
            }
        }

        // U+0300 has NFC_QC=Maybe, so the fast check is inconclusive even
        // when no canonical composition exists. Only an exact normalization
        // comparison may reject the already-normalized path.
        let nfc_quick_check_maybe = "code/packages/q\u{0300}/file.rs";
        assert!(validate_tracked_artifact_snapshot(&[TrackedArtifactEntry {
            ordinal: 8,
            path: nfc_quick_check_maybe.to_string(),
            entry_kind: "regular".to_string(),
        }])
        .is_empty());
    }

    fn make_package(root: &std::path::Path, rel_path: &str, language: &str) -> Package {
        let pkg_path = root.join(rel_path);
        fs::create_dir_all(&pkg_path).unwrap();
        Package {
            name: format!(
                "{language}/{}",
                pkg_path.file_name().unwrap().to_string_lossy()
            ),
            path: pkg_path,
            build_commands: vec!["echo hi".to_string()],
            language: language.to_string(),
        }
    }

    fn make_temp_root(label: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "build_tool_validator_{}_{}_{}",
            label,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn fails_without_normalized_outputs() {
        let root = make_temp_root("missing");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/workflows")).unwrap();

        let packages = vec![
            make_package(&root, "code/packages/elixir/actor", "elixir"),
            make_package(&root, "code/packages/python/actor", "python"),
        ];

        fs::write(
            root.join(".github/workflows/ci.yml"),
            r#"
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
"#,
        )
        .unwrap();

        let error = validate_ci_full_build_toolchains(&root, &packages).unwrap();
        assert!(error.contains(".github/workflows/ci.yml"));
        assert!(error.contains("python"));
        assert!(error.contains("elixir"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allows_normalized_outputs() {
        let root = make_temp_root("normalized");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/workflows")).unwrap();

        let packages = vec![
            make_package(&root, "code/packages/elixir/actor", "elixir"),
            make_package(&root, "code/packages/python/actor", "python"),
        ];

        fs::write(
            root.join(".github/workflows/ci.yml"),
            r#"
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
"#,
        )
        .unwrap();

        assert!(validate_ci_full_build_toolchains(&root, &packages).is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_lua_isolated_build_violations() {
        let root = make_temp_root("lua_violations");
        let _ = fs::remove_dir_all(&root);

        let problem_path = root.join("code/packages/lua/problem_pkg");
        fs::create_dir_all(&problem_path).unwrap();

        let packages = vec![Package {
            name: "lua/problem_pkg".to_string(),
            path: problem_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            problem_path.join("BUILD"),
            r#"
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("coding-adventures-branch-predictor"));
        assert!(error.contains("state_machine before directed_graph"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_guarded_lua_install_without_deps_mode() {
        let root = make_temp_root("lua_guarded");
        let _ = fs::remove_dir_all(&root);

        let guarded_path = root.join("code/packages/lua/guarded_pkg");
        fs::create_dir_all(&guarded_path).unwrap();

        let packages = vec![Package {
            name: "lua/guarded_pkg".to_string(),
            path: guarded_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            guarded_path.join("BUILD"),
            r#"
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("--deps-mode=none or --no-manifest"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_allows_safe_lua_patterns() {
        let root = make_temp_root("lua_safe");
        let _ = fs::remove_dir_all(&root);

        let safe_path = root.join("code/packages/lua/safe_pkg");
        fs::create_dir_all(&safe_path).unwrap();

        let packages = vec![Package {
            name: "lua/safe_pkg".to_string(),
            path: safe_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            safe_path.join("BUILD"),
            r#"
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();
        fs::write(
            safe_path.join("BUILD_windows"),
            r#"
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        assert!(validate_build_contracts(&root, &packages).is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_windows_lua_sibling_drift() {
        let root = make_temp_root("lua_windows_drift");
        let _ = fs::remove_dir_all(&root);

        let package_path = root.join("code/packages/lua/arm1_gatelevel");
        fs::create_dir_all(&package_path).unwrap();

        let packages = vec![Package {
            name: "lua/arm1_gatelevel".to_string(),
            path: package_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            package_path.join("BUILD"),
            r#"
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
"#,
        )
        .unwrap();
        fs::write(
            package_path.join("BUILD_windows"),
            r#"
(cd ..\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("BUILD_windows is missing sibling installs present in BUILD"));
        assert!(error.contains("../logic_gates"));
        assert!(error.contains("../arithmetic"));
        assert!(error.contains("--deps-mode=none or --no-manifest"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_perl_test2_bootstrap_without_notest() {
        let root = make_temp_root("perl_test2");
        let _ = fs::remove_dir_all(&root);

        let package_path = root.join("code/packages/perl/draw-instructions-svg");
        fs::create_dir_all(&package_path).unwrap();

        let packages = vec![Package {
            name: "perl/draw-instructions-svg".to_string(),
            path: package_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "perl".to_string(),
        }];

        fs::write(
            package_path.join("BUILD"),
            r#"
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("Test2::V0 without --notest"));

        let _ = fs::remove_dir_all(&root);
    }
}
