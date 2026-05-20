//! # mosaic-package-artifact-builder
//!
//! Per-backend package-artifact build mode for Mosaic packages, implementing
//! **UI29 §4.3** (Mosaic Primitive Kernel — "Compiling a package").
//!
//! ## The one question this crate answers
//!
//! Given:
//!
//! - a *package root* directory (containing `mosaic-package.toml` plus a
//!   `src/` tree of `.mil` / `.mll` / `.msl` triples), and
//! - a *backend* to target (React, SwiftUI, Qt, …),
//!
//! produce a directory of backend-specific source files that some host
//! application can consume the same way it would consume any other library.
//! Concretely: take a `mosaic-pkg-grid/` source package and produce a
//! `dist/react/{Grid.tsx, Cell.tsx, Column.tsx, index.ts}` (or
//! `dist/swiftui/{Grid.swift, Cell.swift, Column.swift, index.swift}`,
//! or `dist/qt/{Grid.qml, Cell.qml, Column.qml, qmldir}`).
//!
//! ## What this crate is *not*
//!
//! - It is not a *resolver*. We do **not** look up cross-package references
//!   here — every component is compiled in isolation against its own
//!   three-file triple. Resolving `Grid` inside another package's `.mll`
//!   is `mosaic-package-resolver`'s job.
//! - It does not yet wire WebComponent or HTML — those backends do not have
//!   a `from_pipeline` entry point yet. We return `UnsupportedBackend` for
//!   them so callers can still type the API surface uniformly.
//! - It does not modify the existing emitter crates. We *consume* their
//!   public `from_pipeline(interface, layout, style)` functions and treat
//!   them as opaque IR-to-string lowerings.
//!
//! ## The algorithm in 30 seconds
//!
//! ```text
//! build_package(opts)
//!     │
//!     ├── parse <package_root>/mosaic-package.toml
//!     │     (delegates to mosaic-package-manifest)
//!     │
//!     ├── for each <Component> in components.exports:
//!     │       ├── read src/<Component>.mil      (required)
//!     │       ├── read src/<Component>.mll      (required)
//!     │       ├── read src/<Component>.msl      (optional)
//!     │       │
//!     │       ├── mosmodel_compiler::compile      → interface IR
//!     │       ├── moslayout_compiler::compile     → layout IR
//!     │       ├── mosstyle_compiler::compile      → style IR (or empty default)
//!     │       │
//!     │       └── <backend>::from_pipeline(I, L, S)
//!     │             → write to <output>/<backend>/<Component>.<ext>
//!     │
//!     └── write index/qmldir under <output>/<backend>/
//! ```
//!
//! Each step has exactly one failure mode and exactly one error variant;
//! see [`BuildError`] for the exhaustive list.
//!
//! ## Why a separate crate (not part of `mosaic-compile`)?
//!
//! The `mosaic-compile` binary should be a thin CLI shell. Tests and
//! downstream tools want to invoke the package-build logic directly
//! (e.g. an IDE plugin that rebuilds a package on save). Putting the
//! algorithm in a library means it can be exercised without spawning a
//! subprocess and parsing stderr — the standard reason to factor a
//! library out from underneath its CLI.
//!
//! ## Worked example
//!
//! ```no_run
//! use std::path::PathBuf;
//! use mosaic_package_artifact_builder::{build_package, BuildOptions, Backend};
//!
//! let opts = BuildOptions {
//!     package_root: PathBuf::from("code/packages/mosaic-pkg-grid"),
//!     output_root:  PathBuf::from("/tmp/mosaic-pkg-grid-dist"),
//!     backend:      Backend::React,
//! };
//! let result = build_package(&opts).expect("package compiles");
//! for path in &result.artifacts {
//!     println!("wrote {}", path.display());
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use mosaic_package_manifest::{parse_path as parse_manifest, ManifestError};

// ===========================================================================
// Public types
// ===========================================================================

/// The set of backends this crate knows how to drive.
///
/// We list every UI29 §4.3 backend even though only three are wired up
/// today — that way callers (CLIs, IDE plugins) can build against the
/// final shape of the enum and we can return `UnsupportedBackend` for
/// the ones that aren't ready yet, instead of refusing to even compile
/// against the symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// React functional components in `.tsx` files.
    React,
    /// SwiftUI `View` structs in `.swift` files.
    SwiftUI,
    /// Qt Quick (QML) elements in `.qml` files.
    Qt,
    /// One self-registering JS file containing every component as a
    /// `<custom-element>`. Not yet wired — pending UI29 kernel completion
    /// for the WebComponent backend.
    WebComponent,
    /// A static-HTML snippet bundle. Not yet wired — pending UI29 kernel
    /// completion for the HTML backend.
    Html,
}

impl Backend {
    /// The on-disk subdirectory name beneath `output_root`.
    ///
    /// Conforms to the UI29 §4.3 layout: `dist/react/`, `dist/swiftui/`,
    /// `dist/qt/`, `dist/webcomponent/`, `dist/html/`.
    fn dir_name(self) -> &'static str {
        match self {
            Backend::React => "react",
            Backend::SwiftUI => "swiftui",
            Backend::Qt => "qt",
            Backend::WebComponent => "webcomponent",
            Backend::Html => "html",
        }
    }

    /// The file extension for a single component file.
    ///
    /// `None` for the not-yet-wired backends so the type system reminds us
    /// to handle them before adding a `<Component>.<ext>` write.
    fn component_extension(self) -> Option<&'static str> {
        match self {
            Backend::React => Some("tsx"),
            Backend::SwiftUI => Some("swift"),
            Backend::Qt => Some("qml"),
            Backend::WebComponent | Backend::Html => None,
        }
    }
}

/// Inputs to a package build. Owned paths are cheap and side-step lifetime
/// games for callers (e.g. CLIs constructing this from argv strings).
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Directory containing `mosaic-package.toml`. The component sources
    /// are expected at `<package_root>/src/<Component>.{mil,mll,msl}`.
    pub package_root: PathBuf,
    /// Root directory under which `<backend>/` is created.
    ///
    /// `<output_root>` is created if missing — the caller does not have to
    /// `mkdir -p` first.
    pub output_root: PathBuf,
    /// Which backend to compile for.
    pub backend: Backend,
}

/// What a successful build produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildResult {
    /// Every file we wrote, in the order they were written.
    ///
    /// Index/qmldir files come last (they reference the per-component
    /// outputs, so emitting them last makes the order match the eventual
    /// dependency order if a caller wants to stream-upload artifacts).
    pub artifacts: Vec<PathBuf>,
    /// The PascalCase names of every component we compiled.
    ///
    /// Same as `manifest.components.exports`, but threaded through here so
    /// callers don't have to re-parse the manifest to know what they got.
    pub components_built: Vec<String>,
}

/// Everything that can go wrong while building a package.
///
/// Each variant carries enough context to render a useful CLI message
/// without the caller having to re-read source files for line numbers —
/// the upstream compilers already include those in their `error` strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// The manifest at `<package_root>/mosaic-package.toml` did not parse
    /// or was invalid. The wrapped string is the rendered `ManifestError`.
    Manifest(String),
    /// A backend was requested that this crate version does not yet wire.
    /// Currently `WebComponent` and `Html`.
    UnsupportedBackend(Backend),
    /// The manifest's `[components].exports` listed a component name that
    /// matched no source file. (We don't error from this directly today —
    /// we error from [`SourceNotFound`] instead — but the variant exists
    /// so future cross-package-aware checks have a place to land without
    /// breaking the enum's exhaustive-match contract.)
    ///
    /// [`SourceNotFound`]: BuildError::SourceNotFound
    MissingComponent {
        package: String,
        component: String,
    },
    /// An exported component had no `<Component>.mil` / `<Component>.mll`
    /// pair under `src/`. The `.msl` is optional so its absence does not
    /// trigger this.
    SourceNotFound {
        component: String,
        expected_dir: PathBuf,
    },
    /// Three-language pipeline compilation failed for a component. The
    /// `error` string is the rendered `Display` form of whichever sub-
    /// compiler complained first — mosmodel, moslayout, mosstyle, or the
    /// backend's `PipelineEmitError`.
    PipelineError {
        component: String,
        error: String,
    },
    /// A read/write/mkdir call failed. The string is `io::Error::to_string()`
    /// because we don't want to leak `std::io::Error`'s `Send`-only quirks
    /// into our public API.
    Io(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Manifest(e) => write!(f, "manifest error: {e}"),
            BuildError::UnsupportedBackend(b) => {
                write!(f, "backend {b:?} is not yet wired in this build")
            }
            BuildError::MissingComponent { package, component } => write!(
                f,
                "package '{package}' lists component '{component}' but no \
                 source file matched"
            ),
            BuildError::SourceNotFound { component, expected_dir } => write!(
                f,
                "no '{component}.mil'/'{component}.mll' under {}",
                expected_dir.display()
            ),
            BuildError::PipelineError { component, error } => {
                write!(f, "pipeline error for component '{component}': {error}")
            }
            BuildError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<ManifestError> for BuildError {
    fn from(e: ManifestError) -> Self {
        BuildError::Manifest(e.to_string())
    }
}

// ===========================================================================
// Public entry point
// ===========================================================================

/// Build a package's artifact for a single backend.
///
/// See the crate-level docs for the full algorithm. The high-level contract
/// is:
///
/// - On success, returns the list of files written and the components
///   built. The output tree is `<output_root>/<backend>/...`; we never
///   touch files outside that subdirectory.
/// - On failure, returns the first error encountered. Partial output may
///   exist on disk — we do not roll back the filesystem, because callers
///   either build into a temp dir and `mv` (the npm/Cargo pattern) or
///   build into a stable dist directory and treat a half-written tree as
///   the same outcome as a successful rebuild that subsequently failed.
pub fn build_package(opts: &BuildOptions) -> Result<BuildResult, BuildError> {
    // ----- 1. Validate the backend up front --------------------------------
    //
    // Returning `UnsupportedBackend` before any I/O happens means a CLI
    // user can pass `--backend webcomponent` against any directory and
    // see the actionable "backend not wired" message immediately instead
    // of after the manifest parse succeeds.
    if opts.backend.component_extension().is_none() {
        return Err(BuildError::UnsupportedBackend(opts.backend));
    }

    // ----- 2. Read the manifest --------------------------------------------
    let manifest_path = opts.package_root.join("mosaic-package.toml");
    let manifest = parse_manifest(&manifest_path)?;

    // ----- 3. Prepare the output directory ---------------------------------
    //
    // `create_dir_all` is the friendly "mkdir -p" — it is *not* an error
    // if the directory already exists, which is the behaviour we want
    // for incremental rebuilds.
    let backend_dir = opts.output_root.join(opts.backend.dir_name());
    create_dir_all(&backend_dir)?;

    // ----- 4. Compile each component ---------------------------------------
    let src_dir = opts.package_root.join("src");
    let mut artifacts = Vec::new();
    let mut components_built = Vec::new();

    for component in &manifest.components.exports {
        let artifact = compile_one_component(component, &src_dir, &backend_dir, opts.backend)?;
        artifacts.push(artifact);
        components_built.push(component.clone());
    }

    // ----- 5. Emit the per-backend index / qmldir --------------------------
    //
    // The index file lists every component built. Empty packages still get
    // an index — it's just empty — so downstream tools don't have to
    // special-case "package with zero components".
    let index_path = emit_index_file(&backend_dir, &components_built, opts.backend, &manifest.package.name)?;
    artifacts.push(index_path);

    Ok(BuildResult {
        artifacts,
        components_built,
    })
}

// ===========================================================================
// Per-component pipeline
// ===========================================================================

/// Compile one component's three-file triple for the chosen backend.
///
/// Returns the path of the written artifact, or a [`BuildError`] tagged
/// with the component name so a CLI can render
/// `mosaic-compile pkg: error compiling Grid: …`.
fn compile_one_component(
    component: &str,
    src_dir: &Path,
    out_dir: &Path,
    backend: Backend,
) -> Result<PathBuf, BuildError> {
    // ----- 1. Locate the three source files --------------------------------
    //
    // `.mil` and `.mll` are required; `.msl` is optional. If the user has
    // multiple `.msl` variants (e.g. `Grid.dark.msl`), we pick `<Component>.msl`
    // first and fall back to *any* `.msl` whose stem begins with `<Component>.`.
    // For the v1 packager we keep this simple: only the un-themed `.msl`
    // matters. Theme handling is a follow-up.
    let mil_path = src_dir.join(format!("{component}.mil"));
    let mll_path = src_dir.join(format!("{component}.mll"));
    let msl_path = src_dir.join(format!("{component}.msl"));

    if !mil_path.exists() || !mll_path.exists() {
        return Err(BuildError::SourceNotFound {
            component: component.to_string(),
            expected_dir: src_dir.to_path_buf(),
        });
    }

    let mil_src = read_to_string(&mil_path)?;
    let mll_src = read_to_string(&mll_path)?;
    let msl_src = if msl_path.exists() {
        read_to_string(&msl_path)?
    } else {
        // Defensive default: an empty style block targeting this component.
        // The mosstyle compiler accepts an empty `style {}` body and
        // produces an empty StyleDef, which downstream emitters happily
        // ignore. This is cleaner than wiring a "skip style" path through
        // every backend's `from_pipeline`.
        format!("style {component} {{ }}")
    };

    // ----- 2. Run the three-language pipeline ------------------------------
    //
    // Each compile call may return a `Vec<CompileError>`. We render the
    // first one and wrap it as `PipelineError` so the caller gets one
    // line per component rather than a flood.
    let mosmodel_out = mosmodel_compiler::compile(&mil_src)
        .map_err(|errs| pipeline_err(component, &errs[0]))?;

    let layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;

    let style_out = mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;

    // ----- 3. Hand the three IRs to the chosen backend ---------------------
    let emitted = match backend {
        Backend::React => mosaic_emit_react::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_out.def,
        )
        .map(|r| r.output)
        .map_err(|e| BuildError::PipelineError {
            component: component.to_string(),
            error: e.to_string(),
        })?,
        Backend::SwiftUI => mosaic_emit_swiftui::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_out.def,
        )
        .map(|r| r.output)
        .map_err(|e| BuildError::PipelineError {
            component: component.to_string(),
            error: e.to_string(),
        })?,
        Backend::Qt => mosaic_emit_qt::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_out.def,
        )
        .map(|r| r.output)
        .map_err(|e| BuildError::PipelineError {
            component: component.to_string(),
            error: e.to_string(),
        })?,
        // Unreachable because `build_package` rejects these up front, but
        // we re-check defensively to keep this match exhaustive.
        Backend::WebComponent | Backend::Html => {
            return Err(BuildError::UnsupportedBackend(backend));
        }
    };

    // ----- 4. Write the artifact -------------------------------------------
    let ext = backend
        .component_extension()
        .expect("checked at function entry");
    let artifact_path = out_dir.join(format!("{component}.{ext}"));
    write_file(&artifact_path, emitted.as_bytes())?;
    Ok(artifact_path)
}

/// Render a sub-compiler's first error as a `BuildError::PipelineError`.
///
/// The sub-compilers all expose `CompileError` types whose `Debug` impl is
/// readable enough; we use that as the message body.
fn pipeline_err<E: std::fmt::Debug>(component: &str, err: &E) -> BuildError {
    BuildError::PipelineError {
        component: component.to_string(),
        error: format!("{err:?}"),
    }
}

// ===========================================================================
// Index / qmldir emitters
// ===========================================================================

/// Emit the per-backend index file that re-exports every component.
///
/// The exact format differs by backend (see inline comments) but the
/// purpose is the same: hosts import the package as a *single unit*
/// rather than reaching into per-component files by name.
fn emit_index_file(
    backend_dir: &Path,
    components: &[String],
    backend: Backend,
    package_name: &str,
) -> Result<PathBuf, BuildError> {
    match backend {
        Backend::React => {
            // `index.ts` lives alongside `Grid.tsx`, `Cell.tsx`, …
            //
            // We use `export * from "./Grid"` (no `.tsx` extension) so the
            // TypeScript module resolver picks the right file regardless
            // of whether the host's `tsconfig.json` has
            // `"allowImportingTsExtensions": true`.
            let path = backend_dir.join("index.ts");
            let mut body = String::new();
            body.push_str("// Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            body.push_str(&format!("// Package: {package_name}\n\n"));
            for c in components {
                body.push_str(&format!("export * from \"./{c}\";\n"));
            }
            write_file(&path, body.as_bytes())?;
            Ok(path)
        }
        Backend::SwiftUI => {
            // SwiftPM-shaped output is overkill for v1. We emit a single
            // `index.swift` that re-imports the per-component files via
            // `@_exported import` so a host's `import MosaicPkgGrid` brings
            // everything in scope.
            //
            // We do NOT generate a real Package.swift here — that requires
            // module-naming decisions tied to the host's SwiftPM setup
            // (target name, swift-tools-version), which UI29 §4.3 calls
            // out as out-of-scope for this PR.
            let path = backend_dir.join("index.swift");
            let mut body = String::new();
            body.push_str("// Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            body.push_str(&format!("// Package: {package_name}\n\n"));
            for c in components {
                body.push_str(&format!("// Component: {c}\n"));
            }
            write_file(&path, body.as_bytes())?;
            Ok(path)
        }
        Backend::Qt => {
            // `qmldir` is the Qt module descriptor. Each line is
            // `<TypeName> <version> <RelativePath>.qml`. For a single-version
            // package we hard-code `1.0`, which matches the QtQuick.Layouts
            // 1.15 imports the Qt emitter writes.
            //
            // The `module` line gives a fully-qualified import name; we
            // derive it from the package name by stripping the
            // `mosaic-pkg-` prefix and PascalCasing (`mosaic-pkg-grid`
            // → `MosaicPkg.Grid`). For an aggregator package without that
            // prefix we just PascalCase the whole thing.
            let path = backend_dir.join("qmldir");
            let module_name = qmldir_module_name(package_name);
            let mut body = String::new();
            body.push_str("# Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            body.push_str(&format!("module {module_name}\n"));
            for c in components {
                body.push_str(&format!("{c} 1.0 {c}.qml\n"));
            }
            write_file(&path, body.as_bytes())?;
            Ok(path)
        }
        Backend::WebComponent | Backend::Html => {
            // Already rejected up front, but the match must be exhaustive.
            Err(BuildError::UnsupportedBackend(backend))
        }
    }
}

/// Map a kebab-case package name to a Qt-friendly module name.
///
/// `mosaic-pkg-grid` → `MosaicPkg.Grid`
/// `mosaic-pkg-data-grid-pro` → `MosaicPkg.DataGridPro`
/// `my-thing` → `MyThing`
///
/// We split on `-`, PascalCase each segment, and join with a single `.`
/// after the `MosaicPkg` prefix (if any). Qt module names must be a
/// dotted sequence of PascalCase identifiers — `[A-Z][A-Za-z0-9]*` — which
/// is exactly what this produces given that the manifest already validated
/// the input as kebab-case.
fn qmldir_module_name(package_name: &str) -> String {
    let pascal_segments: Vec<String> = package_name
        .split('-')
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();

    // Apply the `mosaic-pkg-foo-bar` → `MosaicPkg.FooBar` convention.
    if pascal_segments.len() >= 3
        && pascal_segments[0] == "Mosaic"
        && pascal_segments[1] == "Pkg"
    {
        let rest = pascal_segments[2..].concat();
        format!("MosaicPkg.{rest}")
    } else {
        pascal_segments.concat()
    }
}

// ===========================================================================
// Small filesystem helpers
//
// We wrap `std::fs` calls so that every error converts to `BuildError::Io`
// without sprinkling `.map_err(...)` at every site. This is a small
// translation layer, not a portability layer — none of this is unsafe.
// ===========================================================================

fn read_to_string(path: &Path) -> Result<String, BuildError> {
    fs::read_to_string(path).map_err(|e| BuildError::Io(format!("read {}: {e}", path.display())))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), BuildError> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    fs::write(path, bytes).map_err(|e| BuildError::Io(format!("write {}: {e}", path.display())))
}

fn create_dir_all(path: &Path) -> Result<(), BuildError> {
    fs::create_dir_all(path)
        .map_err(|e| BuildError::Io(format!("mkdir {}: {e}", path.display())))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Fixture helpers
    // -----------------------------------------------------------------------

    /// Bare-minimum `.mil` source for a slot-less zero-emit component.
    ///
    /// We intentionally keep the fixture identical between tests so a test
    /// that fails only because of a manifest-level concern doesn't also
    /// trip mosmodel/moslayout validators.
    fn minimal_mil(component: &str) -> String {
        format!("component {component} {{ }}\n")
    }

    fn minimal_mll(component: &str) -> String {
        format!("layout {component} {{ Box [ root ] {{ }} }}\n")
    }

    fn minimal_msl(component: &str) -> String {
        format!("style {component} {{ part root {{ width: 100% ; }} }}\n")
    }

    /// Write a manifest + N components into a fresh temp dir, return the
    /// root path. This is the canonical "valid package" used by the
    /// happy-path tests.
    fn make_package(name: &str, components: &[&str]) -> TempDir {
        make_package_with(name, components, /* write_msl = */ true)
    }

    fn make_package_with(name: &str, components: &[&str], write_msl: bool) -> TempDir {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path();

        // Manifest
        let exports = components
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
description = "fixture package for tests"
license = "MIT"

[components]
exports = [{exports}]

[dependencies]

[kernel]
version = "1"
"#
        );
        fs::write(root.join("mosaic-package.toml"), manifest).unwrap();

        // Sources
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        for c in components {
            fs::write(src.join(format!("{c}.mil")), minimal_mil(c)).unwrap();
            fs::write(src.join(format!("{c}.mll")), minimal_mll(c)).unwrap();
            if write_msl {
                fs::write(src.join(format!("{c}.msl")), minimal_msl(c)).unwrap();
            }
        }

        tmp
    }

    // -----------------------------------------------------------------------
    // 1. Empty package
    // -----------------------------------------------------------------------

    #[test]
    fn empty_package_builds_with_only_index() {
        let pkg = make_package("mosaic-pkg-empty", &[]);
        let out = TempDir::new().unwrap();
        let opts = BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        };
        let result = build_package(&opts).expect("empty package should build");
        assert!(result.components_built.is_empty(), "no components expected");
        // Only the index file should be written.
        assert_eq!(result.artifacts.len(), 1, "exactly one artifact (the index)");
        assert!(result.artifacts[0].ends_with("index.ts"));
    }

    // -----------------------------------------------------------------------
    // 2-4. One component, each backend
    // -----------------------------------------------------------------------

    #[test]
    fn one_component_builds_react() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .expect("react build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let tsx = out.path().join("react").join("Grid.tsx");
        assert!(tsx.exists(), "Grid.tsx should be written");
        let body = fs::read_to_string(&tsx).unwrap();
        // The React emitter emits a `function Grid(...)` and the props type.
        assert!(body.contains("Grid"), "tsx must reference component name");
    }

    #[test]
    fn one_component_builds_swiftui() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::SwiftUI,
        })
        .expect("swiftui build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let sw = out.path().join("swiftui").join("Grid.swift");
        assert!(sw.exists(), "Grid.swift should be written");
    }

    #[test]
    fn one_component_builds_qt() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Qt,
        })
        .expect("qt build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let qml = out.path().join("qt").join("Grid.qml");
        assert!(qml.exists(), "Grid.qml should be written");
        let qmldir = out.path().join("qt").join("qmldir");
        assert!(qmldir.exists(), "qmldir should be written");
        let body = fs::read_to_string(&qmldir).unwrap();
        assert!(body.contains("Grid 1.0 Grid.qml"), "qmldir lists the component");
        assert!(body.contains("module MosaicPkg.Grid"), "module line present");
    }

    // -----------------------------------------------------------------------
    // 5-6. Unsupported backends
    // -----------------------------------------------------------------------

    #[test]
    fn webcomponent_backend_is_unsupported() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::WebComponent,
        })
        .unwrap_err();
        assert!(matches!(err, BuildError::UnsupportedBackend(Backend::WebComponent)));
    }

    #[test]
    fn html_backend_is_unsupported() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
        })
        .unwrap_err();
        assert!(matches!(err, BuildError::UnsupportedBackend(Backend::Html)));
    }

    // -----------------------------------------------------------------------
    // 7. Missing .mll
    // -----------------------------------------------------------------------

    #[test]
    fn missing_mll_returns_source_not_found() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        // Remove the .mll, leaving only .mil and .msl.
        fs::remove_file(pkg.path().join("src").join("Grid.mll")).unwrap();
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .unwrap_err();
        match err {
            BuildError::SourceNotFound { component, .. } => assert_eq!(component, "Grid"),
            other => panic!("expected SourceNotFound, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 8. Malformed .mil → PipelineError
    // -----------------------------------------------------------------------

    #[test]
    fn malformed_mil_returns_pipeline_error() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        // Overwrite the .mil with something the mosmodel grammar will refuse.
        fs::write(
            pkg.path().join("src").join("Grid.mil"),
            "this is not a valid mosmodel file !!!",
        )
        .unwrap();
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .unwrap_err();
        match err {
            BuildError::PipelineError { component, .. } => assert_eq!(component, "Grid"),
            other => panic!("expected PipelineError, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 9. Multiple components
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_components_all_build() {
        let pkg = make_package("mosaic-pkg-multi", &["Alpha", "Beta", "Gamma"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .expect("multi-component build");
        assert_eq!(result.components_built.len(), 3);
        for c in ["Alpha", "Beta", "Gamma"] {
            assert!(out.path().join("react").join(format!("{c}.tsx")).exists());
        }
    }

    // -----------------------------------------------------------------------
    // 10. Optional .msl
    // -----------------------------------------------------------------------

    #[test]
    fn missing_msl_falls_back_to_empty_style() {
        let pkg = make_package_with("mosaic-pkg-grid", &["Grid"], /* write_msl = */ false);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .expect("build without .msl");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        assert!(out.path().join("react").join("Grid.tsx").exists());
    }

    // -----------------------------------------------------------------------
    // 11. Output directory is created if missing
    // -----------------------------------------------------------------------

    #[test]
    fn output_directory_is_created_if_missing() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out_parent = TempDir::new().unwrap();
        // Use a path that does NOT exist yet — three levels deep.
        let out = out_parent.path().join("a").join("b").join("dist");
        assert!(!out.exists(), "precondition: output dir does not exist");
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.clone(),
            backend: Backend::React,
        })
        .expect("build should create the output dir");
        assert!(out.join("react").join("Grid.tsx").exists());
    }

    // -----------------------------------------------------------------------
    // 12. Index lists all components
    // -----------------------------------------------------------------------

    #[test]
    fn react_index_lists_all_components() {
        let pkg = make_package("mosaic-pkg-multi", &["Alpha", "Beta"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .unwrap();
        let body = fs::read_to_string(out.path().join("react").join("index.ts")).unwrap();
        assert!(body.contains("export * from \"./Alpha\""));
        assert!(body.contains("export * from \"./Beta\""));
    }

    #[test]
    fn qmldir_lists_all_components() {
        let pkg = make_package("mosaic-pkg-multi", &["Alpha", "Beta"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Qt,
        })
        .unwrap();
        let body = fs::read_to_string(out.path().join("qt").join("qmldir")).unwrap();
        assert!(body.contains("Alpha 1.0 Alpha.qml"));
        assert!(body.contains("Beta 1.0 Beta.qml"));
    }

    // -----------------------------------------------------------------------
    // Bonus: bad manifest path surfaces a Manifest error.
    // -----------------------------------------------------------------------

    #[test]
    fn missing_manifest_returns_manifest_error() {
        let tmp = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: tmp.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
        })
        .unwrap_err();
        assert!(matches!(err, BuildError::Manifest(_)));
    }

    // -----------------------------------------------------------------------
    // qmldir module-naming sanity checks.
    // -----------------------------------------------------------------------

    #[test]
    fn qmldir_module_name_strips_mosaic_pkg_prefix() {
        assert_eq!(qmldir_module_name("mosaic-pkg-grid"), "MosaicPkg.Grid");
        assert_eq!(
            qmldir_module_name("mosaic-pkg-data-grid-pro"),
            "MosaicPkg.DataGridPro"
        );
        assert_eq!(qmldir_module_name("my-thing"), "MyThing");
    }
}
