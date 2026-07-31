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
//! - It is not a standalone *resolver*. Cross-package layout inlining lives in
//!   `mosaic-package-resolver`; this crate coordinates that resolver during
//!   artifact builds and merges dependency package styles before backend
//!   emission.
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
//!     │       ├── inline pkg::P::C references     → resolved layout IR
//!     │       ├── mosstyle_compiler::compile      → style IR (or empty default)
//!     │       ├── merge dependency styles         → final style IR
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
//!     package_root: PathBuf::from("code/packages/mosaic/mosaic-pkg-grid"),
//!     output_root:  PathBuf::from("/tmp/mosaic-pkg-grid-dist"),
//!     backend:      Backend::React,
//!     emit_project: false,
//!     theme:        None, // or Some("light".into()) to build the light theme
//! };
//! let result = build_package(&opts).expect("package compiles");
//! for path in &result.artifacts {
//!     println!("wrote {}", path.display());
//! }
//! ```

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mosaic_package_manifest::{parse_path as parse_manifest, ManifestError, MosaicPackage};
use mosmodel_compiler::{ListInnerType, SlotDecl, SlotDefault, SlotType};

// ===========================================================================
// Public types
// ===========================================================================

/// The set of backends this crate knows how to drive.
///
/// All six UI29 §4.3 backends are wired (since this update). The HTML
/// and WebComponent backends ship as part of UI29-2's follow-up; the
/// XAML backend ships with WinUI3-compatible UserControls + per-
/// component code-behind partials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// React functional components in `.tsx` files.
    React,
    /// Electron desktop projects. Component artifacts reuse the React
    /// `.tsx` shape; project shells add Electron main/preload files.
    Electron,
    /// SwiftUI `View` structs in `.swift` files.
    SwiftUI,
    /// Qt Quick (QML) elements in `.qml` files.
    Qt,
    /// One `.js` file per component, defining a self-registering
    /// `<custom-element>` against the shadow-DOM runtime.
    WebComponent,
    /// A `.html` fragment per component. Slot values resolve at the
    /// host's template engine boundary via `{{handlebars}}` markers.
    Html,
    /// WinUI 3 / UWP. Each component emits a triple:
    /// `{Component}.xaml` (markup), `{Component}.xaml.cs`
    /// (code-behind partial), and `{Component}.Event.cs`
    /// (discriminated event union).
    Xaml,
    /// Flutter / Dart. Each component emits a single `.dart` file
    /// containing a sealed `<Component>Event` union and a
    /// `StatelessWidget` class. Drops into a Flutter `lib/`
    /// directory; the host imports it like any other Dart file.
    Flutter,
    /// Jetpack Compose / Compose Multiplatform. Each component emits a
    /// single `.kt` file containing a sealed `<Component>Event` union and
    /// a `@Composable fun <Component>(...)` entrypoint.
    Compose,
}

impl Backend {
    /// Every backend driven by the MIL/MLL/MSL package pipeline. Keeping the
    /// list here gives cross-backend acceptance tests one exhaustive source of
    /// truth; adding a new enum variant requires extending this list and the
    /// Venture browser gate together.
    pub const ALL: [Self; 9] = [
        Self::React,
        Self::Electron,
        Self::SwiftUI,
        Self::Qt,
        Self::WebComponent,
        Self::Html,
        Self::Xaml,
        Self::Flutter,
        Self::Compose,
    ];

    /// The on-disk subdirectory name beneath `output_root`.
    ///
    /// Conforms to the UI29 §4.3 layout: `dist/react/`, `dist/swiftui/`,
    /// `dist/qt/`, `dist/webcomponent/`, `dist/html/`, `dist/xaml/`,
    /// `dist/flutter/`, `dist/compose/`.
    fn dir_name(self) -> &'static str {
        match self {
            Backend::React => "react",
            Backend::Electron => "electron",
            Backend::SwiftUI => "swiftui",
            Backend::Qt => "qt",
            Backend::WebComponent => "webcomponent",
            Backend::Html => "html",
            Backend::Xaml => "xaml",
            Backend::Flutter => "flutter",
            Backend::Compose => "compose",
        }
    }

    /// The file extension for the *primary* component file. Backends
    /// that emit multiple files per component (currently only XAML —
    /// `.xaml` + `.xaml.cs` + `.Event.cs`) use the extension of the
    /// markup file as their "primary"; the secondary files are written
    /// alongside with their own extensions.
    ///
    /// All package backends now return `Some(...)` since every backend has
    /// a wired `from_pipeline`. The `Option` shape is preserved so a
    /// future hypothetical "manifest-only" backend can still slot in.
    fn component_extension(self) -> Option<&'static str> {
        match self {
            Backend::React => Some("tsx"),
            Backend::Electron => Some("tsx"),
            Backend::SwiftUI => Some("swift"),
            Backend::Qt => Some("qml"),
            Backend::WebComponent => Some("js"),
            Backend::Html => Some("html"),
            Backend::Xaml => Some("xaml"),
            Backend::Flutter => Some("dart"),
            Backend::Compose => Some("kt"),
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
    /// UI32-M: when true, also emit the per-backend project shell
    /// (Vite for React, `<!DOCTYPE>` doc for HTML, Custom Element
    /// for WebComponent, SwiftPM for SwiftUI, etc.) into
    /// `<output_root>/<backend>/` alongside the per-component
    /// artifacts. Default `false` for back-compat.
    ///
    /// v1 mounts only the FIRST component declared in
    /// `[components].exports` as the project root. Multi-component
    /// routing/tabs UI is deferred to UI32-M.1 (per UI32 spec §5
    /// open question 1). Documented as a deviation in the L8
    /// CHANGELOG.
    pub emit_project: bool,
    /// Theme selector for style (`.msl`) resolution. When `Some("light")`,
    /// each component's style is read from `<Component>.light.msl` (falling
    /// back to the bare `<Component>.msl` if no light-specific stylesheet is
    /// authored, then to any themed stylesheet as a last resort). When `None`,
    /// resolution is theme-agnostic: the bare `<Component>.msl` first, else the
    /// alphabetically-first `<Component>.*.msl` (back-compat — this is how the
    /// dark theme was implicitly selected before the theme axis existed).
    ///
    /// This is the *style* analogue of the UI30 layout `variant` axis: `variant`
    /// selects the `.mll` (desktop vs touch), `theme` selects the `.msl` (dark
    /// vs light). Before this field the packager had no theme axis at all, and
    /// authored `.light.msl` files were dead code — never emitted.
    pub theme: Option<String>,
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
    /// All six UI29 §4.3 backends are wired since v0.2; this variant
    /// remains as a future-proof guard against new `Backend::Foo`
    /// variants that forget to wire `compile_one_component`.
    UnsupportedBackend(Backend),
    /// A component or package name in the manifest contains characters
    /// that would be unsafe to interpolate into a path, an XML attribute,
    /// or a JavaScript string literal. Filenames are derived directly
    /// from the manifest, so a name like `../../etc/passwd` or
    /// `X"; rm -rf "` would escape the dist directory or inject into
    /// the generated index files. Validation runs up-front, before any
    /// I/O, so a CLI sees the friendly error before partial output
    /// hits the disk.
    UnsafeName {
        /// What we were validating (`"component"` or `"package"`).
        kind: &'static str,
        /// The offending string verbatim.
        name: String,
        /// A short explanation of what's allowed
        /// (e.g. `[A-Za-z][A-Za-z0-9_]*` for components).
        reason: &'static str,
    },
    /// A manifest-declared host asset path would escape the package root or
    /// backend output directory. Host assets are copied from package-relative
    /// paths to backend-relative paths, so absolute paths, `..`, and `.` are
    /// rejected before any filesystem write.
    UnsafePath {
        /// What path we were validating.
        kind: &'static str,
        /// The offending string verbatim.
        path: String,
        /// A short explanation of what's allowed.
        reason: &'static str,
    },
    /// The manifest's `[components].exports` listed a component name that
    /// matched no source file. (We don't error from this directly today —
    /// we error from [`SourceNotFound`] instead — but the variant exists
    /// so future cross-package-aware checks have a place to land without
    /// breaking the enum's exhaustive-match contract.)
    ///
    /// [`SourceNotFound`]: BuildError::SourceNotFound
    MissingComponent { package: String, component: String },
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
    PipelineError { component: String, error: String },
    /// A `pkg::P::C` reference in a component layout could not be resolved or
    /// inlined before backend emission.
    PackageReferenceError { component: String, error: String },
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
            BuildError::SourceNotFound {
                component,
                expected_dir,
            } => write!(
                f,
                "no '{component}.mil'/'{component}.mll' under {}",
                expected_dir.display()
            ),
            BuildError::PipelineError { component, error } => {
                write!(f, "pipeline error for component '{component}': {error}")
            }
            BuildError::PackageReferenceError { component, error } => {
                write!(
                    f,
                    "package reference error for component '{component}': {error}"
                )
            }
            BuildError::UnsafeName { kind, name, reason } => write!(
                f,
                "unsafe {kind} name '{name}': {reason} (would break path or output safety)"
            ),
            BuildError::UnsafePath { kind, path, reason } => write!(
                f,
                "unsafe {kind} path '{path}': {reason} (would break path or output safety)"
            ),
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
    // All six backends are wired since v0.2; `component_extension`
    // returns `Some(...)` for every variant. The check is kept as a
    // future-proof guard against new `Backend::Foo` variants that
    // forget to wire `compile_one_component`.
    if opts.backend.component_extension().is_none() {
        return Err(BuildError::UnsupportedBackend(opts.backend));
    }

    // ----- 2. Read the manifest --------------------------------------------
    let manifest_path = opts.package_root.join("mosaic-package.toml");
    let manifest = parse_manifest(&manifest_path)?;

    // ----- 2a. Validate names from the manifest ----------------------------
    //
    // The manifest is the threat boundary: a malicious or honestly-
    // typo'd `[package].name` or `[components].exports` entry flows
    // directly into filenames we write under `output_root`, into XML
    // attributes in the XAML props fragment, into JS string literals
    // in the WebComponent index, and into HTML comments in the HTML
    // index. Without validation, a name like `../../etc/passwd` would
    // escape the dist directory and a name like `Grid"; alert(1)//`
    // would break out of the generated `import "./Grid.js"` line.
    //
    // We require strict alphanumeric-plus-underscore for component
    // names (matches the PascalCase convention every existing example
    // uses) and strict kebab-case for package names. Anything else is
    // a hard error before any I/O happens.
    validate_package_name(&manifest.package.name)?;
    for component in &manifest.components.exports {
        validate_component_name(component)?;
    }
    // The theme selector is interpolated into a stylesheet filename and joined
    // onto `src/`, so validate it as a safe path segment before any I/O — the
    // library enforces this itself, not just the CLI (see `validate_theme_name`).
    if let Some(theme) = &opts.theme {
        validate_theme_name(theme)?;
    }

    // ----- 3. Prepare the output directory ---------------------------------
    //
    // `create_dir_all` is the friendly "mkdir -p" — it is *not* an error
    // if the directory already exists, which is the behaviour we want
    // for incremental rebuilds.
    let backend_dir = opts.output_root.join(opts.backend.dir_name());
    create_dir_all(&backend_dir)?;

    // ----- 4. Compile each component (× each variant) ----------------------
    //
    // UI30 multi-layout: for every component, discover its variants
    // by scanning `src/` for `<Component>.<variant>.mll` files, then
    // emit one primary artifact per (component, variant) pair plus any
    // backend-agnostic sidecars such as generated Lattice. The default
    // variant (bare `<Component>.mll`) emits the unsuffixed artifact
    // name `<Component>.<ext>`; named variants emit
    // `<Component>.<variant>.<ext>`.
    //
    // **Back-compat clause:** a component with only a bare
    // `<Component>.mll` (no `.touch.mll`/etc.) still produces the same
    // unsuffixed primary component artifact name; generated sidecars are
    // listed separately in `BuildResult.artifacts`.
    let src_dir = opts.package_root.join("src");
    let package_search_paths = default_package_search_paths(&opts.package_root);
    let mut artifacts = Vec::new();
    let mut components_built = Vec::new();

    for component in &manifest.components.exports {
        let variants = discover_variants(&src_dir, component)?;
        for variant in &variants {
            let component_artifacts = compile_one_component(
                component,
                variant.as_deref(),
                opts.theme.as_deref(),
                &src_dir,
                &backend_dir,
                opts.backend,
                &package_search_paths,
            )?;
            artifacts.extend(component_artifacts);
        }
        // We list the component once in `components_built` even if it
        // produced multiple variant artifacts — the index file (qmldir
        // / index.html / etc.) lists components, not artifacts, and
        // tracking per-variant entries there would mean reworking
        // every per-backend index emitter (deferred to a follow-up).
        components_built.push(component.clone());
    }

    // ----- 5. Emit the per-backend index / qmldir --------------------------
    //
    // The index file lists every component built. Empty packages still get
    // an index — it's just empty — so downstream tools don't have to
    // special-case "package with zero components".
    let index_path = emit_index_file(
        &backend_dir,
        &components_built,
        opts.backend,
        &manifest.package.name,
        &artifacts,
    )?;
    artifacts.push(index_path);

    // ----- 6. UI32-M: optional project-shell emission ----------------------
    //
    // When `opts.emit_project` is true, route through each backend's
    // `from_pipeline_with_options(emit_project: true)` to produce a
    // runnable project shell alongside the per-component artifacts.
    //
    // v1 scope (deferred to UI32-M.1):
    //   - Only the FIRST component in `manifest.components.exports`
    //     is mounted as the shell's root. Authors with multi-component
    //     packages who want a tab/route bar between components hit
    //     this limitation; the spec §5 open question 1 picks
    //     "first-export-default" as the v1 policy.
    //   - Every backend, including XAML, now flows through this same
    //     artifact-builder project-shell path.
    //
    // The shell side-files (package.json, vite.config.ts, etc.) are
    // written into `backend_dir` alongside the per-component
    // artifacts. The per-emitter banner contract (UI32 spec §3.5)
    // means a re-build overwrites them deterministically.
    if opts.emit_project {
        if let Some(first_component) = components_built.first() {
            let shell_artifacts = emit_project_shell(
                first_component,
                &src_dir,
                &backend_dir,
                opts.backend,
                &manifest.package.name,
                &package_search_paths,
                opts.theme.as_deref(),
            )?;
            artifacts.extend(shell_artifacts);
        }
        // Empty packages with emit_project: true don't emit a shell —
        // there is no component to mount. The bare index file from
        // step 5 still lands in `backend_dir`.
    }

    let host_asset_artifacts =
        install_host_assets(&manifest, opts.backend, &opts.package_root, &backend_dir)?;
    artifacts.extend(host_asset_artifacts);

    Ok(BuildResult {
        artifacts,
        components_built,
    })
}

fn install_host_assets(
    manifest: &MosaicPackage,
    backend: Backend,
    package_root: &Path,
    backend_dir: &Path,
) -> Result<Vec<PathBuf>, BuildError> {
    let backend_name = backend.dir_name();
    let mut written = Vec::new();
    for asset in &manifest.host_assets.files {
        if asset.backend != backend_name && asset.backend != "*" {
            continue;
        }

        let source_rel = safe_manifest_relative_path("host asset source", &asset.source)?;
        let target_rel = safe_manifest_relative_path("host asset target", &asset.target)?;
        let source = package_root.join(&source_rel);
        let target = backend_dir.join(&target_rel);
        let bytes = fs::read(&source)
            .map_err(|e| BuildError::Io(format!("read {}: {e}", source.display())))?;
        write_file(&target, &bytes)?;
        activate_host_asset(backend, backend_dir, &target_rel)?;
        written.push(target);
    }
    Ok(written)
}

fn activate_host_asset(
    backend: Backend,
    backend_dir: &Path,
    target_rel: &Path,
) -> Result<(), BuildError> {
    match backend {
        Backend::Html | Backend::WebComponent => activate_html_host_asset(backend_dir, target_rel),
        Backend::React => activate_react_host_asset(backend_dir, target_rel),
        _ => Ok(()),
    }
}

fn activate_html_host_asset(backend_dir: &Path, target_rel: &Path) -> Result<(), BuildError> {
    if !is_html_module_asset(target_rel) {
        return Ok(());
    }

    let index_path = backend_dir.join("index.html");
    if !index_path.exists() {
        return Ok(());
    }

    let script_line = format!(
        "  <script type=\"module\" src=\"./{}\"></script>",
        path_to_web_src(target_rel)
    );
    let mut content = read_to_string(&index_path)?;
    if content.contains(&script_line) {
        return Ok(());
    }

    let insertion_point = content.find("  <script type=\"module\" src=\"./");
    if let Some(script_at) = insertion_point {
        content.insert_str(script_at, &format!("{script_line}\n"));
        write_file(&index_path, content.as_bytes())?;
    } else if let Some(body_at) = content.find("</body>") {
        content.insert_str(body_at, &format!("{script_line}\n"));
        write_file(&index_path, content.as_bytes())?;
    }

    Ok(())
}

fn activate_react_host_asset(backend_dir: &Path, target_rel: &Path) -> Result<(), BuildError> {
    let Some(import_path) = react_host_asset_import_path(target_rel) else {
        return Ok(());
    };

    let main_path = backend_dir.join("src").join("main.tsx");
    if !main_path.exists() {
        return Ok(());
    }

    let import_line = format!("import \"{import_path}\";");
    let mut content = read_to_string(&main_path)?;
    if content.contains(&import_line) {
        return Ok(());
    }

    content.insert_str(0, &format!("{import_line}\n"));
    write_file(&main_path, content.as_bytes())?;
    Ok(())
}

fn is_html_module_asset(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("js") | Some("mjs")
    )
}

fn react_host_asset_import_path(path: &Path) -> Option<String> {
    if !is_react_module_asset(path) {
        return None;
    }

    let src_rel = path.strip_prefix("src").ok()?;
    let web_src = path_to_web_src(src_rel);
    if web_src.is_empty() {
        return None;
    }

    let import_target = web_src
        .strip_suffix(".tsx")
        .or_else(|| web_src.strip_suffix(".ts"))
        .or_else(|| web_src.strip_suffix(".jsx"))
        .or_else(|| web_src.strip_suffix(".js"))
        .unwrap_or(&web_src);
    Some(format!("./{import_target}"))
}

fn is_react_module_asset(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if file_name.ends_with(".d.ts") {
        return false;
    }

    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("ts") | Some("tsx") | Some("js") | Some("jsx") | Some("mjs")
    )
}

fn path_to_web_src(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn safe_manifest_relative_path(kind: &'static str, value: &str) -> Result<PathBuf, BuildError> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return Err(unsafe_path_err(kind, value));
    }

    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            _ => return Err(unsafe_path_err(kind, value)),
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(unsafe_path_err(kind, value));
    }

    Ok(clean)
}

fn unsafe_path_err(kind: &'static str, path: &str) -> BuildError {
    BuildError::UnsafePath {
        kind,
        path: path.to_string(),
        reason: "must be a relative path made of normal path components",
    }
}

/// UI32-M: emit the per-backend project shell for the package's first
/// component. Returns the list of side-file paths written into
/// `backend_dir`.
///
/// Re-parses the first component's `.mil`/`.mll`/`.msl` triple and
/// routes through the backend-specific `from_pipeline_with_options`
/// with `emit_project: true`. The per-emitter `ProjectFiles` struct
/// is then written to disk at fixed relative paths per UI32 spec
/// §2.2 + §3.7.
///
/// XAML is intentionally not wired here — its emitter has its own
/// `EmitOptions::emit_project` mechanism (PR #3917) that runs
/// through `mosaic-compile` directly, bypassing the artifact-builder.
/// Unifying the two paths is queued as UI32-M.1.
fn emit_project_shell(
    component: &str,
    src_dir: &Path,
    backend_dir: &Path,
    backend: Backend,
    package_name: &str,
    package_search_paths: &[PathBuf],
    theme: Option<&str>,
) -> Result<Vec<PathBuf>, BuildError> {
    // Re-read the triple. This duplicates `compile_one_component`'s
    // file-loading logic; we accept the redundancy because the shell
    // emission lives outside the per-component compile loop and we'd
    // rather not thread the parsed IRs through. The triple is small
    // (typically < 1 KiB total), so reading + parsing again is
    // cheap. The `theme` selector reaches the shell's own style the same
    // way it reaches every component's style (via `resolve_style_path`),
    // so an `--emit-project --theme light` build gets a light-styled shell.
    let mil_path = src_dir.join(format!("{component}.mil"));
    let mll_path = src_dir.join(format!("{component}.mll"));
    let msl_path = resolve_style_path(src_dir, component, theme)?;

    if !mil_path.exists() || !mll_path.exists() {
        return Err(BuildError::SourceNotFound {
            component: component.to_string(),
            expected_dir: src_dir.to_path_buf(),
        });
    }

    let mil_src = read_to_string(&mil_path)?;
    let mll_src = read_to_string(&mll_path)?;
    let msl_src = if let Some(msl_path) = msl_path {
        read_to_string(&msl_path)?
    } else {
        format!("style {component} {{ }}")
    };

    let mosmodel_out =
        mosmodel_compiler::compile(&mil_src).map_err(|errs| pipeline_err(component, &errs[0]))?;
    let mut layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    let dependency_style_parts = collect_dependency_style_parts(
        component,
        &layout_out.def,
        package_search_paths,
        theme,
        &mut Vec::new(),
        &mut HashSet::new(),
    )?;
    resolve_layout_package_references(
        component,
        &mut layout_out,
        &mosmodel_out.descriptor_json,
        package_search_paths,
    )?;
    let style_out = mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    let style_def = merge_dependency_styles(style_out.def, dependency_style_parts);

    // Per-backend dispatch. Each branch builds an EmitOptions with
    // emit_project: true, calls the appropriate from_pipeline_with_options,
    // and writes the resulting ProjectFiles side-files into backend_dir
    // at the fixed relative paths from UI32 spec §2.2.
    let mut written: Vec<PathBuf> = Vec::new();
    match backend {
        Backend::React => {
            let react_opts = mosaic_emit_react::pipeline::EmitOptions {
                emit_project: true,
                ..Default::default()
            };
            let r = mosaic_emit_react::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &react_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                let flat: [(&str, &str); 5] = [
                    ("package.json", &proj.package_json),
                    ("vite.config.ts", &proj.vite_config),
                    ("tsconfig.json", &proj.tsconfig_json),
                    ("index.html", &proj.index_html),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
                let nested = backend_dir.join("src/main.tsx");
                if let Some(parent) = nested.parent() {
                    create_dir_all(parent)?;
                }
                write_file(&nested, proj.main_tsx.as_bytes())?;
                written.push(nested);
            }
        }
        Backend::Electron => {
            let npm_name = format!("{package_name}-electron");
            let react_opts = mosaic_emit_react::pipeline::EmitOptions {
                emit_project: true,
                package_name: Some(npm_name.clone()),
                ..Default::default()
            };
            let r = mosaic_emit_react::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &react_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                let flat: [(&str, String); 7] = [
                    (
                        "package.json",
                        build_electron_package_json(&npm_name, &react_opts),
                    ),
                    ("vite.config.ts", proj.vite_config),
                    ("index.html", proj.index_html),
                    ("tsconfig.json", build_electron_renderer_tsconfig()),
                    ("tsconfig.electron.json", build_electron_main_tsconfig()),
                    ("README.md", build_electron_readme(&npm_name, component)),
                    ("src/main.tsx", proj.main_tsx),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    if let Some(parent) = p.parent() {
                        create_dir_all(parent)?;
                    }
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }

                let nested: [(&str, String); 2] = [
                    ("electron/main.ts", build_electron_main_ts(component)),
                    ("electron/preload.ts", build_electron_preload_ts()),
                ];
                for (rel, body) in nested {
                    let p = backend_dir.join(rel);
                    if let Some(parent) = p.parent() {
                        create_dir_all(parent)?;
                    }
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
            }
        }
        Backend::Html => {
            let html_opts = mosaic_emit_html::pipeline::EmitOptions { emit_project: true };
            let r = mosaic_emit_html::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &html_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                // HTML names the shell `index.html` (the bare
                // manifest-only index from step 5 is at the same
                // path, so this overwrites it — by design per UI32
                // §3.4: with --emit-project, the shell IS the index).
                let flat: [(&str, &str); 3] = [
                    ("index.html", &proj.index_html),
                    ("main.js", &proj.main_js),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
            }
        }
        Backend::WebComponent => {
            let wc_opts = mosaic_emit_webcomponent::pipeline::EmitOptions { emit_project: true };
            let r = mosaic_emit_webcomponent::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &wc_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                let flat: [(&str, &str); 3] = [
                    ("index.html", &proj.index_html),
                    ("main.js", &proj.main_js),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
            }
        }
        Backend::Flutter => {
            let fl_opts = mosaic_emit_flutter::pipeline::EmitOptions {
                emit_project: true,
                ..Default::default()
            };
            let r = mosaic_emit_flutter::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &fl_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                let flat: [(&str, &str); 2] = [
                    ("pubspec.yaml", &proj.pubspec_yaml),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
                let nested = backend_dir.join("lib/main.dart");
                if let Some(parent) = nested.parent() {
                    create_dir_all(parent)?;
                }
                write_file(&nested, proj.main_dart.as_bytes())?;
                written.push(nested);
                // Dart package imports may not escape `lib/`. Keep the
                // top-level component artifact for Mosaic package consumers,
                // and mirror it into the runnable Flutter shell so
                // `lib/main.dart` can import it as a package-local library.
                let component_source =
                    read_to_string(&backend_dir.join(format!("{component}.dart")))?;
                let component_copy = backend_dir.join(format!("lib/{component}.dart"));
                write_file(&component_copy, component_source.as_bytes())?;
                written.push(component_copy);
                let host_stub = backend_dir.join("lib/mosaic_host.dart");
                if let Some(parent) = host_stub.parent() {
                    create_dir_all(parent)?;
                }
                write_file(&host_stub, proj.mosaic_host_dart.as_bytes())?;
                written.push(host_stub);
            }
        }
        Backend::Compose => {
            let r = mosaic_emit_compose::pipeline::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            let component_source = r.output;
            let flat: [(&str, String); 3] = [
                (
                    "settings.gradle.kts",
                    build_compose_settings_gradle_kts(package_name),
                ),
                (
                    "build.gradle.kts",
                    build_compose_build_gradle_kts(package_name),
                ),
                ("README.md", build_compose_readme(package_name, component)),
            ];
            for (rel, body) in flat {
                let p = backend_dir.join(rel);
                write_file(&p, body.as_bytes())?;
                written.push(p);
            }

            let main_nested = backend_dir.join("src/main/kotlin/Main.kt");
            write_file(
                &main_nested,
                build_compose_main_kt(component, &mosmodel_out.component.slots).as_bytes(),
            )?;
            written.push(main_nested);

            let component_nested = backend_dir.join(format!("src/main/kotlin/{component}.kt"));
            write_file(&component_nested, component_source.as_bytes())?;
            written.push(component_nested);
        }
        Backend::Qt => {
            let qt_opts = mosaic_emit_qt::pipeline::EmitOptions {
                emit_project: true,
                ..Default::default()
            };
            let r = mosaic_emit_qt::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &qt_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                // Qt's qmldir shell file would conflict with the
                // step-5 qmldir (the module descriptor). The shell's
                // qmldir is the same shape as the index path — UI32
                // §3.4 says --emit-project's shell IS the qmldir.
                let flat: [(&str, &str); 4] = [
                    ("CMakeLists.txt", &proj.cmake_lists),
                    ("main.cpp", &proj.main_cpp),
                    ("qmldir", &proj.qmldir),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
            }
        }
        Backend::SwiftUI => {
            let sw_opts = mosaic_emit_swiftui::pipeline::EmitOptions {
                emit_project: true,
                ..Default::default()
            };
            let r = mosaic_emit_swiftui::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                &sw_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            let component_source = r.output.clone();
            if let Some(proj) = r.project {
                let flat: [(&str, &str); 2] = [
                    ("Package.swift", &proj.package_swift),
                    ("README.md", &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
                let nested = backend_dir.join("Sources/App/App.swift");
                if let Some(parent) = nested.parent() {
                    create_dir_all(parent)?;
                }
                write_file(&nested, proj.app_swift.as_bytes())?;
                written.push(nested);

                let component_nested = backend_dir.join(format!("Sources/App/{component}.swift"));
                write_file(&component_nested, component_source.as_bytes())?;
                written.push(component_nested);
            }
        }
        Backend::Xaml => {
            let xaml_opts = mosaic_emit_xaml::pipeline::EmitOptions {
                emit_project: true,
                ..Default::default()
            };
            let r = mosaic_emit_xaml::pipeline::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                None,
                &xaml_opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;
            if let Some(proj) = r.project {
                let flat: Vec<(String, &str)> = vec![
                    (format!("{component}.csproj"), &proj.csproj),
                    ("App.xaml".to_string(), &proj.app_xaml),
                    ("App.xaml.cs".to_string(), &proj.app_xaml_cs),
                    ("MainWindow.xaml".to_string(), &proj.main_window_xaml),
                    ("MainWindow.xaml.cs".to_string(), &proj.main_window_cs),
                    ("app.manifest".to_string(), &proj.package_manifest),
                    ("build.ps1".to_string(), &proj.build_script),
                    ("README.md".to_string(), &proj.readme),
                ];
                for (rel, body) in flat {
                    let p = backend_dir.join(rel);
                    write_file(&p, body.as_bytes())?;
                    written.push(p);
                }
            }
            for side_file in r.for_view_models.iter().chain(r.if_helpers.iter()) {
                let p = backend_dir.join(&side_file.filename);
                write_file(&p, side_file.source.as_bytes())?;
                written.push(p);
            }
        }
    }
    Ok(written)
}

fn build_electron_package_json(
    npm_name: &str,
    react_opts: &mosaic_emit_react::pipeline::EmitOptions,
) -> String {
    format!(
        "{{\n  \"//\": \"AUTO-GENERATED by mosaic-compile pkg --backend electron --emit-project. Edits will be overwritten on next emit. Fork the file (remove this comment) to customise.\",\n  \"name\": \"{}\",\n  \"private\": true,\n  \"version\": \"0.0.0\",\n  \"type\": \"module\",\n  \"main\": \"dist-electron/main.js\",\n  \"scripts\": {{\n    \"dev\": \"tsc -p tsconfig.electron.json && concurrently -k \\\"vite --host 127.0.0.1\\\" \\\"wait-on http://127.0.0.1:5173 && electron .\\\"\",\n    \"build\": \"tsc -p tsconfig.json && vite build && tsc -p tsconfig.electron.json\",\n    \"start\": \"electron .\",\n    \"preview\": \"vite preview\"\n  }},\n  \"engines\": {{\n    \"node\": \"{}\"\n  }},\n  \"dependencies\": {{\n    \"react\": \"{}\",\n    \"react-dom\": \"{}\"\n  }},\n  \"devDependencies\": {{\n    \"@types/node\": \"26.0.1\",\n    \"@types/react\": \"{}\",\n    \"@types/react-dom\": \"{}\",\n    \"@vitejs/plugin-react-swc\": \"{}\",\n    \"concurrently\": \"10.0.3\",\n    \"electron\": \"42.5.0\",\n    \"typescript\": \"{}\",\n    \"vite\": \"{}\",\n    \"wait-on\": \"9.0.10\"\n  }}\n}}\n",
        npm_name,
        react_opts.pinned_node_engines,
        react_opts.pinned_react,
        react_opts.pinned_react,
        react_opts.pinned_types_react,
        react_opts.pinned_types_react_dom,
        react_opts.pinned_vite_react_plugin,
        react_opts.pinned_typescript,
        react_opts.pinned_vite,
    )
}

fn build_electron_renderer_tsconfig() -> String {
    "{\n  \"compilerOptions\": {\n    \"target\": \"ES2020\",\n    \"useDefineForClassFields\": true,\n    \"lib\": [\"DOM\", \"DOM.Iterable\", \"ES2020\"],\n    \"allowJs\": false,\n    \"skipLibCheck\": true,\n    \"esModuleInterop\": true,\n    \"allowSyntheticDefaultImports\": true,\n    \"strict\": true,\n    \"forceConsistentCasingInFileNames\": true,\n    \"module\": \"ESNext\",\n    \"moduleResolution\": \"Bundler\",\n    \"resolveJsonModule\": true,\n    \"isolatedModules\": true,\n    \"noEmit\": true,\n    \"jsx\": \"react-jsx\"\n  },\n  \"include\": [\"*.tsx\", \"src/**/*.tsx\", \"src/**/*.ts\"]\n}\n"
        .to_string()
}

const COMPOSE_GRADLE_PLUGIN_VERSION: &str = "1.11.1";
const COMPOSE_KOTLIN_PLUGIN_VERSION: &str = "2.3.21";
const COMPOSE_DESKTOP_PACKAGE_VERSION: &str = "1.0.0";

fn build_compose_settings_gradle_kts(package_name: &str) -> String {
    format!(
        concat!(
            "// AUTO-GENERATED by mosaic-compile pkg --backend compose --emit-project. Edits will be overwritten on next emit.\n",
            "pluginManagement {{\n",
            "    repositories {{\n",
            "        google()\n",
            "        mavenCentral()\n",
            "        gradlePluginPortal()\n",
            "    }}\n",
            "}}\n\n",
            "dependencyResolutionManagement {{\n",
            "    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)\n",
            "    repositories {{\n",
            "        google()\n",
            "        mavenCentral()\n",
            "    }}\n",
            "}}\n\n",
            "rootProject.name = \"{}\"\n",
        ),
        escape_kotlin_string(package_name)
    )
}

fn build_compose_build_gradle_kts(package_name: &str) -> String {
    let app_id = compose_gradle_application_id(package_name);
    format!(
        concat!(
            "// AUTO-GENERATED by mosaic-compile pkg --backend compose --emit-project. Edits will be overwritten on next emit.\n",
            "import org.jetbrains.compose.desktop.application.dsl.TargetFormat\n\n",
            "plugins {{\n",
            "    kotlin(\"jvm\") version \"{kotlin_version}\"\n",
            "    id(\"org.jetbrains.kotlin.plugin.compose\") version \"{kotlin_version}\"\n",
            "    id(\"org.jetbrains.compose\") version \"{compose_version}\"\n",
            "}}\n\n",
            "dependencies {{\n",
            "    implementation(compose.desktop.currentOs)\n",
            "}}\n\n",
            "kotlin {{\n",
            "    jvmToolchain(21)\n",
            "}}\n\n",
            "compose.desktop {{\n",
            "    application {{\n",
            "        mainClass = \"MainKt\"\n\n",
            "        nativeDistributions {{\n",
            "            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)\n",
            "            packageName = \"{app_id}\"\n",
            "            packageVersion = \"{package_version}\"\n",
            "        }}\n",
            "    }}\n",
            "}}\n",
        ),
        kotlin_version = COMPOSE_KOTLIN_PLUGIN_VERSION,
        compose_version = COMPOSE_GRADLE_PLUGIN_VERSION,
        app_id = escape_kotlin_string(&app_id),
        package_version = COMPOSE_DESKTOP_PACKAGE_VERSION,
    )
}

fn build_compose_main_kt(component_name: &str, slots: &[SlotDecl]) -> String {
    let root = build_compose_root_invocation(component_name, slots);
    format!(
        concat!(
            "// AUTO-GENERATED by mosaic-compile pkg --backend compose --emit-project. Edits will be overwritten on next emit.\n",
            "import androidx.compose.material.MaterialTheme\n",
            "import androidx.compose.runtime.Composable\n",
            "import androidx.compose.runtime.LaunchedEffect\n",
            "import androidx.compose.runtime.getValue\n",
            "import androidx.compose.runtime.mutableStateOf\n",
            "import androidx.compose.runtime.remember\n",
            "import androidx.compose.runtime.setValue\n",
            "import androidx.compose.ui.window.Window\n",
            "import androidx.compose.ui.window.application\n\n",
            "fun main() = application {{\n",
            "    val mosaicHost = remember {{ MosaicComposeHostBridge.load() }}\n",
            "    var hostProps by remember {{ mutableStateOf<Map<String, Any?>>(emptyMap()) }}\n",
            "    fun applyMosaicResponse(response: Map<String, Any?>?) {{\n",
            "        if (response == null) return\n",
            "        val nextProps = mosaicMap(response[\"props\"])\n",
            "        if (nextProps.isNotEmpty()) {{ hostProps = nextProps }}\n",
            "        val hostIntent = mosaicMap(response[\"hostIntent\"])\n",
            "        if (hostIntent.isNotEmpty()) {{ println(\"hostIntent: $hostIntent\") }}\n",
            "        response[\"error\"]?.let {{ println(\"host error: $it\") }}\n",
            "    }}\n",
            "    LaunchedEffect(mosaicHost) {{\n",
            "        applyMosaicResponse(mosaicHost?.props())\n",
            "    }}\n",
            "    Window(onCloseRequest = ::exitApplication, title = \"{}\") {{\n",
            "        MaterialTheme {{\n",
            "{root}\n",
            "        }}\n",
            "    }}\n",
            "}}\n\n",
            "private class MosaicComposeHostBridge(private val instance: Any) {{\n",
            "    fun props(): Map<String, Any?>? = invokeMap(\"props\")\n",
            "    fun handleEvent(event: Map<String, Any?>): Map<String, Any?>? = invokeMap(\"handleEvent\", event)\n\n",
            "    private fun invokeMap(methodName: String, vararg args: Any): Map<String, Any?>? = runCatching {{\n",
            "        val method = instance.javaClass.methods.firstOrNull {{ method ->\n",
            "            method.name == methodName && method.parameterCount == args.size\n",
            "        }} ?: return@runCatching null\n",
            "        mosaicMap(method.invoke(instance, *args))\n",
            "    }}.getOrNull()\n\n",
            "    companion object {{\n",
            "        fun load(): MosaicComposeHostBridge? = runCatching {{\n",
            "            val clazz = Class.forName(\"MosaicHost\")\n",
            "            MosaicComposeHostBridge(clazz.getDeclaredConstructor().newInstance())\n",
            "        }}.getOrNull()\n",
            "    }}\n",
            "}}\n\n",
            "private fun mosaicMap(value: Any?): Map<String, Any?> {{\n",
            "    val source = value as? Map<*, *> ?: return emptyMap()\n",
            "    return source.entries.mapNotNull {{ entry ->\n",
            "        val key = entry.key as? String ?: return@mapNotNull null\n",
            "        key to entry.value\n",
            "    }}.toMap()\n",
            "}}\n\n",
            "private fun mosaicString(props: Map<String, Any?>, name: String, fallback: String): String =\n",
            "    props[name]?.toString() ?: fallback\n\n",
            "private fun mosaicDouble(props: Map<String, Any?>, name: String, fallback: Double): Double =\n",
            "    when (val value = props[name]) {{\n",
            "        is Number -> value.toDouble()\n",
            "        is String -> value.toDoubleOrNull() ?: fallback\n",
            "        else -> fallback\n",
            "    }}\n\n",
            "private fun mosaicBoolean(props: Map<String, Any?>, name: String, fallback: Boolean): Boolean =\n",
            "    when (val value = props[name]) {{\n",
            "        is Boolean -> value\n",
            "        is String -> value.equals(\"true\", ignoreCase = true)\n",
            "        else -> fallback\n",
            "    }}\n\n",
            "private fun mosaicStringList(props: Map<String, Any?>, name: String): List<String> =\n",
            "    (props[name] as? List<*>)?.map {{ it.toString() }} ?: emptyList()\n\n",
            "private fun mosaicDoubleList(props: Map<String, Any?>, name: String): List<Double> =\n",
            "    (props[name] as? List<*>)?.mapNotNull {{ value ->\n",
            "        when (value) {{\n",
            "            is Number -> value.toDouble()\n",
            "            is String -> value.toDoubleOrNull()\n",
            "            else -> null\n",
            "        }}\n",
            "    }} ?: emptyList()\n\n",
            "private fun mosaicBooleanList(props: Map<String, Any?>, name: String): List<Boolean> =\n",
            "    (props[name] as? List<*>)?.mapNotNull {{ value ->\n",
            "        when (value) {{\n",
            "            is Boolean -> value\n",
            "            is String -> value.equals(\"true\", ignoreCase = true)\n",
            "            else -> null\n",
            "        }}\n",
            "    }} ?: emptyList()\n\n",
            "@Suppress(\"UNCHECKED_CAST\")\n",
            "private fun mosaicNode(\n",
            "    props: Map<String, Any?>,\n",
            "    name: String,\n",
            "    fallback: @Composable () -> Unit,\n",
            "): @Composable () -> Unit =\n",
            "    props[name] as? (@Composable () -> Unit) ?: fallback\n",
        ),
        escape_kotlin_string(component_name),
        root = root,
    )
}

fn build_compose_root_invocation(component_name: &str, slots: &[SlotDecl]) -> String {
    let mut out = format!("            {component_name}(\n");
    for slot in slots {
        let field = to_camel_case_first_lower(&slot.name);
        let value = compose_host_value_for_slot(slot);
        writeln!(out, "                {field} = {value},").unwrap();
    }
    out.push_str("                dispatch = { event ->\n");
    out.push_str(
        "                    val response = mosaicHost?.handleEvent(event.mosaicEnvelope)\n",
    );
    out.push_str(
        "                    if (response == null) println(\"event: ${event.mosaicEnvelope}\")\n",
    );
    out.push_str("                    applyMosaicResponse(response)\n");
    out.push_str("                },\n");
    out.push_str("            )");
    out
}

fn compose_host_value_for_slot(slot: &SlotDecl) -> String {
    let slot_name = escape_kotlin_string(&slot.name);
    let fallback = sample_kotlin_value_for_slot(slot);
    match &slot.r#type {
        SlotType::Text | SlotType::Image | SlotType::Color => {
            format!("mosaicString(hostProps, \"{slot_name}\", {fallback})")
        }
        SlotType::Number => format!("mosaicDouble(hostProps, \"{slot_name}\", {fallback})"),
        SlotType::Bool => format!("mosaicBoolean(hostProps, \"{slot_name}\", {fallback})"),
        SlotType::List(inner) => match inner.as_ref() {
            ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                format!("mosaicStringList(hostProps, \"{slot_name}\")")
            }
            ListInnerType::Number => format!("mosaicDoubleList(hostProps, \"{slot_name}\")"),
            ListInnerType::Bool => format!("mosaicBooleanList(hostProps, \"{slot_name}\")"),
            _ => fallback,
        },
        SlotType::Node | SlotType::Component(_) => {
            format!("mosaicNode(hostProps, \"{slot_name}\", {fallback})")
        }
    }
}

fn sample_kotlin_value_for_slot(slot: &SlotDecl) -> String {
    match &slot.default {
        Some(SlotDefault::Text(value)) => format!("\"{}\"", escape_kotlin_string(value)),
        Some(SlotDefault::Number(value)) => kotlin_double_literal(*value),
        Some(SlotDefault::Bool(value)) => value.to_string(),
        None => sample_kotlin_value_for_slot_type(&slot.r#type, &slot.name),
    }
}

fn sample_kotlin_value_for_slot_type(slot_type: &SlotType, slot_name: &str) -> String {
    match slot_type {
        SlotType::Text => format!(
            "\"Sample {}\"",
            escape_kotlin_string(&kebab_to_pascal_case_for_label(slot_name))
        ),
        SlotType::Number => "0.0".to_string(),
        SlotType::Bool => "false".to_string(),
        SlotType::Image => "\"sample-image\"".to_string(),
        SlotType::Color => "\"#808080\"".to_string(),
        SlotType::Node => "{}".to_string(),
        SlotType::List(_) => "emptyList()".to_string(),
        SlotType::Component(name) => format!("TODO(\"Sample {}\")", escape_kotlin_string(name)),
    }
}

fn kotlin_double_literal(value: f64) -> String {
    if !value.is_finite() {
        return "0.0".to_string();
    }
    let mut out = value.to_string();
    if !out.contains('.') && !out.contains('e') && !out.contains('E') {
        out.push_str(".0");
    }
    out
}

fn compose_gradle_application_id(package_name: &str) -> String {
    package_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn to_camel_case_first_lower(name: &str) -> String {
    let mut out = String::new();
    for (idx, part) in name.split('-').enumerate() {
        if idx == 0 {
            out.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

fn kebab_to_pascal_case_for_label(name: &str) -> String {
    let mut out = String::new();
    for part in name.split('-') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

fn escape_kotlin_string(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                write!(out, "\\u{:04X}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out
}

fn build_compose_readme(package_name: &str, component: &str) -> String {
    let app_id = compose_gradle_application_id(package_name);
    format!(
        "<!-- AUTO-GENERATED by mosaic-compile pkg --backend compose --emit-project. Edits will be overwritten on next emit. -->\n\
# {component} - Compose Desktop shell\n\n\
Auto-generated by `mosaic-compile pkg --backend compose --emit-project`.\n\n\
The top-level `{component}.kt` remains the reusable Mosaic library artifact. The nested `src/main/kotlin/` copy plus Gradle files form a runnable Compose Desktop app that mounts `{component}(...)` with deterministic sample slot values unless an optional `MosaicHost` class is present, in which case generated slot props and event envelopes round-trip through that host.\n\n\
## Prerequisites\n\n\
- JDK 21 or newer.\n\
- Gradle 8.7 or newer.\n\n\
## Run\n\n\
```sh\n\
gradle run\n\
```\n\n\
## Build a native package\n\n\
```sh\n\
gradle packageDistributionForCurrentOS\n\
```\n\n\
## What's in this directory\n\n\
| File | Purpose |\n\
|---|---|\n\
| `{component}.kt` | Reusable Mosaic-compiled Compose component source. |\n\
| `index.kt` | Lightweight manifest of generated Compose components. |\n\
| `settings.gradle.kts` | Gradle settings with pinned repositories. |\n\
| `build.gradle.kts` | Compose Desktop app build pinned to Compose Multiplatform {COMPOSE_GRADLE_PLUGIN_VERSION} and Kotlin {COMPOSE_KOTLIN_PLUGIN_VERSION}. |\n\
| `src/main/kotlin/Main.kt` | Desktop app entrypoint that mounts `{component}` with sample slot values or optional `MosaicHost` props. |\n\
| `src/main/kotlin/{component}.kt` | Source-set copy of the generated component so Gradle can compile it without file moves. |\n\n\
Gradle native package name: `{app_id}`.\n"
    )
}

fn build_electron_main_tsconfig() -> String {
    "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"NodeNext\",\n    \"moduleResolution\": \"NodeNext\",\n    \"strict\": true,\n    \"esModuleInterop\": true,\n    \"skipLibCheck\": true,\n    \"types\": [\"node\"],\n    \"outDir\": \"dist-electron\",\n    \"rootDir\": \"electron\"\n  },\n  \"include\": [\"electron/**/*.ts\"]\n}\n"
        .to_string()
}

fn build_electron_main_ts(component_name: &str) -> String {
    format!(
        "// AUTO-GENERATED by mosaic-compile pkg --backend electron --emit-project. Edits will be overwritten on next emit.\n// Fork the file (remove this banner) to customise.\nimport {{ app, BrowserWindow, ipcMain }} from \"electron\";\nimport {{ existsSync }} from \"node:fs\";\nimport {{ fileURLToPath, pathToFileURL }} from \"node:url\";\nimport path from \"node:path\";\n\ntype MosaicHostRequest = {{\n  component: string;\n  event?: unknown;\n}};\n\ntype MosaicHostResponse = {{ props?: Record<string, unknown> }} | Record<string, unknown> | undefined;\ntype MosaicHost = {{\n  getProps?: (request: MosaicHostRequest) => MosaicHostResponse | Promise<MosaicHostResponse>;\n  handleEvent?: (request: MosaicHostRequest) => MosaicHostResponse | Promise<MosaicHostResponse>;\n}};\n\ntype MosaicHostModule = {{\n  default?: MosaicHost;\n  createMosaicHost?: (request: {{ component: string }}) => MosaicHost | Promise<MosaicHost>;\n}};\n\nconst MOSAIC_GET_PROPS_CHANNEL = \"mosaic:get-props\";\nconst MOSAIC_HANDLE_EVENT_CHANNEL = \"mosaic:handle-event\";\nconst __filename = fileURLToPath(import.meta.url);\nconst __dirname = path.dirname(__filename);\nconst devServerUrl = process.env.MOSAIC_ELECTRON_DEV_SERVER_URL ??\n  (process.env.npm_lifecycle_event === \"dev\" ? \"http://127.0.0.1:5173\" : undefined);\nlet mosaicHost: MosaicHost = {{}};\n\nfunction mosaicHostModuleCandidates(): string[] {{\n  const envModule = process.env.MOSAIC_ELECTRON_HOST_MODULE;\n  if (envModule) {{\n    return [path.resolve(envModule)];\n  }}\n  return [\n    path.join(__dirname, \"host.js\"),\n    path.join(__dirname, \"host.mjs\"),\n    path.join(__dirname, \"..\", \"electron\", \"host.js\"),\n    path.join(__dirname, \"..\", \"electron\", \"host.mjs\"),\n  ];\n}}\n\nasync function loadMosaicHost(): Promise<MosaicHost> {{\n  const hostModulePath = mosaicHostModuleCandidates().find(candidate => existsSync(candidate));\n  if (!hostModulePath) {{\n    return {{}};\n  }}\n  const module = (await import(pathToFileURL(hostModulePath).href)) as MosaicHostModule;\n  const created =\n    typeof module.createMosaicHost === \"function\"\n      ? await module.createMosaicHost({{ component: \"{component_name}\" }})\n      : module.default;\n  return created ?? {{}};\n}}\n\nipcMain.handle(\n  MOSAIC_GET_PROPS_CHANNEL,\n  async (_event, request: MosaicHostRequest): Promise<MosaicHostResponse> =>\n    mosaicHost.getProps?.(request),\n);\nipcMain.handle(\n  MOSAIC_HANDLE_EVENT_CHANNEL,\n  async (_event, request: MosaicHostRequest): Promise<MosaicHostResponse> =>\n    mosaicHost.handleEvent?.(request),\n);\n\nasync function createWindow(): Promise<void> {{\n  const mainWindow = new BrowserWindow({{\n    title: \"{component_name}\",\n    width: 1180,\n    height: 820,\n    minWidth: 760,\n    minHeight: 560,\n    webPreferences: {{\n      contextIsolation: true,\n      nodeIntegration: false,\n      preload: path.join(__dirname, \"preload.js\"),\n    }},\n  }});\n\n  if (devServerUrl) {{\n    await mainWindow.loadURL(devServerUrl);\n  }} else {{\n    await mainWindow.loadFile(path.join(__dirname, \"..\", \"dist\", \"index.html\"));\n  }}\n}}\n\nasync function boot(): Promise<void> {{\n  mosaicHost = await loadMosaicHost();\n  await createWindow();\n}}\n\napp.whenReady().then(() => {{\n  void boot();\n}});\n\napp.on(\"activate\", () => {{\n  if (BrowserWindow.getAllWindows().length === 0) {{\n    void createWindow();\n  }}\n}});\n\napp.on(\"window-all-closed\", () => {{\n  if (process.platform !== \"darwin\") {{\n    app.quit();\n  }}\n}});\n"
    )
}

fn build_electron_preload_ts() -> String {
    "// AUTO-GENERATED by mosaic-compile pkg --backend electron --emit-project. Edits will be overwritten on next emit.\n// Fork the file (remove this banner) to customise.\nimport { contextBridge, ipcRenderer } from \"electron\";\n\ntype MosaicHostRequest = {\n  component: string;\n  event?: unknown;\n};\n\ntype MosaicHostResponse = { props?: Record<string, unknown> } | Record<string, unknown> | undefined;\n\nconst MOSAIC_GET_PROPS_CHANNEL = \"mosaic:get-props\";\nconst MOSAIC_HANDLE_EVENT_CHANNEL = \"mosaic:handle-event\";\n\ncontextBridge.exposeInMainWorld(\"mosaicHost\", {\n  platform: \"electron\",\n  getProps: (request: MosaicHostRequest): Promise<MosaicHostResponse> =>\n    ipcRenderer.invoke(MOSAIC_GET_PROPS_CHANNEL, request),\n  handleEvent: (request: MosaicHostRequest): Promise<MosaicHostResponse> =>\n    ipcRenderer.invoke(MOSAIC_HANDLE_EVENT_CHANNEL, request),\n});\n"
        .to_string()
}

fn build_electron_readme(npm_name: &str, component_name: &str) -> String {
    format!(
        "<!-- AUTO-GENERATED by mosaic-compile pkg --backend electron --emit-project. Edits will be overwritten on next emit. -->\n<!-- Fork the file (remove this banner) to customise. -->\n# {component_name} - Electron + Vite + React shell\n\nAuto-generated by `mosaic-compile pkg --backend electron --emit-project`.\n\nThe renderer imports the same Mosaic-generated `{component_name}.tsx` artifact that the React backend emits. The Electron files are only the desktop host shell.\n\n## Host integration\n\nThe preload exposes `window.mosaicHost.getProps` and `window.mosaicHost.handleEvent` through context-isolated IPC channels. The generated main process delegates those calls to an optional host module. Add `electron/host.ts`, `electron/host.js`, or `electron/host.mjs` that exports `createMosaicHost()` or set `MOSAIC_ELECTRON_HOST_MODULE=/absolute/path/to/host.mjs`; without one, the shell keeps using renderer fallback props.\n\n## Prerequisites\n\n- Node.js >= 18.\n- npm, pnpm, or yarn.\n\n## Run\n\n```sh\nnpm install\nnpm run dev\n```\n\n`npm run dev` compiles `electron/main.ts` and `electron/preload.ts` before launching Electron, then starts the Vite renderer and waits for it to be reachable.\n\n## Build\n\n```sh\nnpm run build\nnpm start\n```\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `{component_name}.tsx` | The Mosaic-compiled renderer component. |\n| `src/main.tsx` | Mounts `<{component_name}>` into the Vite renderer root. |\n| `electron/main.ts` | Electron main process, window host, and Mosaic host IPC handlers. |\n| `electron/preload.ts` | Context-isolated bridge for `window.mosaicHost`. |\n| `package.json` | npm package manifest with pinned Electron/Vite/React dependencies. |\n\nnpm package name: `{npm_name}`.\n"
    )
}

// ===========================================================================
// Per-component pipeline
// ===========================================================================

/// Compile one component's three-file triple for the chosen backend.
///
/// Returns the paths of the written component artifacts, or a [`BuildError`] tagged
/// with the component name so a CLI can render
/// `mosaic-compile pkg: error compiling Grid: …`.
fn compile_one_component(
    component: &str,
    variant: Option<&str>,
    theme: Option<&str>,
    src_dir: &Path,
    out_dir: &Path,
    backend: Backend,
    package_search_paths: &[PathBuf],
) -> Result<Vec<PathBuf>, BuildError> {
    // ----- 1. Locate the three source files --------------------------------
    //
    // `.mil` and `.mll` are required; `.msl` is optional. Style resolution
    // honours the `theme` selector via `resolve_style_path`: a `light` build
    // prefers `<Component>.light.msl`, falling back to the bare
    // `<Component>.msl` and then any themed stylesheet. A theme-agnostic
    // (`None`) build reads the bare `.msl` first, else the alphabetically-first
    // themed stylesheet (historical dark-wins default).
    //
    // UI30 multi-layout: the `.mll` resolution honours the `variant`
    // argument. When `Some("touch")`, we look for `<Component>.touch.mll`
    // (no fallback at this layer — `discover_variants` is the source of
    // truth for which variants exist, so the file is guaranteed to be
    // there). When `None`, we read the bare `<Component>.mll` (default
    // variant).
    let mil_path = src_dir.join(format!("{component}.mil"));
    let mll_path = match variant {
        Some(v) => src_dir.join(format!("{component}.{v}.mll")),
        None => src_dir.join(format!("{component}.mll")),
    };
    let msl_path = resolve_style_path(src_dir, component, theme)?;

    if !mil_path.exists() || !mll_path.exists() {
        return Err(BuildError::SourceNotFound {
            component: component.to_string(),
            expected_dir: src_dir.to_path_buf(),
        });
    }

    let mil_src = read_to_string(&mil_path)?;
    let mll_src = read_to_string(&mll_path)?;
    let msl_src = if let Some(msl_path) = msl_path {
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
    let mosmodel_out =
        mosmodel_compiler::compile(&mil_src).map_err(|errs| pipeline_err(component, &errs[0]))?;

    let mut layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    let dependency_style_parts = collect_dependency_style_parts(
        component,
        &layout_out.def,
        package_search_paths,
        theme,
        &mut Vec::new(),
        &mut HashSet::new(),
    )?;
    resolve_layout_package_references(
        component,
        &mut layout_out,
        &mosmodel_out.descriptor_json,
        package_search_paths,
    )?;

    let style_out = mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    let style_def = merge_dependency_styles(style_out.def, dependency_style_parts);
    let lattice = mosstyle_compiler::emit_lattice(&style_def);

    // ----- 3. Hand the three IRs to the chosen backend ---------------------
    //
    // Each backend produces either:
    //   - a single string (React/SwiftUI/Qt/HTML/WebComponent) → one file
    //   - a multi-file triple (XAML) → three files written together
    //
    // For the single-file shape we write `{Component}.{ext}` and return
    // its path. For XAML we write all three, return the primary `.xaml`
    // path, and write the secondaries alongside.
    let ext = backend
        .component_extension()
        .expect("every backend has an extension since the v0.2 wire-up");

    // UI30 multi-layout output filename:
    //   default variant (variant == None) → `Grid.tsx`     (back-compat)
    //   named variant   (variant == Some) → `Grid.touch.tsx`
    //
    // Embedding the variant infix in the filename lets multiple
    // variants coexist in one output directory without collision.
    // Hosts pick which variant to import at build time (or import
    // all and pick at runtime — UI30 §6 leaves the policy to the
    // host).
    let primary_path = match variant {
        Some(v) => out_dir.join(format!("{component}.{v}.{ext}")),
        None => out_dir.join(format!("{component}.{ext}")),
    };

    let mut backend_artifacts = Vec::new();
    let primary_bytes: String = match backend {
        Backend::React | Backend::Electron => mosaic_emit_react::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::SwiftUI => mosaic_emit_swiftui::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::Qt => mosaic_emit_qt::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::Html => mosaic_emit_html::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::WebComponent => mosaic_emit_webcomponent::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::Xaml => {
            // XAML produces three files per component. We do the full
            // emit here (so the secondary writes happen alongside the
            // primary), then return the primary body string for the
            // shared single-file write at the end of the function.
            //
            // No registry / EmitOptions tweaks for v1: package-builder
            // mode treats every component as a stand-alone UserControl
            // (registry=None) and never emits the project shell
            // (EmitOptions::default()).
            let opts = mosaic_emit_xaml::pipeline::EmitOptions::default();
            let result = mosaic_emit_xaml::pipeline::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_def,
                None,
                &opts,
            )
            .map_err(|e| pipeline_emit_err(component, e))?;

            // Write the secondaries alongside the primary `.xaml`.
            // `.xaml.cs` is the code-behind partial; `.Event.cs` is
            // the discriminated event union. Variant infix applies
            // to both secondaries so a multi-variant XAML build
            // produces e.g. Grid.touch.xaml + Grid.touch.xaml.cs +
            // Grid.touch.Event.cs alongside the desktop trio.
            let (code_behind_path, events_path) = match variant {
                Some(v) => (
                    out_dir.join(format!("{component}.{v}.xaml.cs")),
                    out_dir.join(format!("{component}.{v}.Event.cs")),
                ),
                None => (
                    out_dir.join(format!("{component}.xaml.cs")),
                    out_dir.join(format!("{component}.Event.cs")),
                ),
            };
            write_file(&code_behind_path, result.code_behind.as_bytes())?;
            write_file(&events_path, result.events.as_bytes())?;
            backend_artifacts.push(code_behind_path);
            backend_artifacts.push(events_path);

            // XAML can reference generated C# support files from its markup
            // (for example a ViewModel or an IValueConverter). Package mode
            // must preserve those emitter-owned side files just as project
            // shell mode does; otherwise the packaged XAML cannot compile.
            for side_file in result
                .for_view_models
                .iter()
                .chain(result.if_helpers.iter())
            {
                let side_file_path = out_dir.join(&side_file.filename);
                write_file(&side_file_path, side_file.source.as_bytes())?;
                backend_artifacts.push(side_file_path);
            }

            result.xaml
        }
        Backend::Flutter => mosaic_emit_flutter::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
        Backend::Compose => mosaic_emit_compose::pipeline::from_pipeline(
            &mosmodel_out.component,
            &layout_out.def,
            &style_def,
        )
        .map(|r| r.output)
        .map_err(|e| pipeline_emit_err(component, e))?,
    };

    // ----- 4. Write the primary artifact and backend-agnostic style sidecar --
    write_file(&primary_path, primary_bytes.as_bytes())?;
    let mut artifacts = vec![primary_path];
    artifacts.extend(backend_artifacts);
    if !lattice.trim().is_empty() {
        let lattice_path = match variant {
            Some(v) => out_dir.join(format!("{component}.{v}.lattice")),
            None => out_dir.join(format!("{component}.lattice")),
        };
        write_file(&lattice_path, lattice.as_bytes())?;
        artifacts.push(lattice_path);
    }
    Ok(artifacts)
}

/// UI30 multi-layout — discover the layout variants present for one
/// component by scanning the package's `src/` directory.
///
/// Returns a Vec where each element is either:
///   - `None`            → the default variant (bare `<Component>.mll` exists)
///   - `Some("touch")`   → the named variant (`<Component>.touch.mll` exists)
///
/// **Filesystem is the source of truth.** UI30 §4 sketches a future
/// `[variants]` manifest section with explicit declarations + a fallback
/// chain. v1 of the artifact-builder skips that machinery and just
/// builds whatever layout files it finds — this keeps the diff scoped
/// to one crate (no manifest parser changes) and matches the principle
/// of "what's on disk is what gets shipped." The manifest declaration
/// is a follow-up PR for packages that want to *constrain* (vs.
/// enumerate) variants.
///
/// **Back-compat clause.** A component with only a bare `<Component>.mll`
/// (the existing convention for every published package today) returns
/// `vec![None]` — exactly one default-variant artifact, unchanged
/// behaviour. The variant infix is opt-in via filesystem.
///
/// **Discovery order.** We always emit the default variant FIRST when
/// present, so back-compat consumers see the unsuffixed filename land
/// at predictable timestamps; named variants follow alphabetically.
///
/// **Filename pattern accepted.** `<Component>.<variant>.mll` where
/// `<variant>` is ASCII alphanumeric / `_` / `-` and non-empty. Files
/// failing this pattern (e.g. `Grid..mll`, `Grid.foo bar.mll`) are
/// silently skipped — they'll surface later as moslayout parse
/// errors, not silent misses, because they aren't valid mosaic
/// sources anyway.
fn resolve_layout_package_references(
    component: &str,
    layout_out: &mut moslayout_compiler::CompileOutput,
    descriptor_json: &str,
    package_search_paths: &[PathBuf],
) -> Result<(), BuildError> {
    let resolver =
        mosaic_package_resolver::LayoutPackageResolver::new(package_search_paths.to_vec());
    resolver
        .resolve(&mut layout_out.def)
        .map_err(|e| BuildError::PackageReferenceError {
            component: component.to_string(),
            error: e.to_string(),
        })?;

    if let Some(tag) = mosaic_package_resolver::first_qualified_tag(&layout_out.def.root) {
        return Err(BuildError::PackageReferenceError {
            component: component.to_string(),
            error: format!("resolver left qualified tag `{tag}` in the layout"),
        });
    }

    let resolved_parts = moslayout_compiler::validate(&layout_out.def, Some(descriptor_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    layout_out.parts = resolved_parts;
    layout_out.part_map_json =
        moslayout_compiler::emit_part_map_json(&layout_out.def.component_name, &layout_out.parts);
    Ok(())
}

fn collect_dependency_style_parts(
    owner_component: &str,
    layout: &moslayout_compiler::LayoutDef,
    package_search_paths: &[PathBuf],
    theme: Option<&str>,
    visiting: &mut Vec<(String, String)>,
    collected: &mut HashSet<(String, String)>,
) -> Result<Vec<mosstyle_compiler::PartStyle>, BuildError> {
    collect_dependency_style_parts_from_node(
        owner_component,
        &layout.root,
        package_search_paths,
        theme,
        visiting,
        collected,
    )
}

fn collect_dependency_style_parts_from_node(
    owner_component: &str,
    node: &moslayout_compiler::LayoutNode,
    package_search_paths: &[PathBuf],
    theme: Option<&str>,
    visiting: &mut Vec<(String, String)>,
    collected: &mut HashSet<(String, String)>,
) -> Result<Vec<mosstyle_compiler::PartStyle>, BuildError> {
    let mut parts = Vec::new();

    if let Some((package, component)) = node.package_ref() {
        parts.extend(collect_dependency_component_style_parts(
            owner_component,
            package,
            component,
            package_search_paths,
            theme,
            visiting,
            collected,
        )?);
    }

    for child in &node.children {
        parts.extend(collect_dependency_style_parts_from_node(
            owner_component,
            child,
            package_search_paths,
            theme,
            visiting,
            collected,
        )?);
    }

    Ok(parts)
}

fn collect_dependency_component_style_parts(
    owner_component: &str,
    package: &str,
    component: &str,
    package_search_paths: &[PathBuf],
    theme: Option<&str>,
    visiting: &mut Vec<(String, String)>,
    collected: &mut HashSet<(String, String)>,
) -> Result<Vec<mosstyle_compiler::PartStyle>, BuildError> {
    let key = (package.to_string(), component.to_string());
    if collected.contains(&key) {
        return Ok(Vec::new());
    }
    if visiting.iter().any(|entry| entry == &key) {
        let mut cycle = visiting.clone();
        cycle.push(key);
        return Err(package_reference_err(
            owner_component,
            format!("circular package style reference: {cycle:?}"),
        ));
    }
    visiting.push(key.clone());

    let package_root = locate_dependency_package(owner_component, package, package_search_paths)?;
    let manifest_path = package_root.join("mosaic-package.toml");
    let manifest = parse_manifest(&manifest_path).map_err(|e| {
        package_reference_err(
            owner_component,
            format!(
                "dependency package `{package}` manifest {} failed to parse: {e}",
                manifest_path.display()
            ),
        )
    })?;
    if !manifest.components.exports.iter().any(|e| e == component) {
        return Err(package_reference_err(
            owner_component,
            format!("dependency package `{package}` does not export component `{component}`"),
        ));
    }

    let src_dir = package_root.join("src");
    let mil_path = src_dir.join(format!("{component}.mil"));
    let mll_path = src_dir.join(format!("{component}.mll"));
    let msl_path = resolve_style_path(&src_dir, component, theme)?;

    if !mil_path.exists() || !mll_path.exists() {
        return Err(package_reference_err(
            owner_component,
            format!(
                "dependency component `{package}::{component}` is missing {} or {}",
                mil_path.display(),
                mll_path.display()
            ),
        ));
    }

    let mil_src = read_to_string(&mil_path)?;
    let mll_src = read_to_string(&mll_path)?;
    let msl_src = if let Some(msl_path) = msl_path {
        read_to_string(&msl_path)?
    } else {
        format!("style {component} {{ }}")
    };

    let mosmodel_out =
        mosmodel_compiler::compile(&mil_src).map_err(|errs| pipeline_err(component, &errs[0]))?;
    let mut layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;

    let mut dependency_parts = collect_dependency_style_parts(
        owner_component,
        &layout_out.def,
        package_search_paths,
        theme,
        visiting,
        collected,
    )?;
    resolve_layout_package_references(
        component,
        &mut layout_out,
        &mosmodel_out.descriptor_json,
        package_search_paths,
    )?;

    let style_out = mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json))
        .map_err(|errs| pipeline_err(component, &errs[0]))?;
    dependency_parts.extend(style_out.def.parts);

    visiting.pop();
    collected.insert(key);
    Ok(dependency_parts)
}

fn locate_dependency_package(
    owner_component: &str,
    package: &str,
    package_search_paths: &[PathBuf],
) -> Result<PathBuf, BuildError> {
    for search_root in package_search_paths {
        let entries = match fs::read_dir(search_root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            let manifest_path = candidate.join("mosaic-package.toml");
            if !manifest_path.exists() {
                continue;
            }
            let Ok(manifest) = parse_manifest(&manifest_path) else {
                continue;
            };
            if manifest.package.name == package {
                return Ok(candidate);
            }
        }
    }

    Err(package_reference_err(
        owner_component,
        format!("dependency package `{package}` could not be found"),
    ))
}

fn merge_dependency_styles(
    mut own: mosstyle_compiler::StyleDef,
    mut dependency_parts: Vec<mosstyle_compiler::PartStyle>,
) -> mosstyle_compiler::StyleDef {
    dependency_parts.append(&mut own.parts);
    own.parts = dependency_parts;
    own
}

fn package_reference_err(component: &str, error: impl Into<String>) -> BuildError {
    BuildError::PackageReferenceError {
        component: component.to_string(),
        error: error.into(),
    }
}

/// Resolve which `.msl` stylesheet to compile for `component`, honouring the
/// optional `theme` selector.
///
/// Resolution order:
///
/// | `theme`         | preference order                                             |
/// |-----------------|--------------------------------------------------------------|
/// | `Some("light")` | `Component.light.msl` → `Component.msl` → alphabetically-first `Component.*.msl` |
/// | `None`          | `Component.msl` → alphabetically-first `Component.*.msl`      |
///
/// **Why the exact-theme file wins first.** A `light` build must pick up the
/// component's light stylesheet when one exists — the whole point of the theme
/// axis. Before this parameter, resolution was theme-blind (bare, else
/// alphabetically-first), so `Component.dark.msl` always beat
/// `Component.light.msl` and the light file was never emitted.
///
/// **Why we fall back rather than error on a missing theme file.** During the
/// migration to dual-theme sources, some components are authored dark-only. A
/// `light` build of such a component degrades to the bare/first stylesheet so
/// the build still succeeds with *some* styling rather than an unstyled or
/// failed component. Once every component ships a `.light.msl`, the exact match
/// always wins and the fallback is never taken. This mirrors the UI30 layout
/// `variant` fallback (missing `.<variant>.mll` → bare `.mll`).
fn resolve_style_path(
    src_dir: &Path,
    component: &str,
    theme: Option<&str>,
) -> Result<Option<PathBuf>, BuildError> {
    // 1. Exact theme match: `<Component>.<theme>.msl`.
    if let Some(theme) = theme {
        let themed = src_dir.join(format!("{component}.{theme}.msl"));
        if themed.exists() {
            return Ok(Some(themed));
        }
    }

    // 2. Bare, theme-neutral `<Component>.msl`.
    let default = src_dir.join(format!("{component}.msl"));
    if default.exists() {
        return Ok(Some(default));
    }

    // 3. Fallback: alphabetically-first `<Component>.*.msl`. With no bare
    // stylesheet and no exact theme match, we still emit *a* style rather
    // than nothing. Alphabetical order makes this deterministic (and, by
    // coincidence of naming, keeps `dark` winning over `light` for the
    // theme-agnostic `None` path — the historical default).
    let prefix = format!("{component}.");
    let mut themed = Vec::new();
    for entry in fs::read_dir(src_dir).map_err(|e| BuildError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| BuildError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("msl") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.starts_with(&prefix) {
            themed.push(path);
        }
    }
    themed.sort();
    Ok(themed.into_iter().next())
}

fn default_package_search_paths(package_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(parent) = package_root.parent() {
        push_existing_unique(&mut roots, parent.to_path_buf());
    }
    for ancestor in package_root.ancestors() {
        let packages = ancestor.join("packages");
        push_existing_unique(&mut roots, packages.clone());
        // The mosaic-pkg-* component-package family lives grouped under
        // code/packages/mosaic/ rather than directly under code/packages/ --
        // search there too so dependencies on it resolve without every
        // caller needing to know about the extra directory level.
        push_existing_unique(&mut roots, packages.join("mosaic"));
    }
    roots
}

fn push_existing_unique(roots: &mut Vec<PathBuf>, path: PathBuf) {
    if !path.is_dir() {
        return;
    }
    if roots.iter().any(|existing| existing == &path) {
        return;
    }
    roots.push(path);
}

fn discover_variants(src_dir: &Path, component: &str) -> Result<Vec<Option<String>>, BuildError> {
    let mut variants: Vec<Option<String>> = Vec::new();

    // Default variant: bare `<Component>.mll`.
    let bare = src_dir.join(format!("{component}.mll"));
    if bare.exists() {
        variants.push(None);
    }

    // Named variants: scan for `<Component>.<variant>.mll`. We can't
    // use a globbing crate (zero-deps policy), so iterate `read_dir`
    // and string-match the stem.
    let entries = match fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(e) => {
            return Err(BuildError::Io(format!(
                "failed to read src dir {}: {e}",
                src_dir.display()
            )));
        }
    };

    let prefix = format!("{component}.");
    let suffix = ".mll";
    let bare_name = format!("{component}.mll");
    let mut named: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(s) => s,
            None => continue, // non-UTF8 names are skipped (unlikely on src/)
        };
        // Filter: must start with `<Component>.` and end with `.mll`,
        // AND must NOT be the bare default (`<Component>.mll`) — that
        // file is handled separately by the bare-default check above.
        // Without this `name == bare_name` skip, `Grid.mll` would slip
        // through and the slice math below would underflow (the
        // "middle" would be the empty string between `Grid.` and
        // `.mll` — same 6 chars).
        if !name.starts_with(&prefix) || !name.ends_with(suffix) || name == bare_name {
            continue;
        }
        // Need enough length for a non-empty middle. The minimum valid
        // filename is something like `Grid.x.mll` (prefix `Grid.` + 1
        // middle char + `.mll`).
        if name.len() <= prefix.len() + suffix.len() {
            continue;
        }
        // Strip prefix + `.mll` suffix to recover the variant string.
        // `Grid.touch.mll` → prefix `Grid.` + middle `touch` + `.mll`.
        let middle = &name[prefix.len()..name.len() - suffix.len()];
        if middle.is_empty() {
            continue; // `Grid..mll` — degenerate, skip
        }
        // The middle must itself be a clean identifier — no nested dots
        // (e.g. `Grid.dark.theme.mll` would have middle `dark.theme`,
        // which isn't a single variant name). v1 rejects these as
        // ambiguous; the spec leaves `.<theme>.<variant>` crosses for
        // a follow-up.
        if !middle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        named.push(middle.to_string());
    }
    named.sort();
    named.dedup();
    for v in named {
        variants.push(Some(v));
    }

    // Degenerate case: a component is declared in the manifest but has
    // neither bare nor variant `.mll` files. We surface this as an
    // empty Vec; `build_package`'s for-loop becomes a no-op for that
    // component, and `compile_one_component`'s SourceNotFound check
    // would have caught it anyway via the old code path — but since
    // we now skip the call entirely, push a single None entry so the
    // old error path still fires (consistent UX with pre-UI30 builds).
    if variants.is_empty() {
        variants.push(None);
    }

    Ok(variants)
}

/// Convenience wrapper turning a backend's `PipelineEmitError` into a
/// `BuildError::PipelineError` tagged with the component name. Lifted
/// out of every backend arm so the dispatch above stays compact.
fn pipeline_emit_err<E: std::fmt::Display>(component: &str, e: E) -> BuildError {
    BuildError::PipelineError {
        component: component.to_string(),
        error: e.to_string(),
    }
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
    component_artifacts: &[PathBuf],
) -> Result<PathBuf, BuildError> {
    match backend {
        Backend::React | Backend::Electron => {
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
        Backend::Compose => {
            // Kotlin has no source-level re-export equivalent, so this
            // index is a lightweight manifest that keeps package output
            // discoverable and mirrors the other backend aggregators.
            let path = backend_dir.join("index.kt");
            let mut body = String::new();
            body.push_str("// Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            body.push_str(&format!("// Package: {package_name}\n\n"));
            for c in components {
                body.push_str(&format!("// Component: {c} (see {c}.kt)\n"));
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
        Backend::Html => {
            // `index.html` aggregates every component fragment into one
            // browsable file. Each component is wrapped in a
            // `<section data-component="X">` block so a hosting tool can
            // find and lift individual components. No JS, no styles — the
            // index is itself an HTML fragment, not a full document.
            let path = backend_dir.join("index.html");
            let mut body = String::new();
            body.push_str(
                "<!-- Auto-generated by mosaic-package-artifact-builder. Do not edit. -->\n",
            );
            body.push_str(&format!("<!-- Package: {package_name} -->\n\n"));
            for c in components {
                body.push_str(&format!("<!-- Component: {c} (see {c}.html) -->\n"));
            }
            write_file(&path, body.as_bytes())?;

            // UI31-M Phase 3 — multi-component shell.
            //
            // The bare `index.html` above is an HTML *fragment*: just
            // comments listing components. Useful for tools, useless to
            // open in a browser. Many demos (notably VC2-html, the
            // VisiCalc HTML demo) end up hand-writing a complete `<html>`
            // wrapper that inlines each component's `.html` fragment —
            // boilerplate the builder can absorb.
            //
            // The new `index-shell.html` file IS a complete document.
            // It inlines each component's emitted `.html` content inside
            // a `<section data-component="X">` block so opening it in a
            // browser shows the whole package. The bare `index.html`
            // stays untouched (back-compat) — hosting tools that already
            // consume `index.html` see no change.
            //
            // The shell is intentionally minimal: no styles, no scripts,
            // no `<head>` chrome beyond `<meta charset>` + `<title>`. A
            // demo wanting fancier framing replaces the wrapper but the
            // per-component sections still come from the builder.
            //
            // **Read-back safety.** The per-component `.html` files are
            // produced by mosaic-emit-html (this builder writes them in
            // step 4 above), so reading them back is reading our own
            // output — not user-controlled content. The HTML escape
            // story therefore lives in mosaic-emit-html; nothing extra
            // is needed here.
            let shell_path = backend_dir.join("index-shell.html");
            let mut shell = String::new();
            shell.push_str("<!DOCTYPE html>\n");
            shell.push_str(
                "<!-- Auto-generated by mosaic-package-artifact-builder. Do not edit. -->\n",
            );
            shell.push_str("<html>\n");
            shell.push_str("<head>\n");
            shell.push_str("  <meta charset=\"utf-8\">\n");
            shell.push_str(&format!("  <title>{package_name}</title>\n"));
            shell.push_str("</head>\n");
            shell.push_str("<body>\n");
            for c in components {
                shell.push_str(&format!("  <section data-component=\"{c}\">\n"));
                let frag_path = backend_dir.join(format!("{c}.html"));
                match read_to_string(&frag_path) {
                    Ok(frag) => {
                        // Indent each line of the fragment by 4 spaces
                        // so the shell stays human-readable. Trailing
                        // newlines in the fragment fold naturally into
                        // the section's closing `</section>`.
                        for line in frag.lines() {
                            shell.push_str("    ");
                            shell.push_str(line);
                            shell.push('\n');
                        }
                    }
                    Err(_) => {
                        // Defensive: if a per-component fragment is
                        // missing (shouldn't happen — we just wrote
                        // it), emit a comment so the shell is still
                        // valid HTML and the missing piece is visible
                        // for diagnosis.
                        shell.push_str(&format!("    <!-- {c}.html missing -->\n"));
                    }
                }
                shell.push_str("  </section>\n");
            }
            shell.push_str("</body>\n");
            shell.push_str("</html>\n");
            write_file(&shell_path, shell.as_bytes())?;

            Ok(path)
        }
        Backend::WebComponent => {
            // `index.js` re-exports each component's registration. The
            // host imports this single file and every component
            // self-registers as a `<mosaic-{name}>` custom element on
            // module load. We use bare `import "./X.js"` (not
            // `export *`) because the per-component file's side effect
            // is the `customElements.define(...)` call — there's no
            // named export to forward.
            let path = backend_dir.join("index.js");
            let mut body = String::new();
            body.push_str("// Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            body.push_str(&format!("// Package: {package_name}\n\n"));
            for c in components {
                body.push_str(&format!("import \"./{c}.js\";\n"));
            }
            write_file(&path, body.as_bytes())?;
            Ok(path)
        }
        Backend::Xaml => {
            // XAML packages don't have a single "index" notion in the
            // WinUI 3 world — hosts reference per-component XAML files
            // and per-component code-behind partials individually. We
            // emit a `MosaicPackage.props` MSBuild fragment that a
            // host's `.csproj` can `<Import Project="..."/>` to pull
            // every component's `.xaml` + `.xaml.cs` + `.Event.cs` into
            // the build in one line.
            //
            // The format is the standard MSBuild item-group shape:
            //
            //   <Project xmlns="http://schemas.microsoft.com/...">
            //     <ItemGroup>
            //       <Page Include="Grid.xaml"><Generator>MSBuild:...</Generator></Page>
            //       <Compile Include="Grid.xaml.cs"><DependentUpon>Grid.xaml</DependentUpon></Compile>
            //       <Compile Include="Grid.Event.cs"/>
            //     </ItemGroup>
            //   </Project>
            //
            // Authoring this by hand for every package is error-prone;
            // generating it gets the dependent-upon wiring right every
            // time.
            let path = backend_dir.join("MosaicPackage.props");
            let mut body = String::new();
            body.push_str(
                "<!-- Auto-generated by mosaic-package-artifact-builder. Do not edit. -->\n",
            );
            body.push_str(&format!("<!-- Package: {package_name} -->\n"));
            body.push_str(
                "<Project xmlns=\"http://schemas.microsoft.com/developer/msbuild/2003\">\n",
            );
            body.push_str("  <ItemGroup>\n");
            for c in components {
                body.push_str(&format!(
                    "    <Page Include=\"{c}.xaml\"><Generator>MSBuild:Compile</Generator><SubType>Designer</SubType></Page>\n"
                ));
                body.push_str(&format!(
                    "    <Compile Include=\"{c}.xaml.cs\"><DependentUpon>{c}.xaml</DependentUpon></Compile>\n"
                ));
                body.push_str(&format!("    <Compile Include=\"{c}.Event.cs\"/>\n"));
            }
            let mut support_files = component_artifacts
                .iter()
                .filter_map(|artifact| artifact.file_name().and_then(|name| name.to_str()))
                .filter(|name| {
                    name.ends_with(".cs")
                        && !name.ends_with(".xaml.cs")
                        && !name.ends_with(".Event.cs")
                })
                .collect::<Vec<_>>();
            support_files.sort_unstable();
            support_files.dedup();
            for support_file in support_files {
                body.push_str(&format!("    <Compile Include=\"{support_file}\"/>\n"));
            }
            body.push_str("  </ItemGroup>\n");
            body.push_str("</Project>\n");
            write_file(&path, body.as_bytes())?;
            Ok(path)
        }
        Backend::Flutter => {
            // Flutter packages aggregate via a barrel `index.dart`
            // that re-exports every component file, paired with a
            // minimal `pubspec.yaml` so `flutter pub get` knows the
            // package is a Flutter library and pulls the
            // `flutter/material.dart` dependency. The barrel is the
            // first thing a host imports:
            //
            //     import 'package:mosaic_pkg_grid/index.dart';
            //
            // and `pubspec.yaml` gives the package its name +
            // dependency graph.
            //
            // We emit both files; the returned path is the
            // `index.dart` (matching the other backends' "index is
            // the primary aggregator" convention). The `pubspec.yaml`
            // is a secondary alongside it.
            let index_path = backend_dir.join("index.dart");
            let mut idx = String::new();
            idx.push_str("// Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            idx.push_str(&format!("// Package: {package_name}\n\n"));
            for c in components {
                idx.push_str(&format!("export '{c}.dart';\n"));
            }
            write_file(&index_path, idx.as_bytes())?;

            // Dart package names are snake_case — kebab → snake.
            let dart_pkg = package_name.replace('-', "_");
            let pubspec_path = backend_dir.join("pubspec.yaml");
            let mut pubspec = String::new();
            pubspec.push_str("# Auto-generated by mosaic-package-artifact-builder. Do not edit.\n");
            pubspec.push_str(&format!("name: {dart_pkg}\n"));
            pubspec.push_str("description: Mosaic-generated Flutter package.\n");
            pubspec.push_str("version: 0.0.0\n");
            pubspec.push_str("environment:\n");
            pubspec.push_str("  sdk: ^3.0.0\n");
            pubspec.push_str("dependencies:\n");
            pubspec.push_str("  flutter:\n");
            pubspec.push_str("    sdk: flutter\n");
            write_file(&pubspec_path, pubspec.as_bytes())?;

            Ok(index_path)
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
    if pascal_segments.len() >= 3 && pascal_segments[0] == "Mosaic" && pascal_segments[1] == "Pkg" {
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
    fs::create_dir_all(path).map_err(|e| BuildError::Io(format!("mkdir {}: {e}", path.display())))
}

// ===========================================================================
// Name validation (security boundary — see `build_package` step 2a)
// ===========================================================================

/// Validate a component name from the manifest's `[components].exports`.
///
/// Required shape: `[A-Za-z][A-Za-z0-9_]*` — strict alphanumeric-plus-
/// underscore, must lead with a letter. This is intentionally
/// stricter than what most file systems would accept; component names
/// flow into:
///
/// - Filenames: `out_dir.join(format!("{component}.{ext}"))`. Without
///   validation, `../../etc/passwd` would escape the dist root.
/// - XAML's MSBuild props XML: `<Page Include="{component}.xaml">`.
///   Without validation, `Grid"><Exec Command="rm -rf /"/><Page Include="`
///   would inject an MSBuild task.
/// - WebComponent's index JS: `import "./{component}.js";`. Without
///   validation, `Grid"; fetch(...)//`would break out of the string.
/// - HTML's index comments: `<!-- Component: {component} -->`. Without
///   validation, `Grid --><script>alert(1)</script><!--` would inject
///   into the aggregated index.
///
/// Every existing component in the codebase follows PascalCase
/// (`Grid`, `Cell`, `Button`, `HostInput`, `MosaicPkgDialog`, …)
/// which trivially passes this filter; the validation is purely a
/// hardening pass against malicious or accidentally-malformed
/// manifests.
fn validate_component_name(name: &str) -> Result<(), BuildError> {
    let unsafe_err = || BuildError::UnsafeName {
        kind: "component",
        name: name.to_string(),
        reason: "must match [A-Za-z][A-Za-z0-9_]* (PascalCase recommended)",
    };

    let mut chars = name.chars();
    let first = chars.next().ok_or_else(unsafe_err)?;
    if !first.is_ascii_alphabetic() {
        return Err(unsafe_err());
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(unsafe_err());
        }
    }
    Ok(())
}

/// Validate a package name from the manifest's `[package].name`.
///
/// Required shape: `[a-z][a-z0-9-]*` — strict lowercase-kebab-case,
/// must lead with a letter. Package names flow into the generated
/// index files' "Package: NAME" comments (HTML/JS/XML) and into the
/// Qt qmldir's `module NAME` line. Same threat model as component
/// names, plus the Qt `qmldir_module_name` helper relies on kebab-case
/// to produce a valid `[A-Z][A-Za-z0-9]*` Qt module name.
///
/// Every existing `mosaic-pkg-*` package follows this convention; the
/// validation is purely a hardening pass.
fn validate_package_name(name: &str) -> Result<(), BuildError> {
    let unsafe_err = || BuildError::UnsafeName {
        kind: "package",
        name: name.to_string(),
        reason: "must match [a-z][a-z0-9-]* (kebab-case)",
    };

    let mut chars = name.chars();
    let first = chars.next().ok_or_else(unsafe_err)?;
    if !first.is_ascii_lowercase() {
        return Err(unsafe_err());
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(unsafe_err());
        }
    }
    Ok(())
}

/// Validate the `theme` selector from [`BuildOptions::theme`].
///
/// Required shape: non-empty ASCII alphanumeric / `_` / `-`. The theme string
/// is interpolated into a filename (`<Component>.<theme>.msl`) that is joined
/// onto the package `src/` directory, so — exactly like `variant` — it must be
/// a single safe path segment. Rejecting `/`, `\`, `.`, `..`, and null bytes
/// keeps a hostile or typo'd theme from escaping `src/` and coaxing the
/// compiler into reading an arbitrary `.msl`-suffixed file.
///
/// This is enforced HERE, in the library, rather than only in the
/// `mosaic-compile` CLI shell: `build_package` is a public entry point that
/// downstream tooling (IDE plugins, test harnesses) calls directly, so the
/// library must not trust its caller to have pre-validated the theme — the same
/// reason `component`/`package` names are validated in `build_package` above.
fn validate_theme_name(name: &str) -> Result<(), BuildError> {
    let unsafe_err = || BuildError::UnsafeName {
        kind: "theme",
        name: name.to_string(),
        reason: "must be non-empty ASCII alphanumeric / _ / - (a single path segment)",
    };
    if name.is_empty() {
        return Err(unsafe_err());
    }
    for c in name.chars() {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(unsafe_err());
        }
    }
    Ok(())
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

    fn write_package_manifest(
        root: &Path,
        name: &str,
        components: &[&str],
        dependencies: &[(&str, &str)],
    ) {
        let exports = components
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let deps = dependencies
            .iter()
            .map(|(name, version)| format!("{name} = \"{version}\""))
            .collect::<Vec<_>>()
            .join("\n");
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
{deps}

[kernel]
version = "1"
"#
        );
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("mosaic-package.toml"), manifest).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
    }

    fn write_component_sources(root: &Path, component: &str, mil: &str, mll: &str, msl: &str) {
        let src = root.join("src");
        fs::write(src.join(format!("{component}.mil")), mil).unwrap();
        fs::write(src.join(format!("{component}.mll")), mll).unwrap();
        fs::write(src.join(format!("{component}.msl")), msl).unwrap();
    }

    fn append_host_assets(root: &Path, toml: &str) {
        let manifest_path = root.join("mosaic-package.toml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let manifest = manifest.replace("[kernel]", &format!("{toml}\n[kernel]"));
        fs::write(manifest_path, manifest).unwrap();
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
            emit_project: false,
            theme: None,
        };
        let result = build_package(&opts).expect("empty package should build");
        assert!(result.components_built.is_empty(), "no components expected");
        // Only the index file should be written.
        assert_eq!(
            result.artifacts.len(),
            1,
            "exactly one artifact (the index)"
        );
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
            emit_project: false,
            theme: None,
        })
        .expect("react build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let tsx = out.path().join("react").join("Grid.tsx");
        assert!(tsx.exists(), "Grid.tsx should be written");
        let body = fs::read_to_string(&tsx).unwrap();
        // The React emitter emits a `function Grid(...)` and the props type.
        assert!(body.contains("Grid"), "tsx must reference component name");
        let lattice = out.path().join("react").join("Grid.lattice");
        assert!(lattice.exists(), "Grid.lattice should be written");
        let lattice_body = fs::read_to_string(&lattice).unwrap();
        assert!(
            lattice_body.contains("Generated as Lattice")
                && lattice_body.contains(".mos-Grid-root")
                && lattice_body.contains("width: 100%"),
            "Lattice sidecar should contain scoped component styles"
        );
        assert!(
            result.artifacts.iter().any(|path| path == &lattice),
            "Lattice sidecar should appear in BuildResult.artifacts"
        );
    }

    #[test]
    fn manifest_host_assets_are_copied_for_matching_backend() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let host_dir = pkg.path().join("host").join("web");
        fs::create_dir_all(&host_dir).unwrap();
        fs::write(
            host_dir.join("grid-host.ts"),
            "export const gridHost = true;\n",
        )
        .unwrap();
        append_host_assets(
            pkg.path(),
            r#"[host_assets]
files = [
  { backend = "react", source = "host/web/grid-host.ts", target = "src/grid-host.ts" },
  { backend = "qt", source = "host/web/grid-host.ts", target = "grid-host.ts" },
]"#,
        );

        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: true,
            theme: None,
        })
        .expect("react build");

        let installed = out.path().join("react").join("src").join("grid-host.ts");
        assert_eq!(
            fs::read_to_string(&installed).unwrap(),
            "export const gridHost = true;\n"
        );
        assert!(
            result.artifacts.iter().any(|path| path == &installed),
            "copied host asset should appear in BuildResult.artifacts"
        );
        assert!(
            !out.path().join("react").join("grid-host.ts").exists(),
            "qt-only asset must not be copied for react builds"
        );
        let main = fs::read_to_string(out.path().join("react").join("src").join("main.tsx"))
            .expect("react src/main.tsx");
        assert!(
            main.contains("import \"./grid-host\";"),
            "react project shell should activate copied host module"
        );
    }

    #[test]
    fn manifest_host_assets_activate_html_modules() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let host_dir = pkg.path().join("host").join("web");
        fs::create_dir_all(&host_dir).unwrap();
        fs::write(host_dir.join("grid-host.mjs"), "window.gridHost = true;\n").unwrap();
        append_host_assets(
            pkg.path(),
            r#"[host_assets]
files = [
  { backend = "html", source = "host/web/grid-host.mjs", target = "grid-host.mjs" },
]"#,
        );

        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
            emit_project: true,
            theme: None,
        })
        .expect("html build");

        let installed = out.path().join("html").join("grid-host.mjs");
        assert_eq!(
            fs::read_to_string(&installed).unwrap(),
            "window.gridHost = true;\n"
        );
        assert!(
            result.artifacts.iter().any(|path| path == &installed),
            "copied host asset should appear in BuildResult.artifacts"
        );
        let index =
            fs::read_to_string(out.path().join("html").join("index.html")).expect("index.html");
        let host_at = index
            .find("src=\"./grid-host.mjs\"")
            .expect("html shell should activate copied host module");
        let main_at = index
            .find("src=\"./main.js\"")
            .expect("html shell should load main.js");
        assert!(
            host_at < main_at,
            "host module should load before generated main.js"
        );
    }

    #[test]
    fn manifest_host_assets_reject_escaping_targets() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let host_dir = pkg.path().join("host");
        fs::create_dir_all(&host_dir).unwrap();
        fs::write(host_dir.join("grid-host.ts"), "export {};\n").unwrap();
        append_host_assets(
            pkg.path(),
            r#"[host_assets]
files = [
  { backend = "react", source = "host/grid-host.ts", target = "../grid-host.ts" },
]"#,
        );

        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .unwrap_err();
        assert!(
            matches!(
                err,
                BuildError::UnsafePath {
                    kind: "host asset target",
                    ..
                }
            ),
            "expected UnsafePath(host asset target), got {err:?}"
        );
    }

    #[test]
    fn one_component_builds_swiftui() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::SwiftUI,
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
        })
        .expect("qt build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let qml = out.path().join("qt").join("Grid.qml");
        assert!(qml.exists(), "Grid.qml should be written");
        let qmldir = out.path().join("qt").join("qmldir");
        assert!(qmldir.exists(), "qmldir should be written");
        let body = fs::read_to_string(&qmldir).unwrap();
        assert!(
            body.contains("Grid 1.0 Grid.qml"),
            "qmldir lists the component"
        );
        assert!(
            body.contains("module MosaicPkg.Grid"),
            "module line present"
        );
    }

    // -----------------------------------------------------------------------
    // 5-7. Newly-wired backends (HTML, WebComponent, XAML). UI29-2 follow-up.
    // -----------------------------------------------------------------------

    /// HTML backend: writes `{Component}.html` per component plus a
    /// fragment-shaped `index.html` aggregator. Static-HTML fragments
    /// hold `{{slot}}` template markers a host's template engine
    /// substitutes at render time.
    #[test]
    fn html_backend_writes_html_fragment_per_component() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
            emit_project: false,
            theme: None,
        })
        .expect("html build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        assert!(out.path().join("html").join("Grid.html").exists());
        let idx = out.path().join("html").join("index.html");
        assert!(idx.exists());
        let body = fs::read_to_string(&idx).unwrap();
        assert!(body.contains("Component: Grid"));
        assert!(body.contains("Package: mosaic-pkg-grid"));
    }

    /// UI31-M Phase 3 — multi-component shell.
    ///
    /// In addition to the bare `index.html` (a fragment of component-
    /// listing comments), the HTML backend now writes
    /// `index-shell.html`: a complete `<html><body>` document that
    /// inlines each component's emitted `.html` content inside a
    /// `<section data-component="X">` block. This eats the demo-side
    /// boilerplate that today hand-writes the wrapper + the
    /// `<section>` blocks (see VC2-html's `index.html`).
    ///
    /// Verifies:
    ///   - The new file exists (`html/index-shell.html`).
    ///   - It carries a `<!DOCTYPE html>` (i.e. is a complete document,
    ///     not a fragment).
    ///   - It contains `<section data-component="Grid">` (the per-
    ///     component mount block).
    ///   - The package name shows up in the `<title>`.
    ///   - The original bare `index.html` is unchanged (back-compat —
    ///     no tool relying on it sees a regression).
    #[test]
    fn html_backend_writes_multi_component_index_shell_in_addition_to_bare_index() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
            emit_project: false,
            theme: None,
        })
        .expect("html build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);

        let shell_path = out.path().join("html").join("index-shell.html");
        assert!(shell_path.exists(), "index-shell.html should be written");
        let shell = fs::read_to_string(&shell_path).unwrap();
        assert!(
            shell.starts_with("<!DOCTYPE html>"),
            "index-shell.html must be a complete document, got:\n{shell}"
        );
        assert!(
            shell.contains("<section data-component=\"Grid\">"),
            "expected <section data-component=\"Grid\">, got:\n{shell}"
        );
        assert!(
            shell.contains("<title>mosaic-pkg-grid</title>"),
            "expected package name in <title>, got:\n{shell}"
        );

        // Back-compat: the bare index.html still carries comments
        // only (no DOCTYPE, no <section>). Hosting tools that already
        // parse it as a fragment must continue to work.
        let bare = fs::read_to_string(out.path().join("html").join("index.html")).unwrap();
        assert!(
            !bare.contains("<!DOCTYPE"),
            "bare index.html must remain a fragment, got:\n{bare}"
        );
        assert!(
            !bare.contains("<section"),
            "bare index.html must not contain section blocks, got:\n{bare}"
        );
    }

    /// WebComponent backend: writes one `.js` per component (each
    /// self-registers a `<custom-element>` on import) plus an
    /// `index.js` that imports each one in turn.
    #[test]
    fn webcomponent_backend_writes_js_per_component() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::WebComponent,
            emit_project: false,
            theme: None,
        })
        .expect("webcomponent build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        assert!(out.path().join("webcomponent").join("Grid.js").exists());
        let idx = out.path().join("webcomponent").join("index.js");
        assert!(idx.exists());
        let body = fs::read_to_string(&idx).unwrap();
        assert!(body.contains("import \"./Grid.js\""));
    }

    /// XAML backend: writes the three-file triple per component
    /// (`.xaml` + `.xaml.cs` + `.Event.cs`) plus a
    /// `MosaicPackage.props` MSBuild fragment that wires every
    /// component into a host's `.csproj` via a single `<Import>`.
    #[test]
    fn xaml_backend_writes_triple_per_component_and_props_fragment() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Xaml,
            emit_project: false,
            theme: None,
        })
        .expect("xaml build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let xaml_dir = out.path().join("xaml");
        assert!(xaml_dir.join("Grid.xaml").exists(), "primary .xaml present");
        assert!(
            xaml_dir.join("Grid.xaml.cs").exists(),
            "code-behind .xaml.cs present"
        );
        assert!(
            xaml_dir.join("Grid.Event.cs").exists(),
            "event union .Event.cs present"
        );
        let props_path = xaml_dir.join("MosaicPackage.props");
        assert!(props_path.exists(), "MSBuild fragment present");
        let props = fs::read_to_string(&props_path).unwrap();
        assert!(props.contains("<Page Include=\"Grid.xaml\""));
        assert!(props.contains("<Compile Include=\"Grid.xaml.cs\""));
        assert!(props.contains("DependentUpon>Grid.xaml<"));
        assert!(props.contains("<Compile Include=\"Grid.Event.cs\""));
    }

    #[test]
    fn xaml_package_pipeline_writes_native_focus_converter_side_file() {
        let pkg = make_package("mosaic-pkg-focus", &["FocusField"]);
        write_component_sources(
            pkg.path(),
            "FocusField",
            "component FocusField { }\n",
            r#"
layout FocusField {
  HostInput [ field ] ( placeholder : "Search" )
}
"#,
            r##"
style FocusField {
  part field {
    border-color : "#d0d0d0" ;
    state focused {
      border-color : "#e0942a" ;
    }
  }
}
"##,
        );
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Xaml,
            emit_project: false,
            theme: None,
        })
        .expect("xaml focus package build");

        let xaml_dir = out.path().join("xaml");
        let xaml = fs::read_to_string(xaml_dir.join("FocusField.xaml")).unwrap();
        assert!(
            xaml.contains(
                "Binding FocusState, ElementName=Field, Converter={StaticResource FocusStateToBoolConverter}"
            ),
            "package output must preserve native focus activation:\n{xaml}"
        );
        let converter_path = xaml_dir.join("FocusStateToBoolConverter.cs");
        assert!(
            converter_path.exists(),
            "package pipeline must write the converter referenced by XAML"
        );
        let converter = fs::read_to_string(&converter_path).unwrap();
        assert!(converter.contains("state != FocusState.Unfocused"));
        let props = fs::read_to_string(xaml_dir.join("MosaicPackage.props")).unwrap();
        assert!(
            props.contains("<Compile Include=\"FocusStateToBoolConverter.cs\"/>"),
            "package import must compile the converter referenced by XAML:\n{props}"
        );
        assert!(
            result
                .artifacts
                .iter()
                .any(|artifact| artifact == &converter_path),
            "generated converter must be reported as a package artifact"
        );
    }

    /// Flutter backend: writes one `.dart` file per component, an
    /// `index.dart` barrel re-exporting each, and a minimal
    /// `pubspec.yaml` so `flutter pub get` recognises the package.
    /// Package name is kebab-to-snake-cased for Dart's package-name
    /// convention.
    #[test]
    fn flutter_backend_writes_dart_per_component_with_pubspec() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Flutter,
            emit_project: false,
            theme: None,
        })
        .expect("flutter build");
        assert_eq!(result.components_built, vec!["Grid".to_string()]);
        let flutter_dir = out.path().join("flutter");
        assert!(
            flutter_dir.join("Grid.dart").exists(),
            "primary .dart present"
        );
        let index_path = flutter_dir.join("index.dart");
        assert!(index_path.exists(), "index.dart barrel present");
        let idx = fs::read_to_string(&index_path).unwrap();
        assert!(idx.contains("export 'Grid.dart';"));
        let pubspec_path = flutter_dir.join("pubspec.yaml");
        assert!(pubspec_path.exists(), "pubspec.yaml present");
        let pubspec = fs::read_to_string(&pubspec_path).unwrap();
        assert!(
            pubspec.contains("name: mosaic_pkg_grid"),
            "package name must be kebab→snake cased, got:\n{pubspec}"
        );
        assert!(pubspec.contains("sdk: flutter"));
    }

    #[test]
    fn dependency_component_styles_are_merged_into_parent_html_artifact() {
        let workspace = TempDir::new().unwrap();
        let child = workspace.path().join("mosaic-pkg-accent");
        let parent = workspace.path().join("mosaic-pkg-shell");

        write_package_manifest(&child, "mosaic-pkg-accent", &["Accent"], &[]);
        write_component_sources(
            &child,
            "Accent",
            r#"component Accent { slot label : text ; }"#,
            r#"layout Accent {
  Box [ accent-panel ] {
    Text [ accent-label ] ( content : slot: label )
  }
}"#,
            r##"style Accent {
  part accent-panel { background : "#123456" ; }
  part accent-label { color : "#abcdef" ; }
}"##,
        );

        write_package_manifest(
            &parent,
            "mosaic-pkg-shell",
            &["Shell"],
            &[("mosaic-pkg-accent", "0.1.0")],
        );
        write_component_sources(
            &parent,
            "Shell",
            r#"component Shell { slot label : text ; }"#,
            r#"layout Shell {
  Column [ shell-root ] {
    pkg::mosaic-pkg-accent::Accent (
      label : slot: label
    )
  }
}"#,
            r#"style Shell {
  part shell-root { padding : 8 ; }
}"#,
        );

        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: parent,
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
            emit_project: false,
            theme: None,
        })
        .expect("parent package should build with dependency styles");

        let html = fs::read_to_string(out.path().join("html").join("Shell.html")).unwrap();
        assert!(
            html.contains("#123456"),
            "dependency background style should be present in emitted HTML:\n{html}"
        );
        assert!(
            html.contains("#abcdef"),
            "dependency text style should be present in emitted HTML:\n{html}"
        );
    }

    #[test]
    fn parent_style_overrides_dependency_style_for_same_part_name() {
        let workspace = TempDir::new().unwrap();
        let child = workspace.path().join("mosaic-pkg-accent");
        let parent = workspace.path().join("mosaic-pkg-shell");

        write_package_manifest(&child, "mosaic-pkg-accent", &["Accent"], &[]);
        write_component_sources(
            &child,
            "Accent",
            r#"component Accent { slot label : text ; }"#,
            r#"layout Accent {
  Box [ accent-panel ] {
    Text [ accent-label ] ( content : slot: label )
  }
}"#,
            r##"style Accent {
  part accent-panel { background : "#123456" ; }
  part accent-label { color : "#abcdef" ; }
}"##,
        );

        write_package_manifest(
            &parent,
            "mosaic-pkg-shell",
            &["Shell"],
            &[("mosaic-pkg-accent", "0.1.0")],
        );
        write_component_sources(
            &parent,
            "Shell",
            r#"component Shell { slot label : text ; }"#,
            r#"layout Shell {
  Column [ shell-root ] {
    pkg::mosaic-pkg-accent::Accent (
      label : slot: label
    )
  }
}"#,
            r##"style Shell {
  part shell-root { padding : 8 ; }
  part accent-panel { background : "#654321" ; }
}"##,
        );

        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: parent,
            output_root: out.path().to_path_buf(),
            backend: Backend::Html,
            emit_project: false,
            theme: None,
        })
        .expect("parent package should build with a dependency style override");

        let html = fs::read_to_string(out.path().join("html").join("Shell.html")).unwrap();
        assert!(
            html.contains("#654321"),
            "parent override should be present in emitted HTML:\n{html}"
        );
        assert!(
            !html.contains("#123456"),
            "dependency style for the same part should be overridden by the parent:\n{html}"
        );
        assert!(
            html.contains("#abcdef"),
            "non-overridden dependency part styles should still be present:\n{html}"
        );
    }

    // -----------------------------------------------------------------------
    // Security boundary — name validation (see `validate_*_name` helpers).
    // Both vectors caught during security review of the backend-wiring PR.
    // -----------------------------------------------------------------------

    /// Helper: build a manifest with arbitrary `package.name` and
    /// `components.exports`, allowing values that would normally fail
    /// the validators. Used only by the security-boundary tests below.
    fn make_package_raw(pkg_name: &str, components: &[&str]) -> TempDir {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path();
        let exports = components
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = format!(
            r#"
[package]
name = "{pkg_name}"
version = "0.1.0"
description = "fixture"
license = "MIT"

[components]
exports = [{exports}]

[dependencies]

[kernel]
version = "1"
"#
        );
        fs::write(root.join("mosaic-package.toml"), manifest).unwrap();
        // Intentionally do NOT create src/ — validation must fire
        // before any source-file lookup happens.
        tmp
    }

    /// Path-traversal regression: a component name with `..` /  `/`
    /// would let an attacker-controlled manifest write outside
    /// `output_root`. Validation must reject the name up front before
    /// any I/O happens.
    #[test]
    fn component_name_with_path_traversal_is_rejected() {
        let pkg = make_package_raw("mosaic-pkg-evil", &["../../etc/passwd"]);
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .unwrap_err();
        assert!(
            matches!(err, BuildError::UnsafeName { kind, .. } if kind == "component")
                || matches!(err, BuildError::Manifest(_)),
            "expected UnsafeName(component, …) or Manifest(...), got {err:?}"
        );
    }

    /// Component name containing a path separator alone (no `..`)
    /// must also be rejected — the joined `out_dir.join("foo/bar")`
    /// would silently create a subdirectory under the dist root.
    #[test]
    fn component_name_with_slash_is_rejected() {
        let pkg = make_package_raw("mosaic-pkg-evil", &["foo/bar"]);
        let out = TempDir::new().unwrap();
        let err = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .unwrap_err();
        // The manifest parser may catch some of these earlier as a
        // `Manifest(...)` error (it does its own kebab/PascalCase
        // sanity-check); we accept either error type as long as the
        // build is rejected. The validator's role is to be the second
        // line of defense if the manifest parser ever loosens its
        // grammar.
        assert!(
            matches!(err, BuildError::UnsafeName { kind, .. } if kind == "component")
                || matches!(err, BuildError::Manifest(_)),
            "expected UnsafeName(component, …) or Manifest(...), got {err:?}"
        );
    }

    /// HTML/JS/XML injection regression: a component name containing
    /// quote / angle-bracket / `-->` characters would break out of
    /// the generated `import "./X.js"`, `<Page Include="X.xaml">`,
    /// or `<!-- Component: X -->` strings. The validator's strict
    /// `[A-Za-z][A-Za-z0-9_]*` rule blocks every such character.
    #[test]
    fn component_name_with_injection_characters_is_rejected() {
        // Some injection chars (`"`) trip the TOML parser before our
        // validator runs — that's fine, both are rejection paths.
        // The validator is the second line of defence; either error
        // type counts.
        for bad in [
            "Grid<script>",
            "Grid-->",
            "Grid; rm -rf /",
            "1Grid",     // must lead with a letter
            "",          // empty
            "Grid-Hi",   // hyphen not allowed in components (PascalCase only)
            "Grid.evil", // dot not allowed
            "Grid$",     // special chars
        ] {
            let pkg = make_package_raw("mosaic-pkg-evil", &[bad]);
            let out = TempDir::new().unwrap();
            let err = build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend: Backend::Html,
                emit_project: false,
                theme: None,
            })
            .unwrap_err();
            assert!(
                matches!(err, BuildError::UnsafeName { kind, .. } if kind == "component")
                    || matches!(err, BuildError::Manifest(_)),
                "expected UnsafeName or Manifest error for {bad:?}, got {err:?}"
            );
        }
    }

    /// Package name validation: kebab-case only. Capitals, dots,
    /// path separators, and injection characters must all be
    /// rejected before reaching the qmldir / props / index files.
    /// The manifest parser already enforces most of this (it does
    /// its own kebab-case sanity check), so the assertion accepts
    /// either error type. The validator's role is to be the second
    /// line of defense if the manifest parser ever loosens its
    /// grammar.
    #[test]
    fn package_name_validation_rejects_unsafe_shapes() {
        for bad in [
            "Mosaic-Pkg-Grid", // capitals not allowed
            "mosaic.pkg.grid", // dots not allowed
            "../escape",       // path traversal
            "mosaic-pkg-grid\"",
            "9starts-with-digit",
            "",
        ] {
            let pkg = make_package_raw(bad, &["Grid"]);
            let out = TempDir::new().unwrap();
            let err = build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend: Backend::React,
                emit_project: false,
                theme: None,
            })
            .unwrap_err();
            assert!(
                matches!(err, BuildError::UnsafeName { kind, .. } if kind == "package")
                    || matches!(err, BuildError::Manifest(_)),
                "expected UnsafeName(package, …) or Manifest(...) for {bad:?}, got {err:?}"
            );
        }
    }

    /// Positive case: the standard PascalCase component names that
    /// every existing package uses must pass validation cleanly.
    #[test]
    fn standard_component_names_pass_validation() {
        assert!(validate_component_name("Grid").is_ok());
        assert!(validate_component_name("HostInput").is_ok());
        assert!(validate_component_name("Component1").is_ok());
        assert!(validate_component_name("A_b").is_ok());
    }

    /// Positive case: the standard `mosaic-pkg-*` package names that
    /// every existing package uses must pass validation cleanly.
    #[test]
    fn standard_package_names_pass_validation() {
        assert!(validate_package_name("mosaic-pkg-grid").is_ok());
        assert!(validate_package_name("mosaic-pkg-dialog").is_ok());
        assert!(validate_package_name("mosaic-pkg-toolkit").is_ok());
        assert!(validate_package_name("a").is_ok());
        assert!(validate_package_name("foo-bar-1").is_ok());
    }

    /// Cross-cutting: a multi-component package builds every component
    /// on every newly-wired backend without losing any. Pins the
    /// "no silent skip" invariant.
    #[test]
    fn multi_component_builds_on_all_newer_backends() {
        let pkg = make_package("mosaic-pkg-multi", &["Alpha", "Beta"]);
        let out = TempDir::new().unwrap();

        for backend in [
            Backend::Html,
            Backend::WebComponent,
            Backend::Xaml,
            Backend::Flutter,
        ] {
            let result = build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend,
                emit_project: false,
                theme: None,
            })
            .unwrap_or_else(|e| panic!("{backend:?} build failed: {e:?}"));
            assert_eq!(result.components_built.len(), 2);
        }

        // All four backends must have produced their per-component files.
        assert!(out.path().join("html").join("Alpha.html").exists());
        assert!(out.path().join("html").join("Beta.html").exists());
        assert!(out.path().join("webcomponent").join("Alpha.js").exists());
        assert!(out.path().join("webcomponent").join("Beta.js").exists());
        assert!(out.path().join("xaml").join("Alpha.xaml").exists());
        assert!(out.path().join("xaml").join("Beta.xaml").exists());
        assert!(out.path().join("flutter").join("Alpha.dart").exists());
        assert!(out.path().join("flutter").join("Beta.dart").exists());
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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
            emit_project: false,
            theme: None,
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

    // -----------------------------------------------------------------------
    // UI30 multi-layout — discover_variants + variant artifact filenames
    // -----------------------------------------------------------------------

    /// Bare default only: a component with `Grid.mll` (no variant files)
    /// returns `[None]` — the back-compat case that proves UI30 didn't
    /// regress the single-variant pipeline.
    #[test]
    fn discover_variants_bare_default_only_returns_single_none() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let src = pkg.path().join("src");
        let v = discover_variants(&src, "Grid").unwrap();
        assert_eq!(v, vec![None]);
    }

    /// Bare default + one named variant: returns `[None, Some("touch")]`
    /// in that order (default first per UI30 §5).
    #[test]
    fn discover_variants_default_plus_named_returns_both_in_order() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let src = pkg.path().join("src");
        fs::write(src.join("Grid.touch.mll"), minimal_mll("Grid")).unwrap();
        let v = discover_variants(&src, "Grid").unwrap();
        assert_eq!(v, vec![None, Some("touch".to_string())]);
    }

    /// Multiple named variants without a bare default: only the named
    /// variants are returned, sorted alphabetically. This is the
    /// "strict mode" the spec mentions — the package author can omit
    /// the bare default to prevent the fallback chain from firing.
    #[test]
    fn discover_variants_only_named_variants_no_default() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().to_path_buf();
        // No bare Grid.mll; only desktop + touch variants.
        fs::write(src.join("Grid.mil"), minimal_mil("Grid")).unwrap();
        fs::write(src.join("Grid.touch.mll"), minimal_mll("Grid")).unwrap();
        fs::write(src.join("Grid.desktop.mll"), minimal_mll("Grid")).unwrap();
        let v = discover_variants(&src, "Grid").unwrap();
        assert_eq!(
            v,
            vec![Some("desktop".to_string()), Some("touch".to_string())]
        );
    }

    /// No `.mll` files at all: returns `[None]` so the existing
    /// SourceNotFound error path still fires (back-compat UX).
    #[test]
    fn discover_variants_no_mll_files_returns_single_none() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Grid.mil"), minimal_mil("Grid")).unwrap();
        let v = discover_variants(tmp.path(), "Grid").unwrap();
        assert_eq!(v, vec![None]);
    }

    /// Different components in the same src/ don't cross-pollute. Looking
    /// for `Grid`'s variants must not pick up `Sidebar.touch.mll`.
    #[test]
    fn discover_variants_does_not_cross_pollute_components() {
        let pkg = make_package("mosaic-pkg-multi", &["Grid", "Sidebar"]);
        let src = pkg.path().join("src");
        fs::write(src.join("Sidebar.touch.mll"), minimal_mll("Sidebar")).unwrap();
        let v = discover_variants(&src, "Grid").unwrap();
        assert_eq!(
            v,
            vec![None],
            "Grid should only see its own .mll, not Sidebar's"
        );
    }

    /// Filenames that share a prefix-stem but have weird middles are
    /// silently skipped (the middle would be ambiguous as a variant
    /// name). `Grid.dark.theme.mll` has middle `dark.theme` which
    /// contains a dot — we skip it.
    #[test]
    fn discover_variants_skips_ambiguous_dotted_middles() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().to_path_buf();
        fs::write(src.join("Grid.mil"), minimal_mil("Grid")).unwrap();
        fs::write(src.join("Grid.mll"), minimal_mll("Grid")).unwrap();
        fs::write(src.join("Grid.touch.mll"), minimal_mll("Grid")).unwrap();
        // Decoy: dotted middle is not a single clean variant name.
        fs::write(src.join("Grid.dark.theme.mll"), minimal_mll("Grid")).unwrap();
        let v = discover_variants(&src, "Grid").unwrap();
        assert_eq!(
            v,
            vec![None, Some("touch".to_string())],
            "ambiguous dotted middle must be skipped"
        );
    }

    // -----------------------------------------------------------------------
    // Theme axis — resolve_style_path honours the `theme` selector, the
    // style (`.msl`) analogue of the layout (`.mll`) `variant` axis.
    // -----------------------------------------------------------------------

    /// A themed style file wins when it exists; otherwise resolution falls
    /// back to the bare `.msl`, then to the alphabetically-first stylesheet.
    /// This is the unit-level contract that keeps `.light.msl` from being the
    /// dead code it was before the theme axis existed.
    #[test]
    fn resolve_style_path_honours_theme_selector() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path();
        fs::write(src.join("Grid.dark.msl"), minimal_msl("Grid")).unwrap();
        fs::write(src.join("Grid.light.msl"), minimal_msl("Grid")).unwrap();

        // Exact theme match wins for each theme.
        let light = resolve_style_path(src, "Grid", Some("light"))
            .unwrap()
            .unwrap();
        assert_eq!(light.file_name().unwrap(), "Grid.light.msl");
        let dark = resolve_style_path(src, "Grid", Some("dark"))
            .unwrap()
            .unwrap();
        assert_eq!(dark.file_name().unwrap(), "Grid.dark.msl");

        // Theme-agnostic (None) resolution keeps the historical
        // alphabetically-first default → `dark` beats `light`.
        let none = resolve_style_path(src, "Grid", None).unwrap().unwrap();
        assert_eq!(
            none.file_name().unwrap(),
            "Grid.dark.msl",
            "None must preserve the pre-theme-axis dark-wins default"
        );

        // A requested theme with no matching file, and no bare `.msl`,
        // degrades to the alphabetically-first stylesheet rather than
        // erroring or producing nothing (migration-friendly fallback).
        let sepia = resolve_style_path(src, "Grid", Some("sepia"))
            .unwrap()
            .unwrap();
        assert_eq!(sepia.file_name().unwrap(), "Grid.dark.msl");

        // A bare `.msl` is the theme-neutral fallback ahead of the
        // alphabetical scan: adding it makes an unknown theme resolve to
        // the bare file, and the None path prefer it too.
        fs::write(src.join("Grid.msl"), minimal_msl("Grid")).unwrap();
        let bare_fallback = resolve_style_path(src, "Grid", Some("sepia"))
            .unwrap()
            .unwrap();
        assert_eq!(bare_fallback.file_name().unwrap(), "Grid.msl");
        let none_bare = resolve_style_path(src, "Grid", None).unwrap().unwrap();
        assert_eq!(none_bare.file_name().unwrap(), "Grid.msl");
        // But the exact theme file still wins over the bare file.
        let light_over_bare = resolve_style_path(src, "Grid", Some("light"))
            .unwrap()
            .unwrap();
        assert_eq!(light_over_bare.file_name().unwrap(), "Grid.light.msl");
    }

    /// End-to-end: `build_package` with `theme: Some("light")` emits the
    /// LIGHT stylesheet's declarations, not the dark ones. Proves the theme
    /// selector reaches the emitted artifact (the React `.lattice` sidecar).
    #[test]
    fn build_package_theme_selects_light_style() {
        // Component authored with two distinct themed stylesheets and no
        // bare `.msl` — exactly how the Engram components are shaped.
        let pkg = make_package_with("mosaic-pkg-grid", &["Grid"], /* write_msl = */ false);
        let src = pkg.path().join("src");
        fs::write(
            src.join("Grid.dark.msl"),
            "style Grid { part root { width: 11% ; } }\n",
        )
        .unwrap();
        fs::write(
            src.join("Grid.light.msl"),
            "style Grid { part root { width: 22% ; } }\n",
        )
        .unwrap();

        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: Some("light".to_string()),
        })
        .expect("light-theme react build");

        let lattice = fs::read_to_string(out.path().join("react").join("Grid.lattice")).unwrap();
        assert!(
            lattice.contains("width: 22%"),
            "light build must emit the LIGHT stylesheet's declarations"
        );
        assert!(
            !lattice.contains("width: 11%"),
            "light build must NOT emit the dark stylesheet's declarations"
        );
    }

    /// `build_package` — the public library entry point — rejects a
    /// path-traversing `theme` itself, without relying on the CLI guard. This
    /// is the defense-in-depth contract for programmatic callers.
    #[test]
    fn build_package_rejects_unsafe_theme() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        for bad in ["../../../etc/passwd", "a/b", "a.b", "", "x\0y"] {
            let err = build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend: Backend::React,
                emit_project: false,
                theme: Some(bad.to_string()),
            })
            .unwrap_err();
            assert!(
                matches!(&err, BuildError::UnsafeName { kind, .. } if *kind == "theme"),
                "theme {bad:?} must be rejected as UnsafeName(theme), got {err:?}"
            );
        }
        // A safe theme name is accepted (no themed file exists, so it falls
        // back to the bare stylesheet — the point is validation lets it through).
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: Some("light".to_string()),
        })
        .expect("a safe theme name must pass validation");
    }

    /// End-to-end: a package with one component + one named variant
    /// builds BOTH artifacts under their UI30 filenames. `Grid.tsx`
    /// (default) and `Grid.touch.tsx` (variant) coexist in the same
    /// output directory.
    #[test]
    fn build_package_emits_both_default_and_variant_artifacts() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        // Add a touch variant alongside the default.
        let src = pkg.path().join("src");
        fs::write(src.join("Grid.touch.mll"), minimal_mll("Grid")).unwrap();

        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .expect("multi-variant build");

        // components_built tracks COMPONENTS, not artifacts (the index
        // file should list Grid once, not twice).
        assert_eq!(result.components_built, vec!["Grid".to_string()]);

        // Two component artifacts + two Lattice sidecars + one index file.
        let default_path = out.path().join("react").join("Grid.tsx");
        let touch_path = out.path().join("react").join("Grid.touch.tsx");
        let default_lattice = out.path().join("react").join("Grid.lattice");
        let touch_lattice = out.path().join("react").join("Grid.touch.lattice");
        assert!(default_path.exists(), "Grid.tsx (default) must exist");
        assert!(touch_path.exists(), "Grid.touch.tsx (variant) must exist");
        assert!(
            default_lattice.exists() && touch_lattice.exists(),
            "each variant should have a matching Lattice sidecar"
        );
        assert!(
            result.artifacts.iter().any(|p| p == &default_path)
                && result.artifacts.iter().any(|p| p == &touch_path)
                && result.artifacts.iter().any(|p| p == &default_lattice)
                && result.artifacts.iter().any(|p| p == &touch_lattice),
            "component and Lattice artifact paths must be in the result"
        );
    }

    /// Back-compat regression test: a package with only the bare
    /// default `.mll` still produces exactly one unsuffixed primary
    /// artifact per component. UI30 is opt-in via filesystem and existing
    /// component-code filenames must build identically.
    #[test]
    fn build_package_without_variants_is_unchanged_from_pre_ui30() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .expect("single-variant build");

        // Exactly one component artifact + one Lattice sidecar + the index file.
        assert_eq!(result.artifacts.len(), 3);
        let default_path = out.path().join("react").join("Grid.tsx");
        assert!(default_path.exists());
        // NO variant-suffixed file should exist.
        assert!(
            !out.path().join("react").join("Grid.touch.tsx").exists(),
            "no variant file should be created without explicit .touch.mll"
        );
    }

    // =====================================================================
    // UI32-M — `emit_project` shell tests
    //
    // Covers per-PR gates from UI32 spec §3.1-§3.8 at the
    // artifact-builder layer (the per-emitter L2-L7 PRs cover the
    // shell-content gates already; these tests cover the build_package
    // integration).
    // =====================================================================

    /// §3.4 Composable: default options (emit_project: false) leave
    /// the build_package output bit-for-bit identical to pre-UI32-M.
    /// No shell side-files appear in backend_dir.
    #[test]
    fn ui32_m_emit_project_false_does_not_emit_shell_side_files() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: false,
            theme: None,
        })
        .expect("react build with emit_project: false");
        assert!(out.path().join("react").join("Grid.tsx").exists());
        // None of the L2 React shell side-files should exist.
        assert!(
            !out.path().join("react").join("package.json").exists(),
            "package.json must not exist when emit_project is false"
        );
        assert!(
            !out.path().join("react").join("vite.config.ts").exists(),
            "vite.config.ts must not exist when emit_project is false"
        );
        // No shell artifacts beyond the per-component sidecar pair + index.ts.
        assert_eq!(result.artifacts.len(), 3); // Grid.tsx + Grid.lattice + index.ts
    }

    /// §3.4 Composable: when emit_project is true, the React backend
    /// produces a Vite project shell alongside the per-component
    /// .tsx + the index.ts (no overwrite of the bare per-component
    /// artifacts).
    #[test]
    fn ui32_m_emit_project_true_writes_react_vite_shell() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        let result = build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::React,
            emit_project: true,
            theme: None,
        })
        .expect("react build with emit_project: true");

        let dir = out.path().join("react");
        // Per-component artifacts still present.
        assert!(dir.join("Grid.tsx").exists());
        assert!(dir.join("index.ts").exists());
        // L2 React shell side-files now present.
        assert!(dir.join("package.json").exists(), "package.json missing");
        assert!(
            dir.join("vite.config.ts").exists(),
            "vite.config.ts missing"
        );
        assert!(dir.join("tsconfig.json").exists(), "tsconfig.json missing");
        assert!(dir.join("index.html").exists(), "index.html missing");
        assert!(dir.join("README.md").exists(), "README.md missing");
        assert!(
            dir.join("src/main.tsx").exists(),
            "src/main.tsx missing (nested per Vite convention)"
        );
        // package.json carries the per-emitter banner.
        let pkg_json = fs::read_to_string(dir.join("package.json")).unwrap();
        assert!(
            pkg_json.contains("AUTO-GENERATED by mosaic-compile --emit-project"),
            "package.json missing UI32 §3.5 banner"
        );
        // The shell side-files appear in the returned artifacts list
        // so callers can stream-upload them.
        assert!(
            result.artifacts.iter().any(|p| p.ends_with("package.json")),
            "package.json must appear in result.artifacts"
        );
        assert!(
            result.artifacts.iter().any(|p| p.ends_with("main.tsx")),
            "src/main.tsx must appear in result.artifacts"
        );
    }

    /// Per-backend smoke test: each backend produces its expected
    /// shell side-files when emit_project is true. Doesn't
    /// re-test the banner/pinning/etc. contracts (the per-emitter
    /// L2-L7 PRs do that); just confirms the artifact-builder
    /// routes correctly.
    #[test]
    fn ui32_m_emit_project_true_produces_expected_shell_per_backend() {
        for (backend, expected_files) in [
            (
                Backend::React,
                vec![
                    "package.json",
                    "vite.config.ts",
                    "tsconfig.json",
                    "index.html",
                    "README.md",
                    "src/main.tsx",
                ],
            ),
            (
                Backend::Electron,
                vec![
                    "Grid.tsx",
                    "index.ts",
                    "package.json",
                    "vite.config.ts",
                    "index.html",
                    "tsconfig.json",
                    "tsconfig.electron.json",
                    "src/main.tsx",
                    "electron/main.ts",
                    "electron/preload.ts",
                    "README.md",
                ],
            ),
            (Backend::Html, vec!["index.html", "main.js", "README.md"]),
            (
                Backend::WebComponent,
                vec!["index.html", "main.js", "README.md"],
            ),
            (
                Backend::Flutter,
                vec![
                    "pubspec.yaml",
                    "README.md",
                    "lib/main.dart",
                    "lib/Grid.dart",
                    "lib/mosaic_host.dart",
                ],
            ),
            (
                Backend::Compose,
                vec![
                    "Grid.kt",
                    "index.kt",
                    "settings.gradle.kts",
                    "build.gradle.kts",
                    "src/main/kotlin/Main.kt",
                    "src/main/kotlin/Grid.kt",
                    "README.md",
                ],
            ),
            (
                Backend::Qt,
                vec!["CMakeLists.txt", "main.cpp", "qmldir", "README.md"],
            ),
            (
                Backend::SwiftUI,
                vec![
                    "Package.swift",
                    "README.md",
                    "Sources/App/App.swift",
                    "Sources/App/Grid.swift",
                ],
            ),
            (
                Backend::Xaml,
                vec![
                    "Grid.csproj",
                    "App.xaml",
                    "App.xaml.cs",
                    "MainWindow.xaml",
                    "MainWindow.xaml.cs",
                    "app.manifest",
                    "build.ps1",
                    "README.md",
                ],
            ),
        ] {
            let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
            let out = TempDir::new().unwrap();
            build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend,
                emit_project: true,
                theme: None,
            })
            .unwrap_or_else(|e| panic!("{backend:?} build failed: {e:?}"));
            let dir = out.path().join(backend.dir_name());
            for rel in &expected_files {
                assert!(
                    dir.join(rel).exists(),
                    "{backend:?}: expected shell file `{rel}` missing"
                );
            }
        }
    }

    #[test]
    fn venture_browser_builds_and_mounts_host_surface_on_every_backend() {
        let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("derive code root")
            .join("programs/mosaic/venture-browser");

        for backend in Backend::ALL {
            let out = TempDir::new().expect("backend output dir");
            build_package(&BuildOptions {
                package_root: package_root.clone(),
                output_root: out.path().to_path_buf(),
                backend,
                emit_project: true,
                theme: Some("light".to_string()),
            })
            .unwrap_or_else(|error| panic!("{backend:?} Venture build failed: {error:?}"));

            let extension = backend.component_extension().expect("text backend");
            let artifact = out
                .path()
                .join(backend.dir_name())
                .join(format!("VentureChrome.{extension}"));
            let source = fs::read_to_string(&artifact)
                .unwrap_or_else(|error| panic!("read {}: {error}", artifact.display()));
            let expected_mounts: &[&str] = match backend {
                Backend::React | Backend::Electron => &[
                    "data-mosaic-host-surface=\"content-surface\"",
                    "{contentSurface}",
                ],
                Backend::SwiftUI => &["contentSurface"],
                Backend::Qt => &["Loader {", "sourceComponent: mosaicRoot.contentSurface"],
                Backend::WebComponent => &["<slot name=\"content-surface\"></slot>"],
                Backend::Html => &["{{{contentSurface}}}"],
                Backend::Xaml => {
                    &["<ContentPresenter Content=\"{x:Bind ContentSurface, Mode=OneWay}\"/>"]
                }
                Backend::Flutter => &["final Widget contentSurface;", "contentSurface,"],
                Backend::Compose => &["contentSurface: @Composable () -> Unit", "contentSurface()"],
            };
            for expected in expected_mounts {
                assert!(
                    source.contains(expected),
                    "{backend:?} must mount the host surface with {expected:?}:\n{source}"
                );
            }
            assert!(
                !source.contains("component reference 'HostSurface'"),
                "{backend:?} must not silently replace HostSurface with a placeholder"
            );

            let (shell_path, expected_shell_mounts): (&str, &[&str]) = match backend {
                Backend::React | Backend::Electron => (
                    "src/main.tsx",
                    &[
                        "window.mosaicHost?.getProps",
                        "<{component_name} {...props}",
                    ],
                ),
                Backend::SwiftUI => (
                    "Sources/App/App.swift",
                    &[
                        "contentSurface: host.node(named: \"content-surface\")",
                        "optional func node(named name: NSString)",
                        "MosaicHostPlatformView",
                    ],
                ),
                Backend::Qt => (
                    "main.cpp",
                    &[
                        "root->setProperty(\"mosaicHost\"",
                        "QVariant::fromValue(static_cast<QObject *>(&mosaicHost))",
                    ],
                ),
                Backend::WebComponent => (
                    "main.js",
                    &[
                        "applyNodeSlot(slot, value)",
                        "value.setAttribute(\"slot\", slot.name)",
                    ],
                ),
                Backend::Html => (
                    "main.js",
                    &[
                        "value instanceof Node",
                        "surface.replaceChildren(value)",
                        "trustedHostHtml(readPath(context, key))",
                        "function trustedHostHtml(value)",
                    ],
                ),
                Backend::Xaml => (
                    "MainWindow.xaml.cs",
                    &[
                        "TryApplyMosaicHostProps(this.Component)",
                        "FindMosaicHostMethod(\"ApplyProps\"",
                    ],
                ),
                Backend::Flutter => (
                    "lib/main.dart",
                    &[
                        "contentSurface: mosaicWidget(_hostProps, \"content-surface\"",
                        "Widget mosaicWidget(",
                    ],
                ),
                Backend::Compose => (
                    "src/main/kotlin/Main.kt",
                    &[
                        "contentSurface = mosaicNode(hostProps, \"content-surface\"",
                        "private fun mosaicNode(",
                    ],
                ),
            };
            let shell = fs::read_to_string(out.path().join(backend.dir_name()).join(shell_path))
                .unwrap_or_else(|error| {
                    panic!("read {backend:?} Venture project shell {shell_path}: {error}")
                });
            for expected in expected_shell_mounts {
                let expected = expected.replace("{component_name}", "VentureChrome");
                assert!(
                    shell.contains(&expected),
                    "{backend:?} project shell must accept the host surface through its MosaicHost contract with {expected:?}:\n{shell}"
                );
            }
        }
    }

    #[test]
    fn flutter_project_shell_exposes_mosaic_host_hook() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Flutter,
            emit_project: true,
            theme: None,
        })
        .expect("flutter package build");

        let dir = out.path().join("flutter");
        let main_dart = fs::read_to_string(dir.join("lib/main.dart")).expect("main.dart");
        assert!(main_dart.contains("import 'mosaic_host.dart';"));
        assert!(main_dart.contains("MosaicHost.load()"));
        assert!(main_dart.contains("_queueMosaicResponse(_mosaicHost?.props())"));
        assert!(main_dart.contains("String mosaicString(Map<String, Object?> props"));
        assert!(main_dart.contains("_mosaicHost?.handleEvent(event.mosaicEnvelope)"));
        assert!(main_dart.contains("_queueMosaicResponse(response);"));
        assert!(main_dart.contains("debugPrint(\"event: ${event.mosaicEnvelope}\")"));

        let host = fs::read_to_string(dir.join("lib/mosaic_host.dart")).expect("mosaic_host.dart");
        assert!(host.contains("class MosaicHost"));
        assert!(host.contains("static MosaicHost? load() => null;"));
        assert!(host.contains("FutureOr<Map<String, Object?>?> handleEvent"));
    }

    #[test]
    fn compose_package_artifact_exposes_mosaic_event_envelope() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Compose,
            emit_project: true,
            theme: None,
        })
        .expect("compose package build");

        let dir = out.path().join("compose");
        let kotlin = fs::read_to_string(dir.join("Grid.kt")).expect("Grid.kt");
        assert!(kotlin.contains("sealed class GridEvent {"));
        assert!(kotlin.contains("abstract val mosaicName: String"));
        assert!(kotlin.contains("val mosaicEnvelope: Map<String, Any?>"));
        assert!(kotlin.contains("@Composable"));
        assert!(kotlin.contains("fun Grid("));

        let index = fs::read_to_string(dir.join("index.kt")).expect("index.kt");
        assert!(index.contains("// Component: Grid (see Grid.kt)"));
        let settings =
            fs::read_to_string(dir.join("settings.gradle.kts")).expect("settings.gradle.kts");
        assert!(settings.contains("rootProject.name = \"mosaic-pkg-grid\""));
        let gradle = fs::read_to_string(dir.join("build.gradle.kts")).expect("build.gradle.kts");
        assert!(gradle.contains("id(\"org.jetbrains.compose\") version \"1.11.1\""));
        assert!(gradle.contains("id(\"org.jetbrains.kotlin.plugin.compose\") version \"2.3.21\""));
        assert!(gradle.contains("kotlin(\"jvm\") version \"2.3.21\""));
        assert!(gradle.contains("mainClass = \"MainKt\""));
        assert!(gradle.contains("packageName = \"mosaic_pkg_grid\""));
        let main_kt = fs::read_to_string(dir.join("src/main/kotlin/Main.kt")).expect("Main.kt");
        assert!(main_kt.contains("fun main() = application"));
        assert!(main_kt.contains("Window(onCloseRequest = ::exitApplication, title = \"Grid\")"));
        assert!(main_kt.contains("MosaicComposeHostBridge.load()"));
        assert!(main_kt.contains("var hostProps by remember"));
        assert!(main_kt.contains("applyMosaicResponse(mosaicHost?.props())"));
        assert!(main_kt.contains("Grid("));
        assert!(main_kt.contains("private fun mosaicString("));
        assert!(main_kt.contains("mosaicHost?.handleEvent(event.mosaicEnvelope)"));
        assert!(
            main_kt.contains("if (response == null) println(\"event: ${event.mosaicEnvelope}\")")
        );
        assert!(main_kt.contains("Class.forName(\"MosaicHost\")"));
        let nested_kotlin =
            fs::read_to_string(dir.join("src/main/kotlin/Grid.kt")).expect("src Grid.kt");
        assert_eq!(kotlin, nested_kotlin);
        let readme = fs::read_to_string(dir.join("README.md")).expect("README.md");
        assert!(readme.contains("Compose Desktop shell"));
        assert!(readme.contains("optional `MosaicHost`"));
    }

    #[test]
    fn ui32_m_electron_project_shell_exposes_mosaic_host_ipc_bridge() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Electron,
            emit_project: true,
            theme: None,
        })
        .expect("electron build with emit_project: true");

        let dir = out.path().join("electron");
        let package_json = fs::read_to_string(dir.join("package.json")).unwrap();
        assert!(
            package_json.contains("\"dev\": \"tsc -p tsconfig.electron.json && concurrently -k")
        );
        let readme = fs::read_to_string(dir.join("README.md")).unwrap();
        assert!(
            readme.contains("`npm run dev` compiles `electron/main.ts` and `electron/preload.ts`")
        );

        let main_ts = fs::read_to_string(dir.join("electron/main.ts")).unwrap();
        assert!(main_ts.contains("import { app, BrowserWindow, ipcMain } from \"electron\";"));
        assert!(main_ts.contains("import { existsSync } from \"node:fs\";"));
        assert!(main_ts.contains("pathToFileURL"));
        assert!(main_ts.contains("MOSAIC_ELECTRON_HOST_MODULE"));
        assert!(main_ts.contains("function mosaicHostModuleCandidates(): string[]"));
        assert!(main_ts.contains("path.join(__dirname, \"..\", \"electron\", \"host.js\")"));
        assert!(main_ts.contains("path.join(__dirname, \"..\", \"electron\", \"host.mjs\")"));
        assert!(main_ts.contains("async function loadMosaicHost()"));
        assert!(main_ts.contains("let mosaicHost: MosaicHost = {};"));
        assert!(main_ts.contains("MOSAIC_GET_PROPS_CHANNEL"));
        assert!(main_ts.contains("ipcMain.handle("));
        assert!(main_ts.contains("mosaic:get-props"));
        assert!(main_ts.contains("mosaic:handle-event"));
        assert!(main_ts.contains("mosaicHost.getProps?.(request)"));
        assert!(main_ts.contains("mosaicHost.handleEvent?.(request)"));
        assert!(!main_ts.contains("=> undefined"));

        let preload_ts = fs::read_to_string(dir.join("electron/preload.ts")).unwrap();
        assert!(preload_ts.contains("import { contextBridge, ipcRenderer } from \"electron\";"));
        assert!(preload_ts.contains("contextBridge.exposeInMainWorld(\"mosaicHost\""));
        assert!(preload_ts.contains("getProps: (request: MosaicHostRequest)"));
        assert!(preload_ts.contains("handleEvent: (request: MosaicHostRequest)"));
        assert!(preload_ts.contains("ipcRenderer.invoke(MOSAIC_GET_PROPS_CHANNEL, request)"));
        assert!(preload_ts.contains("ipcRenderer.invoke(MOSAIC_HANDLE_EVENT_CHANNEL, request)"));
    }

    /// XAML's --emit-project path writes a full WinUI host shell via
    /// the package artifact builder, alongside the component triple
    /// and package props fragment.
    #[test]
    fn ui32_m_emit_project_true_xaml_writes_project_shell() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out = TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: pkg.path().to_path_buf(),
            output_root: out.path().to_path_buf(),
            backend: Backend::Xaml,
            emit_project: true,
            theme: None,
        })
        .expect("xaml build with emit_project: true");
        let dir = out.path().join("xaml");
        assert!(dir.join("Grid.xaml").exists());
        assert!(dir.join("Grid.xaml.cs").exists());
        assert!(dir.join("Grid.Event.cs").exists());
        assert!(dir.join("MosaicPackage.props").exists());
        assert!(dir.join("Grid.csproj").exists());
        assert!(dir.join("App.xaml").exists());
        assert!(dir.join("MainWindow.xaml").exists());
    }

    /// §3.1 Reproducible: two emit_project builds against the same
    /// inputs produce bit-for-bit identical shell side-files.
    #[test]
    fn ui32_m_emit_project_shell_is_byte_deterministic() {
        let pkg = make_package("mosaic-pkg-grid", &["Grid"]);
        let out_a = TempDir::new().unwrap();
        let out_b = TempDir::new().unwrap();
        for out in [&out_a, &out_b] {
            build_package(&BuildOptions {
                package_root: pkg.path().to_path_buf(),
                output_root: out.path().to_path_buf(),
                backend: Backend::React,
                emit_project: true,
                theme: None,
            })
            .expect("react build");
        }
        for shell_file in [
            "package.json",
            "vite.config.ts",
            "index.html",
            "README.md",
            "src/main.tsx",
        ] {
            let a = fs::read_to_string(out_a.path().join("react").join(shell_file)).unwrap();
            let b = fs::read_to_string(out_b.path().join("react").join(shell_file)).unwrap();
            assert_eq!(a, b, "`{shell_file}` is not deterministic between runs");
        }
    }
}
