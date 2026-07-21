//! # mosaic-package-resolver
//!
//! Component-reference resolver for Mosaic packages, implementing
//! **UI29 §4.4** (Mosaic Primitive Kernel — "Resolving a component
//! reference").
//!
//! ## The one question this crate answers
//!
//! Given a tag name from some user's `.mll` file — e.g. `Grid` or `Row`
//! or `Floof` — what *is* it?
//!
//! ```text
//!                       resolve("Grid")
//!                              │
//!         ┌────────────────────┼────────────────────┐
//!         ▼                    ▼                    ▼
//!     Resolution::         Resolution::            None
//!     Kernel               Component { … }      (unknown tag —
//!     (Row, Box, …)        (declared by a       compiler will
//!                          dependency)          surface a "no
//!                                                such tag" error)
//! ```
//!
//! The resolver is a precomputed `HashMap` plus a `HashSet` of kernel
//! names, so `resolve` is O(1).  The cost is paid once in [`build`].
//!
//! ## What `build` does, in order
//!
//! 1. Tries to read `<package_root>/mosaic-package.toml`.  Absent is
//!    fine — a manifest-less user package can still consult the kernel
//!    primitives.
//! 2. For each `(dep_name, _ver)` in the manifest's `[dependencies]`:
//!    a. Search each path in `search_paths` for a child directory.  We
//!    try `mosaic-pkg-<dep_name>` first (the UI29 §4.1 convention),
//!    then fall back to the literal `<dep_name>`.
//!    b. Read that dep's `mosaic-package.toml` via
//!    `mosaic_package_manifest::parse_path`.
//!    c. For each entry in its `[components].exports`, register a
//!    `Resolution::Component { … }` keyed by the component name.
//!    d. If two dependencies export the same name → `DuplicateExport`.
//! 3. Build the kernel set from [`KERNEL_PRIMITIVES`].
//!
//! ## Why two name forms (`mosaic-pkg-<name>` and `<name>`)?
//!
//! In production a user's manifest will say
//! `mosaic-pkg-grid = "0.1.0"`, and on disk the package is at
//! `code/packages/mosaic/mosaic-pkg-grid/`.  The literal-name fallback exists
//! for tests and ad-hoc packages where the directory just happens to be
//! named the same as the dep key.  Trying both is cheap and keeps the
//! UX forgiving.
//!
//! ## Error surface
//!
//! ```text
//!     build(...)
//!         │
//!         ├── DependencyNotFound    ← name not in any search path
//!         ├── BadDependencyManifest ← dep's TOML didn't parse
//!         ├── DuplicateExport       ← two deps export same name
//!         └── Io                    ← read_dir / canonicalize failed
//! ```
//!
//! Resolving a *tag* never errors — unknown is `None`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mosaic_package_manifest::MosaicPackage;
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp, LayoutPropValue};

// ---------------------------------------------------------------------------
// Kernel set
// ---------------------------------------------------------------------------

/// The frozen Mosaic primitive kernel.  See **UI29 §2.1**.
///
/// UI29 §2.1 enumerates *15* primitives.  We list 16 here because the
/// `moslayout-compiler` tokenizer treats `Else` as a separate tag even
/// though UI29 describes it as the second half of an `If/Else` pair
/// (the parser stitches the two together at AST-construction time).
/// Treating `Else` as a kernel-known tag here means the resolver never
/// flags a perfectly valid `Else { … }` block as "unknown tag" simply
/// because the spec table didn't list it on its own row.  All 15 of the
/// UI29 §2.1 names are present, plus `Else`.
pub const KERNEL_PRIMITIVES: &[&str] = &[
    // Containers
    "Box",
    "Row",
    "Column",
    "Stack",
    // Leaves
    "Text",
    "Image",
    "Spacer",
    "Divider",
    "Icon",
    // Control flow (§3)
    "If",
    "Else",
    "For",
    // Host primitives (§2.1 "Host*" rows + UI29-1's HostDialog +
    // UI29-2's HostCheckbox/HostRadio + UI29-4's HostLink/HostTooltip/
    // HostNumberInput).
    // HostDialog was added in UI29-1 after mosaic-pkg-dialog v0.1.0
    // demonstrated that composing a dialog from Box+Column+Text loses
    // modal/focus/top-layer/accessibility semantics that only the
    // host's native dialog primitive (DOM <dialog>, Qt Popup, SwiftUI
    // .sheet, XAML ContentDialog) provides.
    // HostCheckbox + HostRadio were added in UI29-2 after a
    // mosaic-pkg-toolkit audit found Checkbox/Radio were fake
    // HostButton wrappers, losing the platform-native a11y role,
    // checked-state visuals (tri-state, focus ring), and keyboard
    // semantics that only the real checkbox/radio widget provides.
    // HostLink, HostTooltip, HostNumberInput were added in UI29-4
    // after the post-UI29-2 audit found Breadcrumb and Nav still
    // faked `<a>` via HostButton (losing role="link", Ctrl-click
    // open-new-tab, visited-state styling). HostTooltip and
    // HostNumberInput were promoted in the same batch — the
    // tooltip's a11y wiring (aria-describedby + hover/long-press
    // trigger heuristics) and the number-input's mobile-numeric-
    // keyboard / SpinBox-with-stepper-buttons are not reachable
    // via composition from existing kernel primitives.
    "HostInput",
    "HostButton",
    "HostTable",
    "HostScroll",
    "HostDialog",
    "HostCheckbox",
    "HostRadio",
    "HostLink",
    "HostTooltip",
    "HostNumberInput",
    // UI31 — `HostTable` sibling primitives. The structural sub-tags
    // (HostTableColGroup / HostTableHead / HostTableBody /
    // HostTableFoot) plus the cell-defining `Col` lower together with
    // HostTable into a real semantic-HTML `<table>` (and the matching
    // native widgets on every other backend). Pre-UI31 these were
    // recognised only by the React emitter's HostTable dispatcher and
    // sat outside KERNEL_PRIMITIVES — UI31 makes them first-class so
    // future backends don't need to special-case them and so
    // package-resolver-driven validation accepts them at parse time.
    "HostTableColGroup",
    "HostTableHead",
    "HostTableBody",
    "HostTableFoot",
    "Col",
    // UI35 — the drag-and-drop family. The kernel previously had no drag
    // primitive at all, which made the defining gesture of board software
    // ("drag a card to another column") inexpressible in a `.mll`.
    // Composition cannot supply it: every backend has its own native drag
    // system, and the keyboard-equivalent path, screen-reader
    // announcements, and touch support that make dragging usable are
    // per-platform concerns. Two primitives because a drag has two ends —
    // a source and a sink — and a card is typically both. The drag payload
    // is an opaque key + kind the kernel never interprets, and a drop
    // reports `before | after | into` relative to the target, which is what
    // lets one family express list reorder, cross-container moves, outline
    // nesting, and calendar drops. See `code/specs/UI35-host-drag-drop.md`.
    "HostDraggable",
    "HostDropTarget",
];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// What a tag resolves to.
///
/// `PartialEq` is implemented so tests can compare resolutions directly.
/// Equality on `PathBuf` is byte-wise; tests that need canonicalization-
/// independent comparison should compare the `package` + `component`
/// fields instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A UI29 kernel primitive.  No package to look up — the backend
    /// emitter handles it directly.
    Kernel,
    /// A component exported by a Mosaic package this user depends on.
    Component {
        /// The package's `[package].name`, e.g. `"mosaic-pkg-grid"`.
        package: String,
        /// Absolute path to the package's root directory on disk
        /// (the directory containing its `mosaic-package.toml`).
        package_path: PathBuf,
        /// The component name as it appears in the package's
        /// `[components].exports`, e.g. `"Grid"`.
        component: String,
    },
}

/// Build-time errors.  See crate-level docs for the full surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// A dependency listed in `[dependencies]` could not be located.
    DependencyNotFound {
        /// The dep name (as it appeared in the manifest).
        package: String,
        /// The search paths we looked in (in order).
        searched: Vec<PathBuf>,
    },
    /// A dependency's `mosaic-package.toml` did not parse.
    BadDependencyManifest {
        /// The dep name.
        package: String,
        /// The error from `mosaic_package_manifest::parse_path`,
        /// stringified.
        error: String,
    },
    /// Two dependencies export the same component name.
    DuplicateExport {
        component: String,
        package_a: String,
        package_b: String,
    },
    /// Filesystem I/O error (canonicalize, read_dir, etc).
    Io(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyNotFound { package, searched } => {
                write!(
                    f,
                    "dependency `{package}` not found in any search path ({} searched)",
                    searched.len()
                )
            }
            Self::BadDependencyManifest { package, error } => {
                write!(
                    f,
                    "dependency `{package}` has a malformed manifest: {error}"
                )
            }
            Self::DuplicateExport {
                component,
                package_a,
                package_b,
            } => {
                write!(
                    f,
                    "component `{component}` is exported by both `{package_a}` and `{package_b}`"
                )
            }
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ResolveError {}

// ---------------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------------

/// The resolution table.  Built once, queried often.
///
/// Internally:
/// - `kernel` is a `HashSet<&'static str>` of kernel primitives — checked
///   first because the kernel is small and tags are typically kernel-y.
/// - `table` is a `HashMap<String, Resolution>` containing only
///   `Resolution::Component` entries (kernel hits short-circuit before
///   the map is consulted).
pub struct Resolver {
    table: HashMap<String, Resolution>,
    kernel: HashSet<&'static str>,
}

impl std::fmt::Debug for Resolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolver")
            .field("kernel_count", &self.kernel.len())
            .field("component_count", &self.table.len())
            .finish()
    }
}

impl Resolver {
    /// Look up a tag.  Returns:
    ///
    /// - `Some(&Resolution::Kernel)` if `tag` is in [`KERNEL_PRIMITIVES`]
    /// - `Some(&Resolution::Component { … })` if a dep exports it
    /// - `None` otherwise
    pub fn resolve(&self, tag: &str) -> Option<&Resolution> {
        // Kernel takes precedence: a malicious or careless package
        // can't shadow `Row` even if it tried to export a component
        // named "Row" — the resolver will simply never *consult* the
        // table for kernel-named tags.
        if self.kernel.contains(tag) {
            // Return a reference to a const Kernel.  We carry one
            // sentinel value per resolver instance for this purpose.
            return Some(&KERNEL_RESOLUTION);
        }
        self.table.get(tag)
    }

    /// Whether this tag is known.  Equivalent to `resolve(tag).is_some()`
    /// but slightly cheaper because it avoids returning a reference.
    pub fn knows(&self, tag: &str) -> bool {
        self.kernel.contains(tag) || self.table.contains_key(tag)
    }

    /// Number of dep-exported components in the table.  Useful for
    /// diagnostics / tests.
    pub fn component_count(&self) -> usize {
        self.table.len()
    }

    /// Iterate (component-name, resolution) pairs for non-kernel
    /// entries.  Order is unspecified (it's a HashMap).
    pub fn components(&self) -> impl Iterator<Item = (&str, &Resolution)> {
        self.table.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// The single shared `Resolution::Kernel` value `resolve` hands out
/// references to.  Defined as a `static` so the reference has `'static`
/// lifetime — same lifetime as the `Resolver` carrying it.
static KERNEL_RESOLUTION: Resolution = Resolution::Kernel;

// ---------------------------------------------------------------------------
// Build entry point
// ---------------------------------------------------------------------------

/// Build a resolver for a user's package.
///
/// `package_root`: the directory containing the user's
/// `mosaic-package.toml`.  May not actually contain a manifest — the
/// resolver still works, just with an empty component table.
///
/// `search_paths`: directories to search for dependencies, in
/// preference order.  Typically this is `[code/packages/]` in this
/// monorepo; in a future cargo-style setup it might be a per-user
/// cache plus a project-local `vendor/` directory.
pub fn build(package_root: &Path, search_paths: &[PathBuf]) -> Result<Resolver, ResolveError> {
    // ----------------------------------------------------------------
    // Step 1: read the user's manifest (if any).
    // ----------------------------------------------------------------
    let manifest_path = package_root.join("mosaic-package.toml");
    let user_manifest = if manifest_path.exists() {
        match mosaic_package_manifest::parse_path(&manifest_path) {
            Ok(m) => Some(m),
            Err(e) => {
                // The user's own manifest being malformed isn't quite a
                // "bad dependency" — but mapping it to the same variant
                // keeps the error model small and the message clear.
                return Err(ResolveError::BadDependencyManifest {
                    package: "<user>".to_string(),
                    error: e.to_string(),
                });
            }
        }
    } else {
        None
    };

    // ----------------------------------------------------------------
    // Step 2: walk dependencies and populate the table.
    // ----------------------------------------------------------------
    let mut table: HashMap<String, Resolution> = HashMap::new();
    // Track which dep exported each component, so DuplicateExport
    // diagnostics can name *both* contributing packages.
    let mut export_origin: HashMap<String, String> = HashMap::new();

    if let Some(manifest) = user_manifest {
        for dep_name in manifest.dependencies.keys() {
            // ----- locate the dep on disk -----
            let dep_root = locate_dependency(dep_name, search_paths)?;
            // Canonicalize so `package_path` is absolute regardless of
            // how the caller passed the search paths.
            let dep_root = std::fs::canonicalize(&dep_root)
                .map_err(|e| ResolveError::Io(format!("canonicalize {dep_root:?}: {e}")))?;

            // ----- read the dep's manifest -----
            let dep_manifest_path = dep_root.join("mosaic-package.toml");
            let dep_manifest =
                mosaic_package_manifest::parse_path(&dep_manifest_path).map_err(|e| {
                    ResolveError::BadDependencyManifest {
                        package: dep_name.clone(),
                        error: e.to_string(),
                    }
                })?;

            // ----- register each export -----
            for component in &dep_manifest.components.exports {
                if let Some(prior) = export_origin.get(component) {
                    return Err(ResolveError::DuplicateExport {
                        component: component.clone(),
                        package_a: prior.clone(),
                        package_b: dep_manifest.package.name.clone(),
                    });
                }
                export_origin.insert(component.clone(), dep_manifest.package.name.clone());
                table.insert(
                    component.clone(),
                    Resolution::Component {
                        package: dep_manifest.package.name.clone(),
                        package_path: dep_root.clone(),
                        component: component.clone(),
                    },
                );
            }
        }
    }

    // ----------------------------------------------------------------
    // Step 3: build the kernel set.
    // ----------------------------------------------------------------
    let kernel: HashSet<&'static str> = KERNEL_PRIMITIVES.iter().copied().collect();

    Ok(Resolver { table, kernel })
}

/// Find a dependency by name in the given search paths.
///
/// Returns the first directory found.  We try the `mosaic-pkg-<name>`
/// form first (UI29 §4.1 convention) and fall back to the literal
/// `<name>`.
fn locate_dependency(dep_name: &str, search_paths: &[PathBuf]) -> Result<PathBuf, ResolveError> {
    // Candidate directory names to try, in order.  If the dep is already
    // named with the `mosaic-pkg-` prefix the two candidates collapse to
    // one — we dedup via a tiny inline check rather than a set.
    let candidates: Vec<String> = if dep_name.starts_with("mosaic-pkg-") {
        vec![dep_name.to_string()]
    } else {
        vec![format!("mosaic-pkg-{dep_name}"), dep_name.to_string()]
    };

    for path in search_paths {
        for candidate in &candidates {
            let candidate_path = path.join(candidate);
            // The manifest must exist for a directory to count as a package.
            // (A bare directory with no manifest is not a package; finding
            // such a directory and then immediately failing on the manifest
            // read would produce a confusing error.)
            if candidate_path.join("mosaic-package.toml").is_file() {
                return Ok(candidate_path);
            }
        }
    }

    Err(ResolveError::DependencyNotFound {
        package: dep_name.to_string(),
        searched: search_paths.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Layout package-reference inliner
// ---------------------------------------------------------------------------

/// Errors produced while substituting `pkg::P::C` layout references.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutResolveError {
    PackageNotFound {
        package: String,
    },
    ComponentNotExported {
        package: String,
        component: String,
        available: Vec<String>,
    },
    ComponentSourceMissing {
        package: String,
        component: String,
        file: PathBuf,
    },
    CircularPackageReference {
        cycle: Vec<(String, String)>,
    },
    ManifestParseError {
        package: String,
        path: PathBuf,
        detail: String,
    },
    NestedCompileError {
        package: String,
        component: String,
        detail: String,
    },
    IoError {
        package: String,
        component: String,
        path: PathBuf,
        detail: String,
    },
    SymlinkEscape {
        package: String,
        component: String,
        attempted: PathBuf,
        canonical: PathBuf,
    },
}

impl std::fmt::Display for LayoutResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageNotFound { package } => {
                write!(f, "package `{package}` was not found in the search path")
            }
            Self::ComponentNotExported {
                package,
                component,
                available,
            } => write!(
                f,
                "package `{package}` does not export `{component}`; available exports: {available:?}"
            ),
            Self::ComponentSourceMissing {
                package,
                component,
                file,
            } => write!(
                f,
                "package `{package}` component `{component}` is missing {}",
                file.display()
            ),
            Self::CircularPackageReference { cycle } => {
                write!(f, "circular package reference: {cycle:?}")
            }
            Self::ManifestParseError {
                package,
                path,
                detail,
            } => write!(
                f,
                "package `{package}` manifest {} could not be parsed: {detail}",
                path.display()
            ),
            Self::NestedCompileError {
                package,
                component,
                detail,
            } => write!(
                f,
                "package `{package}` component `{component}` failed to compile: {detail}"
            ),
            Self::IoError {
                package,
                component,
                path,
                detail,
            } => write!(
                f,
                "io error while resolving `{package}`/`{component}` at {}: {detail}",
                path.display()
            ),
            Self::SymlinkEscape {
                package,
                component,
                attempted,
                canonical,
            } => write!(
                f,
                "package `{package}` component `{component}` source {} resolves outside the package root at {}",
                attempted.display(),
                canonical.display()
            ),
        }
    }
}

impl std::error::Error for LayoutResolveError {}

/// Substitutes `pkg::P::C` nodes in a compiled [`LayoutDef`].
///
/// The inliner is intentionally source-level: it compiles the referenced
/// package component's `.mil`/`.mll`, rewires call-site slot/event bindings,
/// qualifies sibling component references, and leaves backend emitters with a
/// layout tree containing no qualified tags.
pub struct LayoutPackageResolver {
    search_paths: Vec<PathBuf>,
    cache: Mutex<HashMap<(String, String), LayoutDef>>,
}

impl LayoutPackageResolver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Mutate `layout` in place, replacing every `pkg::P::C` node with the
    /// referenced component's compiled layout tree.
    pub fn resolve(&self, layout: &mut LayoutDef) -> Result<(), LayoutResolveError> {
        let mut visiting: Vec<(String, String)> = Vec::new();
        self.resolve_node(&mut layout.root, &mut visiting)
    }

    fn resolve_node(
        &self,
        node: &mut LayoutNode,
        visiting: &mut Vec<(String, String)>,
    ) -> Result<(), LayoutResolveError> {
        let pushed: Vec<(String, String)> = {
            let mut pushed = Vec::new();
            while let Some((pkg, comp)) = node
                .package_ref()
                .map(|(p, c)| (p.to_string(), c.to_string()))
            {
                if visiting.iter().any(|p| p == &(pkg.clone(), comp.clone())) {
                    let mut cycle = visiting.clone();
                    cycle.push((pkg, comp));
                    return Err(LayoutResolveError::CircularPackageReference { cycle });
                }
                visiting.push((pkg.clone(), comp.clone()));
                pushed.push((pkg.clone(), comp.clone()));
                let resolved = self.resolve_component(&pkg, &comp)?;
                self.substitute(node, resolved, &pkg)?;
            }
            pushed
        };

        let mut i = 0;
        while i < node.children.len() {
            self.resolve_node(&mut node.children[i], visiting)?;
            i += 1;
        }

        for _ in &pushed {
            visiting.pop();
        }
        Ok(())
    }

    fn substitute(
        &self,
        target: &mut LayoutNode,
        resolved: LayoutDef,
        pkg: &str,
    ) -> Result<(), LayoutResolveError> {
        let call_props = std::mem::take(&mut target.props);
        let consumer_part = target.part_name.take();
        let _ = std::mem::take(&mut target.children);

        target.tag = resolved.root.tag;
        target.part_name = consumer_part.or(resolved.root.part_name);
        target.props = resolved.root.props;
        target.children = resolved.root.children;

        let bindings = build_binding_map(&call_props);
        rewrite_bindings(target, &bindings);

        let exports = self.package_exports(pkg)?;
        qualify_local_refs(target, pkg, &exports);

        Ok(())
    }

    fn resolve_component(&self, pkg: &str, comp: &str) -> Result<LayoutDef, LayoutResolveError> {
        let key = (pkg.to_string(), comp.to_string());
        if let Some(cached) = self.cache.lock().unwrap().get(&key).cloned() {
            return Ok(cached);
        }

        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;

        if !manifest.components.exports.iter().any(|e| e == comp) {
            return Err(LayoutResolveError::ComponentNotExported {
                package: pkg.to_string(),
                component: comp.to_string(),
                available: manifest.components.exports.clone(),
            });
        }

        let canon_root =
            std::fs::canonicalize(&pkg_root).map_err(|e| LayoutResolveError::IoError {
                package: pkg.to_string(),
                component: comp.to_string(),
                path: pkg_root.clone(),
                detail: format!("canonicalize package root failed: {e}"),
            })?;

        let src_dir = pkg_root.join("src");
        let mil_path = src_dir.join(format!("{comp}.mil"));
        let mll_path = src_dir.join(format!("{comp}.mll"));
        verify_inside_package(pkg, comp, &mil_path, &canon_root)?;
        verify_inside_package(pkg, comp, &mll_path, &canon_root)?;

        let mil_src = read_component_source(pkg, comp, &mil_path)?;
        let mll_src = read_component_source(pkg, comp, &mll_path)?;

        let mosmodel_out = mosmodel_compiler::compile(&mil_src).map_err(|errs| {
            LayoutResolveError::NestedCompileError {
                package: pkg.to_string(),
                component: comp.to_string(),
                detail: format!("{errs:?}"),
            }
        })?;
        let layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
            .map_err(|errs| LayoutResolveError::NestedCompileError {
                package: pkg.to_string(),
                component: comp.to_string(),
                detail: format!("{errs:?}"),
            })?;

        let def = layout_out.def;
        self.cache.lock().unwrap().insert(key, def.clone());
        Ok(def)
    }

    fn package_exports(&self, pkg: &str) -> Result<Vec<String>, LayoutResolveError> {
        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;
        Ok(manifest.components.exports)
    }

    fn locate_package(&self, pkg: &str) -> Result<PathBuf, LayoutResolveError> {
        for search_root in &self.search_paths {
            let candidate_dirs = match std::fs::read_dir(search_root) {
                Ok(it) => it,
                Err(_) => continue,
            };
            for entry in candidate_dirs.flatten() {
                let candidate = entry.path();
                let manifest_path = candidate.join("mosaic-package.toml");
                if !manifest_path.exists() {
                    continue;
                }
                if let Ok(src) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(m) = mosaic_package_manifest::parse(&src) {
                        if m.package.name == pkg {
                            return Ok(candidate);
                        }
                    }
                }
            }
        }
        Err(LayoutResolveError::PackageNotFound {
            package: pkg.to_string(),
        })
    }

    fn read_manifest(
        &self,
        pkg: &str,
        pkg_root: &Path,
    ) -> Result<MosaicPackage, LayoutResolveError> {
        let manifest_path = pkg_root.join("mosaic-package.toml");
        let src =
            std::fs::read_to_string(&manifest_path).map_err(|e| LayoutResolveError::IoError {
                package: pkg.to_string(),
                component: String::new(),
                path: manifest_path.clone(),
                detail: e.to_string(),
            })?;
        mosaic_package_manifest::parse(&src).map_err(|e| LayoutResolveError::ManifestParseError {
            package: pkg.to_string(),
            path: manifest_path,
            detail: format!("{e:?}"),
        })
    }
}

fn read_component_source(pkg: &str, comp: &str, path: &Path) -> Result<String, LayoutResolveError> {
    std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LayoutResolveError::ComponentSourceMissing {
                package: pkg.to_string(),
                component: comp.to_string(),
                file: path.to_path_buf(),
            }
        } else {
            LayoutResolveError::IoError {
                package: pkg.to_string(),
                component: comp.to_string(),
                path: path.to_path_buf(),
                detail: e.to_string(),
            }
        }
    })
}

fn build_binding_map(call_props: &[LayoutProp]) -> HashMap<String, LayoutPropValue> {
    let mut bindings = HashMap::with_capacity(call_props.len() * 2);
    for prop in call_props {
        bindings.insert(prop.name.clone(), prop.value.clone());
        let camel = to_camel_case_first_lower(&prop.name);
        if camel != prop.name {
            bindings.insert(camel, prop.value.clone());
        }
    }
    bindings
}

fn rewrite_bindings(node: &mut LayoutNode, bindings: &HashMap<String, LayoutPropValue>) {
    for prop in &mut node.props {
        match &prop.value {
            LayoutPropValue::SlotRef(name) => {
                if let Some(value) = bindings.get(name) {
                    prop.value = value.clone();
                }
            }
            LayoutPropValue::EmitRef(name) => {
                if let Some(value) = bindings.get(name) {
                    prop.value = value.clone();
                }
            }
            LayoutPropValue::Expr(text) => {
                let rewritten = rewrite_expression_bindings(text, bindings);
                if rewritten != *text {
                    prop.value = LayoutPropValue::Expr(rewritten);
                }
            }
            _ => {}
        }
    }
    for child in &mut node.children {
        rewrite_bindings(child, bindings);
    }
}

fn rewrite_expression_bindings(expr: &str, bindings: &HashMap<String, LayoutPropValue>) -> String {
    let mut out = String::with_capacity(expr.len());
    let bytes = expr.as_bytes();
    let mut i = 0;
    let mut prev_non_ws: Option<u8> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            let start = i;
            i += 1;
            let mut escaped = false;
            while i < bytes.len() {
                let c = bytes[i];
                i += 1;
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == b'\\' {
                    escaped = true;
                    continue;
                }
                if c == b'"' {
                    break;
                }
            }
            out.push_str(&expr[start..i]);
            prev_non_ws = Some(b'"');
            continue;
        }

        if is_identifier_start(b) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_identifier_continue(bytes[i]) {
                i += 1;
            }
            let ident = &expr[start..i];
            if prev_non_ws != Some(b'.') {
                if let Some(value) = bindings.get(ident) {
                    out.push_str(&layout_prop_value_as_expression(value));
                } else {
                    out.push_str(ident);
                }
            } else {
                out.push_str(ident);
            }
            prev_non_ws = Some(bytes[i - 1]);
            continue;
        }

        out.push(b as char);
        if !b.is_ascii_whitespace() {
            prev_non_ws = Some(b);
        }
        i += 1;
    }

    out
}

fn layout_prop_value_as_expression(value: &LayoutPropValue) -> String {
    match value {
        LayoutPropValue::SlotRef(name) => to_camel_case_first_lower(name),
        LayoutPropValue::EmitRef(name) => to_camel_case_first_lower(name),
        LayoutPropValue::Keyword(name) => to_camel_case_first_lower(name),
        LayoutPropValue::Number(n) => n.to_string(),
        LayoutPropValue::String(s) => js_string_literal(s),
        LayoutPropValue::Expr(text) => format!("( {text} )"),
    }
}

fn js_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn is_identifier_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn is_identifier_continue(b: u8) -> bool {
    is_identifier_start(b) || b.is_ascii_digit()
}

fn to_camel_case_first_lower(s: &str) -> String {
    let mut out = String::new();
    let mut cap_next = false;
    let mut first = true;
    for ch in s.chars() {
        if ch == '-' {
            cap_next = true;
            continue;
        }
        if first {
            out.push(ch.to_ascii_lowercase());
            first = false;
        } else if cap_next {
            out.push(ch.to_ascii_uppercase());
            cap_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn qualify_local_refs(node: &mut LayoutNode, pkg: &str, pkg_exports: &[String]) {
    if node.package_ref().is_none() && pkg_exports.iter().any(|e| e == &node.tag) {
        node.tag = format!("pkg::{}::{}", pkg, node.tag);
    }
    for child in &mut node.children {
        qualify_local_refs(child, pkg, pkg_exports);
    }
}

fn verify_inside_package(
    pkg: &str,
    comp: &str,
    attempted: &Path,
    canon_root: &Path,
) -> Result<(), LayoutResolveError> {
    let canon = match std::fs::canonicalize(attempted) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(LayoutResolveError::IoError {
                package: pkg.to_string(),
                component: comp.to_string(),
                path: attempted.to_path_buf(),
                detail: format!("canonicalize failed: {e}"),
            });
        }
    };
    if !canon.starts_with(canon_root) {
        return Err(LayoutResolveError::SymlinkEscape {
            package: pkg.to_string(),
            component: comp.to_string(),
            attempted: attempted.to_path_buf(),
            canonical: canon,
        });
    }
    Ok(())
}

/// Return the first still-qualified package reference tag in `root`, if any.
pub fn first_qualified_tag(root: &LayoutNode) -> Option<&str> {
    if root.package_ref().is_some() {
        return Some(&root.tag);
    }
    for child in &root.children {
        if let Some(tag) = first_qualified_tag(child) {
            return Some(tag);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ---- tiny test helpers ----

    /// Write a `mosaic-package.toml` into `dir` with the given fields.
    fn write_manifest(dir: &Path, name: &str, exports: &[&str], deps: &[(&str, &str)]) {
        let deps_block: String = deps
            .iter()
            .map(|(k, v)| format!("{k} = \"{v}\"\n"))
            .collect();
        let exports_list = exports
            .iter()
            .map(|e| format!("\"{e}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let toml = format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
description = "test package"
license = "MIT"

[components]
exports = [{exports_list}]

[dependencies]
{deps_block}
[kernel]
version = "1"
"#
        );
        fs::write(dir.join("mosaic-package.toml"), toml).unwrap();
    }

    /// Create a package directory at `parent/dirname` with a manifest,
    /// returning its path.
    fn make_pkg(parent: &Path, dirname: &str, manifest_name: &str, exports: &[&str]) -> PathBuf {
        let path = parent.join(dirname);
        fs::create_dir_all(&path).unwrap();
        write_manifest(&path, manifest_name, exports, &[]);
        path
    }

    fn write_component(pkg_root: &Path, name: &str, mil: &str, mll: &str) {
        let src = pkg_root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join(format!("{name}.mil")), mil).unwrap();
        fs::write(src.join(format!("{name}.mll")), mll).unwrap();
    }

    fn consumer_layout(source: &str) -> LayoutDef {
        let ast = moslayout_compiler::parse_layout(source).expect("parse layout");
        moslayout_compiler::analyze(&ast).expect("analyze layout")
    }

    // ---- Test 1: empty package, no manifest, no deps ----

    #[test]
    fn empty_package_no_manifest_kernel_only() {
        let tmp = TempDir::new().unwrap();
        // No manifest written — package_root is just an empty dir.
        let r = build(tmp.path(), &[]).expect("manifest-less must succeed");
        assert!(matches!(r.resolve("Box"), Some(Resolution::Kernel)));
        assert!(matches!(r.resolve("HostScroll"), Some(Resolution::Kernel)));
        assert!(r.resolve("Grid").is_none());
        assert_eq!(r.component_count(), 0);
    }

    // ---- Test 2: manifest with no dependencies ----

    #[test]
    fn manifest_no_dependencies() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "mosaic-pkg-user", &[], &[]);
        let r = build(tmp.path(), &[]).expect("no-deps must succeed");
        assert!(matches!(r.resolve("Row"), Some(Resolution::Kernel)));
        assert!(r.resolve("Whatever").is_none());
        assert_eq!(r.component_count(), 0);
    }

    // ---- Test 3: single dep, one export ----

    #[test]
    fn single_dep_one_export() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0")],
        );

        let r = build(&user, std::slice::from_ref(&pkgs)).expect("build ok");
        match r.resolve("Grid") {
            Some(Resolution::Component {
                package, component, ..
            }) => {
                assert_eq!(package, "mosaic-pkg-grid");
                assert_eq!(component, "Grid");
            }
            other => panic!("expected Component, got {other:?}"),
        }
    }

    // ---- Test 4: dep with multiple exports ----

    #[test]
    fn dep_multiple_exports() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(
            &pkgs,
            "mosaic-pkg-grid",
            "mosaic-pkg-grid",
            &["Grid", "Cell", "ColumnDef"],
        );

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0")],
        );

        let r = build(&user, &[pkgs]).expect("build ok");
        assert!(r.knows("Grid"));
        assert!(r.knows("Cell"));
        assert!(r.knows("ColumnDef"));
        assert_eq!(r.component_count(), 3);
    }

    // ---- Test 5: two deps, no collision ----

    #[test]
    fn two_deps_no_collision() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);
        make_pkg(
            &pkgs,
            "mosaic-pkg-tabs",
            "mosaic-pkg-tabs",
            &["Tabs", "Tab"],
        );

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0"), ("mosaic-pkg-tabs", "0.1.0")],
        );

        let r = build(&user, &[pkgs]).expect("build ok");
        assert!(r.knows("Grid"));
        assert!(r.knows("Tabs"));
        assert!(r.knows("Tab"));
        assert_eq!(r.component_count(), 3);
    }

    // ---- Test 6: two deps with colliding exports ----

    #[test]
    fn duplicate_export_errors() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);
        make_pkg(&pkgs, "mosaic-pkg-other", "mosaic-pkg-other", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0"), ("mosaic-pkg-other", "0.1.0")],
        );

        let err = build(&user, &[pkgs]).expect_err("must collide");
        match err {
            ResolveError::DuplicateExport { component, .. } => {
                assert_eq!(component, "Grid");
            }
            other => panic!("expected DuplicateExport, got {other:?}"),
        }
    }

    // ---- Test 7: dep not in any search path ----

    #[test]
    fn dependency_not_found_errors() {
        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-ghost", "0.1.0")],
        );

        // Empty search path → can't possibly find it.
        let err = build(&user, &[]).expect_err("must fail");
        assert!(
            matches!(err, ResolveError::DependencyNotFound { ref package, .. } if package == "mosaic-pkg-ghost")
        );
    }

    // ---- Test 8: dep with malformed manifest ----

    #[test]
    fn bad_dependency_manifest_errors() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        let bad = pkgs.join("mosaic-pkg-bad");
        fs::create_dir_all(&bad).unwrap();
        // Write a manifest that is syntactically valid TOML but is missing
        // required fields, so the manifest parser rejects it.
        fs::write(bad.join("mosaic-package.toml"), "garbage = true\n").unwrap();

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-bad", "0.1.0")],
        );

        let err = build(&user, &[pkgs]).expect_err("must fail");
        assert!(
            matches!(err, ResolveError::BadDependencyManifest { ref package, .. } if package == "mosaic-pkg-bad")
        );
    }

    // ---- Test 9: Resolver::knows ----

    #[test]
    fn knows_returns_true_for_kernel_and_component_false_for_unknown() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0")],
        );

        let r = build(&user, &[pkgs]).expect("ok");
        assert!(r.knows("Box"));
        assert!(r.knows("Grid"));
        assert!(!r.knows("Floof"));
    }

    // ---- Test 10: resolve returns appropriate variants ----

    #[test]
    fn resolve_returns_correct_variants() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0")],
        );

        let r = build(&user, &[pkgs]).expect("ok");

        // Kernel:
        assert!(matches!(r.resolve("If"), Some(Resolution::Kernel)));
        // Component:
        assert!(matches!(
            r.resolve("Grid"),
            Some(Resolution::Component { .. })
        ));
        // Unknown:
        assert!(r.resolve("ZZZ").is_none());
    }

    // ---- Test 11: kernel set contains all UI29 §2.1 primitives ----

    #[test]
    fn kernel_set_covers_ui29_section_2_1() {
        // The twenty-six primitives — fifteen from UI29 §2.1, plus
        // HostDialog added in UI29-1 (#3846), plus HostCheckbox and
        // HostRadio added in UI29-2 (#3978), plus HostLink,
        // HostTooltip, and HostNumberInput added in UI29-4, plus the
        // five UI31 HostTable structural sub-tags (HostTableColGroup,
        // HostTableHead, HostTableBody, HostTableFoot, Col).
        let expected_26 = [
            "Box",
            "Row",
            "Column",
            "Stack",
            "Text",
            "Image",
            "Spacer",
            "Divider",
            "Icon",
            "If",
            "For",
            "HostInput",
            "HostButton",
            "HostTable",
            "HostScroll",
            "HostDialog",
            "HostCheckbox",
            "HostRadio",
            "HostLink",
            "HostTooltip",
            "HostNumberInput",
            "HostTableColGroup",
            "HostTableHead",
            "HostTableBody",
            "HostTableFoot",
            "Col",
        ];
        for name in &expected_26 {
            assert!(
                KERNEL_PRIMITIVES.contains(name),
                "kernel must include `{name}`"
            );
        }
        // And `Else` because the parser treats it as its own tag.
        assert!(KERNEL_PRIMITIVES.contains(&"Else"));
        // Sanity: no duplicates.
        let mut sorted: Vec<&&str> = KERNEL_PRIMITIVES.iter().collect();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), KERNEL_PRIMITIVES.len(), "no duplicates");
    }

    // ---- Test 11b: UI35 drag-and-drop family ----

    /// The kernel gained no drag primitive until UI35, which is why a board's
    /// defining gesture was inexpressible. Pinned separately from the UI29/UI31
    /// roster so a refactor that drops one of the pair is caught here, and so
    /// `resolve_tag` keeps classifying them as kernel rather than sending them
    /// down the package-reference path.
    #[test]
    fn kernel_set_covers_ui35_drag_and_drop() {
        for name in ["HostDraggable", "HostDropTarget"] {
            assert!(
                KERNEL_PRIMITIVES.contains(&name),
                "UI35 kernel must include `{name}`"
            );
        }
    }

    // ---- Test 12: package_path is absolute ----

    #[test]
    fn package_path_is_absolute() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(
            &user,
            "mosaic-pkg-user",
            &[],
            &[("mosaic-pkg-grid", "0.1.0")],
        );

        // Pass a *relative* search path on purpose — we want to verify
        // the resolver canonicalizes it before storing.  We do that by
        // changing into tmp and then using a relative path; if that's
        // unreliable in parallel tests, just pass the absolute path
        // and rely on canonicalize() to resolve `..` / symlinks.
        let r = build(&user, std::slice::from_ref(&pkgs)).expect("ok");
        match r.resolve("Grid") {
            Some(Resolution::Component { package_path, .. }) => {
                assert!(
                    package_path.is_absolute(),
                    "package_path must be absolute, got {package_path:?}"
                );
                // Should end with the package directory name.
                assert!(package_path.ends_with("mosaic-pkg-grid"));
            }
            other => panic!("expected Component, got {other:?}"),
        }
    }

    // ---- Extra test: dep dir without `mosaic-pkg-` prefix is also found ----

    #[test]
    fn dep_with_literal_name_match() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        // Directory is named exactly the same as the dep key.
        make_pkg(&pkgs, "widgets", "widgets", &["Spinner"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(&user, "mosaic-pkg-user", &[], &[("widgets", "0.1.0")]);

        let r = build(&user, &[pkgs]).expect("ok");
        assert!(matches!(
            r.resolve("Spinner"),
            Some(Resolution::Component { .. })
        ));
    }

    #[test]
    fn layout_inliner_substitutes_pkg_refs_and_rewrites_bindings() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let mini = make_pkg(&pkgs, "mosaic-pkg-mini", "mosaic-pkg-mini", &["Greet"]);
        write_component(
            &mini,
            "Greet",
            r#"component Greet { slot label : text ; emit onClick ; }"#,
            r#"layout Greet {
  HostButton [ greet-button ] (
    label : slot: label ,
    onClick : emit: onClick
  )
}"#,
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout Demo {
  pkg::mosaic-pkg-mini::Greet (
    label : slot: outer-label ,
    onClick : emit: outer-click
  )
}"#,
        );

        resolver.resolve(&mut layout).expect("layout resolves");

        assert!(first_qualified_tag(&layout.root).is_none());
        assert_eq!(layout.root.tag, "HostButton");
        assert_eq!(layout.root.part_name.as_deref(), Some("greet-button"));
        assert!(
            layout.root.props.iter().any(|prop| {
                prop.name == "label"
                    && prop.value == LayoutPropValue::SlotRef("outer-label".to_string())
            }),
            "slot binding should be rewritten to the consumer slot"
        );
        assert!(
            layout.root.props.iter().any(|prop| {
                prop.name == "onClick"
                    && prop.value == LayoutPropValue::EmitRef("outer-click".to_string())
            }),
            "emit binding should be rewritten to the consumer emit"
        );
    }

    #[test]
    fn layout_inliner_rewrites_expression_binding_identifiers() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let mini = make_pkg(
            &pkgs,
            "mosaic-pkg-selector",
            "mosaic-pkg-selector",
            &["Selector"],
        );
        write_component(
            &mini,
            "Selector",
            r#"component Selector {
  slot selected-index : number ;
  slot label : text ;
}"#,
            r#"layout Selector {
  If ( when: selectedIndex == 0 ) {
    Text [ selected-label ] ( slot: label )
  }
}"#,
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout Demo {
  pkg::mosaic-pkg-selector::Selector (
    selected-index : slot: browser-selected-index ,
    label : slot: outer-label
  )
}"#,
        );

        resolver.resolve(&mut layout).expect("layout resolves");

        assert_eq!(layout.root.tag, "If");
        assert!(
            layout.root.props.iter().any(|prop| {
                prop.name == "when"
                    && prop.value == LayoutPropValue::Expr("browserSelectedIndex == 0".to_string())
            }),
            "expression binding should be rewritten to the consumer slot: {:#?}",
            layout.root.props
        );
        assert!(
            layout.root.children[0].props.iter().any(|prop| {
                prop.name == "slot"
                    && prop.value == LayoutPropValue::SlotRef("outer-label".to_string())
            }),
            "nested direct slot binding should still be rewritten"
        );
    }

    #[test]
    fn expression_binding_rewrite_skips_member_names_and_strings() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "selectedIndex".to_string(),
            LayoutPropValue::SlotRef("browser-selected-index".to_string()),
        );

        assert_eq!(
            rewrite_expression_bindings(
                r#"i == selectedIndex && item.selectedIndex != "selectedIndex""#,
                &bindings
            ),
            r#"i == browserSelectedIndex && item.selectedIndex != "selectedIndex""#
        );
    }

    #[test]
    fn layout_inliner_detects_package_reference_cycles() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let cyclic = make_pkg(&pkgs, "mosaic-pkg-cyclic", "mosaic-pkg-cyclic", &["A", "B"]);
        write_component(&cyclic, "A", r#"component A { }"#, r#"layout A { B }"#);
        write_component(&cyclic, "B", r#"component B { }"#, r#"layout B { A }"#);

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(r#"layout Demo { pkg::mosaic-pkg-cyclic::A { } }"#);

        let err = resolver.resolve(&mut layout).unwrap_err();
        assert!(
            matches!(err, LayoutResolveError::CircularPackageReference { .. }),
            "expected cycle error, got {err:?}"
        );
    }
}
