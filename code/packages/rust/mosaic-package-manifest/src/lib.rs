//! # mosaic-package-manifest
//!
//! Parser and validator for `mosaic-package.toml` files, the manifest format
//! defined in **UI29 §4.1 / §4.2** (Mosaic Primitive Kernel — package layer).
//!
//! ## Why a manifest at all?
//!
//! The Mosaic primitive kernel needs to know three things before it can load
//! a package off disk:
//!
//! 1. **Who is this package?**            → `[package]` (name, version, …)
//! 2. **What does it publish?**           → `[components]` (PascalCase exports)
//! 3. **What does it need from me?**      → `[dependencies]` and `[kernel]`
//!
//! These four sections answer those questions, and **nothing else** is
//! permitted to depend on file layout or directory scanning.  The manifest is
//! the single source of truth for what a package is.
//!
//! ## Worked example
//!
//! ```
//! use mosaic_package_manifest::parse;
//!
//! let src = r#"
//! [package]
//! name = "mosaic-pkg-grid"
//! version = "0.1.0"
//! description = "Spreadsheet-style data grid"
//! license = "MIT OR Apache-2.0"
//!
//! [components]
//! exports = ["Grid", "Cell", "Column"]
//!
//! [dependencies]
//!
//! [kernel]
//! version = "1"
//! "#;
//!
//! let pkg = parse(src).expect("manifest valid");
//! assert_eq!(pkg.package.name, "mosaic-pkg-grid");
//! assert_eq!(pkg.components.exports, vec!["Grid", "Cell", "Column"]);
//! assert!(pkg.dependencies.is_empty());
//! assert_eq!(pkg.kernel.version, "1");
//! ```
//!
//! ## The validation philosophy
//!
//! This parser performs **structural** validation, not **interpretive**
//! validation.  That is:
//!
//! - It rejects `package.name = "Mosaic_Grid"` because the *shape* is wrong
//!   (kebab-case violation).
//! - It does **not** reject `package.license = "definitely a license"`
//!   because the meaning of that string belongs to the SPDX validator, not
//!   to the manifest parser.
//!
//! This boundary makes the parser cheap, testable, and stable: it never
//! needs to know about real-world identifiers, only about character classes.
//!
//! ## Error model in one table
//!
//! | Variant                 | Triggered by                                          |
//! |-------------------------|--------------------------------------------------------|
//! | `TomlSyntax`            | `toml::de::Error` — file is not even valid TOML       |
//! | `MissingField`          | a required `[section].field` is absent                |
//! | `InvalidPackageName`    | `package.name` or a dependency key not kebab-case     |
//! | `InvalidComponentName`  | an entry in `components.exports` not PascalCase       |
//! | `InvalidKernelVersion`  | `kernel.version` is anything other than `"1"`         |
//! | `InvalidSemverString`   | `package.version` or a dependency value not semver-y  |
//!
//! Each error is *one cause, one variant* — no compound errors, no batched
//! collection.  The first thing wrong with the manifest is the only thing
//! the caller hears about; fixing it and re-running is the workflow.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Public IR
// ---------------------------------------------------------------------------

/// A fully parsed and validated `mosaic-package.toml`.
///
/// Every field on every nested struct has already been syntactically checked
/// by the time you hold one of these — you can hand it to the kernel loader
/// without re-validating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MosaicPackage {
    pub package: PackageMeta,
    pub components: ComponentsSection,
    pub dependencies: HashMap<String, String>,
    pub host_assets: HostAssetsSection,
    pub kernel: KernelSection,
}

/// The `[package]` table: identity + metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMeta {
    /// Kebab-case identifier, e.g. `mosaic-pkg-grid`.
    pub name: String,
    /// Semver-like string, e.g. `0.1.0` or `1.2.3-rc.4`.
    pub version: String,
    /// Free-text human description; just needs to be non-empty.
    pub description: String,
    /// SPDX expression or any non-empty string the publisher chose.
    pub license: String,
}

/// The `[components]` table: PascalCase exports list.
///
/// An empty list is allowed — a package may be metadata-only (e.g. a
/// dependency aggregator that re-exports via its own dependencies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentsSection {
    pub exports: Vec<String>,
}

/// Optional files a package wants copied into generated host project shells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostAssetsSection {
    pub files: Vec<HostAsset>,
}

/// A single package-relative file copy into one backend output directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAsset {
    pub backend: String,
    pub source: String,
    pub target: String,
}

/// The `[kernel]` table: which ABI version of the primitive kernel this
/// package targets.  Currently only `"1"` is recognized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSection {
    pub version: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Everything that can go wrong while parsing a manifest.
///
/// Each variant maps to exactly one structural failure mode; see the table
/// in the crate-level docs for the full mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// The TOML did not even parse.  Payload is `toml::de::Error::to_string()`.
    TomlSyntax(String),
    /// A required field was absent.  E.g. `[package]` table missing entirely,
    /// or `version` missing from `[package]`.
    MissingField { section: String, field: String },
    /// A package name (in `[package].name` or a key under `[dependencies]`)
    /// did not match the kebab-case regex.
    InvalidPackageName(String),
    /// A component export name was not PascalCase.
    InvalidComponentName(String),
    /// `kernel.version` was something other than `"1"`.
    InvalidKernelVersion(String),
    /// A version string (in `[package].version` or a `[dependencies]` value)
    /// did not match the semver-like regex.
    InvalidSemverString(String),
}

impl std::fmt::Display for ManifestError {
    // The `Display` impl exists so consumers can `?` errors up to a CLI
    // top-level handler that just prints them.  We keep messages short and
    // diagnostic — they tell you *which* field broke, not how to fix it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TomlSyntax(e) => write!(f, "TOML syntax error: {e}"),
            Self::MissingField { section, field } => {
                write!(f, "missing required field [{section}].{field}")
            }
            Self::InvalidPackageName(n) => {
                write!(
                    f,
                    "invalid package name `{n}` (must be kebab-case starting with a letter)"
                )
            }
            Self::InvalidComponentName(n) => {
                write!(f, "invalid component name `{n}` (must be PascalCase)")
            }
            Self::InvalidKernelVersion(v) => {
                write!(
                    f,
                    "invalid kernel version `{v}` (only \"1\" is currently supported)"
                )
            }
            Self::InvalidSemverString(v) => {
                write!(f, "invalid semver-like version string `{v}`")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

// ---------------------------------------------------------------------------
// Raw deserialization shape
// ---------------------------------------------------------------------------
//
// We do **not** want serde to also do validation.  We make every section
// optional at the serde layer so that *missing-section* is reported by our
// own `MissingField` variant rather than by serde's generic
// "missing field `package`" message — that way the error model is uniform.

#[derive(Debug, Deserialize)]
struct RawManifest {
    package: Option<RawPackage>,
    components: Option<RawComponents>,
    dependencies: Option<HashMap<String, String>>,
    host_assets: Option<RawHostAssets>,
    kernel: Option<RawKernel>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawComponents {
    exports: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawHostAssets {
    files: Option<Vec<RawHostAsset>>,
}

#[derive(Debug, Deserialize)]
struct RawHostAsset {
    backend: Option<String>,
    source: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawKernel {
    version: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation regexes
// ---------------------------------------------------------------------------
//
// These three regexes encode the entire shape contract.  They are compiled
// at first use via a tiny manual `OnceLock` — no `lazy_static!`, no
// `once_cell` — so the dep tree stays at exactly {serde, toml, regex}.

use regex::Regex;
use std::sync::OnceLock;

fn kebab_case_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Kebab-case: lowercase letter start, then lowercase-alphanumeric, with
    // hyphens between segments.  Each segment must also start with a letter
    // (so `a-1b` is fine but `a--b` and `a-1` would each be wrong — the
    // second segment `1` would fail the segment-starts-with-letter rule).
    RE.get_or_init(|| Regex::new(r"^[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*$").unwrap())
}

fn pascal_case_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // PascalCase: uppercase letter start, then any alphanumerics.
    // (We do *not* require an internal uppercase — `A` is valid PascalCase.)
    RE.get_or_init(|| Regex::new(r"^[A-Z][a-zA-Z0-9]*$").unwrap())
}

fn semver_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Semver-like: MAJOR.MINOR.PATCH with an optional pre-release suffix.
    // We deliberately don't pull in the full `semver` crate — the manifest
    // shape only needs the *string form* to look right; the kernel loader
    // can do real version-range arithmetic when it resolves dependencies.
    RE.get_or_init(|| Regex::new(r"^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$").unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse and validate a `mosaic-package.toml` manifest from a string.
///
/// Returns `Err` on the first structural problem encountered; see
/// [`ManifestError`] for the exhaustive list of possible failures.
pub fn parse(toml_source: &str) -> Result<MosaicPackage, ManifestError> {
    // Step 1: turn raw bytes into TOML.  Any failure here is purely syntactic
    // and we wrap it as `TomlSyntax(message)`.
    let raw: RawManifest =
        toml::from_str(toml_source).map_err(|e| ManifestError::TomlSyntax(e.to_string()))?;

    // Step 2: every required section must exist.
    let raw_pkg = raw.package.ok_or(ManifestError::MissingField {
        section: "package".into(),
        field: "<section>".into(),
    })?;
    let raw_components = raw.components.ok_or(ManifestError::MissingField {
        section: "components".into(),
        field: "<section>".into(),
    })?;
    let raw_kernel = raw.kernel.ok_or(ManifestError::MissingField {
        section: "kernel".into(),
        field: "<section>".into(),
    })?;
    // `[dependencies]` is the one section that may be absent — a package
    // with no deps still has to declare *something* for the kernel to read,
    // but TOML conventionally lets you omit empty tables.  We accept either.
    let raw_deps = raw.dependencies.unwrap_or_default();
    let raw_host_assets = raw.host_assets;

    // Step 3: validate the `[package]` section field by field.
    let package = validate_package(raw_pkg)?;

    // Step 4: validate `[components]`.
    let components = validate_components(raw_components)?;

    // Step 5: validate `[dependencies]` — each (name, version) pair must
    // satisfy the same kebab/semver rules as `[package]`.
    let dependencies = validate_dependencies(raw_deps)?;

    // Step 6: validate optional host asset declarations.
    let host_assets = validate_host_assets(raw_host_assets)?;

    // Step 7: validate `[kernel]`.
    let kernel = validate_kernel(raw_kernel)?;

    Ok(MosaicPackage {
        package,
        components,
        dependencies,
        host_assets,
        kernel,
    })
}

/// Read a manifest file from disk and parse it.
///
/// IO errors are surfaced as `TomlSyntax(io error: …)` for a uniform error
/// model — the caller doesn't generally care whether the manifest was
/// unreadable or unparseable, only that it could not be loaded.
pub fn parse_path(path: &Path) -> Result<MosaicPackage, ManifestError> {
    let source = fs::read_to_string(path)
        .map_err(|e| ManifestError::TomlSyntax(format!("io error reading manifest: {e}")))?;
    parse(&source)
}

// ---------------------------------------------------------------------------
// Validators (one per section)
// ---------------------------------------------------------------------------

fn validate_package(raw: RawPackage) -> Result<PackageMeta, ManifestError> {
    // Every field on `[package]` is required.  We surface absence with
    // section+field labels so the error is actionable.
    let name = require(raw.name, "package", "name")?;
    let version = require(raw.version, "package", "version")?;
    let description = require(raw.description, "package", "description")?;
    let license = require(raw.license, "package", "license")?;

    // Structural checks.
    if !kebab_case_re().is_match(&name) {
        return Err(ManifestError::InvalidPackageName(name));
    }
    if !semver_re().is_match(&version) {
        return Err(ManifestError::InvalidSemverString(version));
    }
    if description.is_empty() {
        return Err(ManifestError::MissingField {
            section: "package".into(),
            field: "description".into(),
        });
    }
    if license.is_empty() {
        return Err(ManifestError::MissingField {
            section: "package".into(),
            field: "license".into(),
        });
    }

    Ok(PackageMeta {
        name,
        version,
        description,
        license,
    })
}

fn validate_components(raw: RawComponents) -> Result<ComponentsSection, ManifestError> {
    // `exports` is the only field, and it is required to be present (even
    // if empty).  This makes "I forgot to declare what I export" loud, and
    // distinguishes it from "I have nothing to export."
    let exports = require(raw.exports, "components", "exports")?;
    for name in &exports {
        if !pascal_case_re().is_match(name) {
            return Err(ManifestError::InvalidComponentName(name.clone()));
        }
    }
    Ok(ComponentsSection { exports })
}

fn validate_dependencies(
    raw: HashMap<String, String>,
) -> Result<HashMap<String, String>, ManifestError> {
    // Each (key, value) pair is a (package-name, semver) pair and gets
    // exactly the same validation as `[package]`.
    for (dep_name, dep_version) in &raw {
        if !kebab_case_re().is_match(dep_name) {
            return Err(ManifestError::InvalidPackageName(dep_name.clone()));
        }
        if !semver_re().is_match(dep_version) {
            return Err(ManifestError::InvalidSemverString(dep_version.clone()));
        }
    }
    Ok(raw)
}

fn validate_host_assets(raw: Option<RawHostAssets>) -> Result<HostAssetsSection, ManifestError> {
    let Some(raw) = raw else {
        return Ok(HostAssetsSection::default());
    };
    let raw_files = raw.files.unwrap_or_default();
    let mut files = Vec::with_capacity(raw_files.len());

    for file in raw_files {
        let backend = require_non_empty(file.backend, "host_assets.files", "backend")?;
        let source = require_non_empty(file.source, "host_assets.files", "source")?;
        let target = require_non_empty(file.target, "host_assets.files", "target")?;
        files.push(HostAsset {
            backend,
            source,
            target,
        });
    }

    Ok(HostAssetsSection { files })
}

fn validate_kernel(raw: RawKernel) -> Result<KernelSection, ManifestError> {
    let version = require(raw.version, "kernel", "version")?;
    // Only "1" is recognized today.  When the kernel ABI advances we'll add
    // "2" here and bump the major version of this crate — clients shouldn't
    // silently see new ABI versions accepted by an older manifest parser.
    if version != "1" {
        return Err(ManifestError::InvalidKernelVersion(version));
    }
    Ok(KernelSection { version })
}

/// Small helper: turn an `Option<T>` into either the value or a
/// `MissingField { section, field }`.  Keeps every validator uniform.
fn require<T>(opt: Option<T>, section: &str, field: &str) -> Result<T, ManifestError> {
    opt.ok_or_else(|| ManifestError::MissingField {
        section: section.into(),
        field: field.into(),
    })
}

fn require_non_empty(
    opt: Option<String>,
    section: &str,
    field: &str,
) -> Result<String, ManifestError> {
    let value = require(opt, section, field)?;
    if value.is_empty() {
        return Err(ManifestError::MissingField {
            section: section.into(),
            field: field.into(),
        });
    }
    Ok(value)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The happy-path manifest from the spec.  If this ever stops parsing
    /// cleanly, something fundamental is wrong with the validators.
    const GOOD_MANIFEST: &str = r#"
[package]
name = "mosaic-pkg-grid"
version = "0.1.0"
description = "Spreadsheet-style data grid built on UI29 kernel primitives"
license = "MIT OR Apache-2.0"

[components]
exports = ["Grid", "Cell", "Column"]

[dependencies]

[kernel]
version = "1"
"#;

    #[test]
    fn parses_good_manifest() {
        let pkg = parse(GOOD_MANIFEST).expect("good manifest must parse");
        assert_eq!(pkg.package.name, "mosaic-pkg-grid");
        assert_eq!(pkg.package.version, "0.1.0");
        assert_eq!(pkg.package.license, "MIT OR Apache-2.0");
        assert_eq!(pkg.components.exports, vec!["Grid", "Cell", "Column"]);
        assert!(pkg.dependencies.is_empty());
        assert!(pkg.host_assets.files.is_empty());
        assert_eq!(pkg.kernel.version, "1");
    }

    #[test]
    fn missing_package_section_errors() {
        let src = r#"
[components]
exports = []
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref section, .. } if section == "package"),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_components_section_errors() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref section, .. } if section == "components"),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_kernel_section_errors() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref section, .. } if section == "kernel"),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_non_kebab_package_name() {
        // Underscores are not kebab-case.
        let src = r#"
[package]
name = "Mosaic_Grid"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidPackageName(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_bad_semver() {
        let src = r#"
[package]
name = "x"
version = "1.0"
description = "x"
license = "MIT"
[components]
exports = []
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidSemverString(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_non_pascal_component_name() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = ["grid"]
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidComponentName(ref n) if n == "grid"),
            "got {err:?}"
        );
    }

    #[test]
    fn allows_empty_exports() {
        // A package can be metadata-only — see CHANGELOG note about
        // aggregator packages that re-export via dependencies.
        let src = r#"
[package]
name = "mosaic-pkg-meta"
version = "0.1.0"
description = "Metadata-only aggregator package"
license = "MIT"
[components]
exports = []
[kernel]
version = "1"
"#;
        let pkg = parse(src).expect("empty exports must be allowed");
        assert!(pkg.components.exports.is_empty());
    }

    #[test]
    fn parses_dependencies() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
[dependencies]
mosaic-pkg-grid = "0.2.0"
mosaic-pkg-form = "1.0.0-rc.1"
[kernel]
version = "1"
"#;
        let pkg = parse(src).expect("manifest valid");
        assert_eq!(pkg.dependencies.len(), 2);
        assert_eq!(pkg.dependencies["mosaic-pkg-grid"], "0.2.0");
        assert_eq!(pkg.dependencies["mosaic-pkg-form"], "1.0.0-rc.1");
    }

    #[test]
    fn parses_optional_host_asset_files() {
        let src = r#"
[package]
name = "mosaic-pkg-form"
version = "0.1.0"
description = "Form package"
license = "MIT"
[components]
exports = ["Form"]
[dependencies]
[host_assets]
files = [
  { backend = "react", source = "host/web/form-host.ts", target = "src/form-host.ts" },
  { backend = "xaml", source = "host/xaml/MosaicHost.cs", target = "MosaicHost.cs" },
]
[kernel]
version = "1"
"#;
        let pkg = parse(src).expect("manifest valid");
        assert_eq!(pkg.host_assets.files.len(), 2);
        assert_eq!(pkg.host_assets.files[0].backend, "react");
        assert_eq!(pkg.host_assets.files[0].source, "host/web/form-host.ts");
        assert_eq!(pkg.host_assets.files[0].target, "src/form-host.ts");
        assert_eq!(pkg.host_assets.files[1].backend, "xaml");
    }

    #[test]
    fn host_asset_files_require_source_and_target() {
        let src = r#"
[package]
name = "mosaic-pkg-form"
version = "0.1.0"
description = "Form package"
license = "MIT"
[components]
exports = ["Form"]
[dependencies]
[host_assets]
files = [
  { backend = "react", source = "host/web/form-host.ts" },
]
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref section, ref field } if section == "host_assets.files" && field == "target"),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_bad_dependency_name() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
[dependencies]
NotKebab = "0.1.0"
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidPackageName(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_bad_dependency_version() {
        let src = r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
[dependencies]
mosaic-pkg-grid = "not-a-version"
[kernel]
version = "1"
"#;
        let err = parse(src).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidSemverString(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_kernel_version_other_than_1() {
        for bad in ["0", "2", "1.0", ""] {
            let src = format!(
                r#"
[package]
name = "x"
version = "0.1.0"
description = "x"
license = "MIT"
[components]
exports = []
[kernel]
version = "{bad}"
"#
            );
            let err = parse(&src).unwrap_err();
            assert!(
                matches!(err, ManifestError::InvalidKernelVersion(_)),
                "for {bad:?} got {err:?}"
            );
        }
    }

    #[test]
    fn reports_toml_syntax_error() {
        // Unterminated string — TOML will refuse to even tokenize this.
        let src = "[package\nname = ";
        let err = parse(src).unwrap_err();
        assert!(matches!(err, ManifestError::TomlSyntax(_)), "got {err:?}");
    }

    #[test]
    fn parse_path_reads_from_filesystem() {
        // Write to a temp file inside the OS temp dir.  We pick a name that
        // includes the process ID so parallel test runs don't collide.
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "mosaic-package-manifest-test-{}.toml",
            std::process::id()
        ));
        fs::write(&tmp, GOOD_MANIFEST).expect("write tmp manifest");

        let pkg = parse_path(&tmp).expect("parse_path must succeed");
        assert_eq!(pkg.package.name, "mosaic-pkg-grid");

        // Clean up so we don't litter /tmp.
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn parse_path_reports_io_error() {
        // Path that almost certainly doesn't exist.
        let nope = Path::new("/this/path/should/not/exist/manifest.toml");
        let err = parse_path(nope).unwrap_err();
        assert!(matches!(err, ManifestError::TomlSyntax(_)));
    }
}
