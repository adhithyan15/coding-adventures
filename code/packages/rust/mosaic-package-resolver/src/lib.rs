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
//! 2. For each `(dep_name, requirement)` in the manifest's `[dependencies]`,
//!    in name order:
//!    a. Search each path in `search_paths` for a child directory.  We
//!    try `mosaic-pkg-<dep_name>` first (the UI29 §4.1 convention),
//!    then fall back to the literal `<dep_name>`.
//!    b. Read that dep's `mosaic-package.toml` via
//!    `mosaic_package_manifest::parse_path`.
//!    c. Check its `[package].version` against `requirement` → else
//!    `VersionMismatch` (see "Version requirements" below).
//!    d. For each entry in its `[components].exports`, register a
//!    `Resolution::Component { … }` keyed by the component name.
//!    e. If two dependencies export the same name → `DuplicateExport`.
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
//! ## Version requirements (mosaic-package-publishing-v1 §7.4)
//!
//! A requirement is a bare version (caret: `"0.16.0"` accepts
//! `>=0.16.0, <0.17.0`) or `=version` (exact); the manifest parser has
//! already refused every other form. Both locators compare it with the
//! `[package].version` of the package they actually found, and a miss is
//! an error, never a warning:
//!
//! ```text
//!   consumer                         dependency on disk
//!   mosaic-pkg-toolkit = "0.11.0"    mosaic-pkg-toolkit 0.16.0
//!            │                                 │
//!            └──── >=0.11.0, <0.12.0 ∌ 0.16.0 ─┘ → VersionMismatch
//! ```
//!
//! A requirement that does not describe what the build compiles against
//! is a comment that looks like a promise. Failing loudly is what keeps
//! the strings truthful.
//!
//! **One version per package per build.** Lookup is by name and returns
//! one directory, so a build never holds two versions of one package;
//! every requirer is checked against that one.
//!
//! - [`build`] checks the consumer's direct dependencies.
//! - [`LayoutPackageResolver`] checks every package it inlines from against
//!   that package's own `[dependencies]`, so a requirement anywhere in the
//!   `pkg::` graph is checked. [`LayoutPackageResolver::check_dependencies`]
//!   checks the consuming package itself, which the inliner never sees.
//!
//! ## Error surface
//!
//! ```text
//!     build(...)
//!         │
//!         ├── DependencyNotFound    ← name not in any search path
//!         ├── BadDependencyManifest ← dep's TOML didn't parse
//!         ├── VersionMismatch       ← dep's version fails the requirement
//!         ├── DuplicateExport       ← two deps export same name
//!         └── Io                    ← read_dir / canonicalize failed
//! ```
//!
//! Resolving a *tag* never errors — unknown is `None`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mosaic_package_manifest::{MosaicPackage, Version, VersionReq};
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp, LayoutPropValue};
use mosmodel_compiler::SlotDefault;

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
    // UI29-2's HostCheckbox/HostRadio + UI29-3's HostSlider + UI29-4's
    // HostLink/HostTooltip/HostNumberInput + UI38's HostSwitch).
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
    // HostSlider preserves the native adjustable role, announced
    // range/value, keyboard increments, drag behavior, and platform
    // thumb/track rendering that layout composition cannot reproduce.
    // HostSwitch similarly preserves the native switch role, announced
    // on/off state, keyboard/touch behavior, and platform rendering.
    "HostInput",
    "HostButton",
    "HostTable",
    "HostScroll",
    "HostDialog",
    "HostSurface",
    "HostCheckbox",
    "HostRadio",
    "HostLink",
    "HostTooltip",
    "HostNumberInput",
    "HostSlider",
    "HostSwitch",
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
    // UI39 — the kernel drawing primitive. Four parametric shape kinds
    // (`circle`/`line`/`curve`/`arc`) rather than an arbitrary SVG path
    // string or a structured path-command list — see UI39 §3.1/§2 for the
    // inclusion-criteria reasoning (every platform has a native vector-
    // drawing surface; no CSS-only composition survives native lowering).
    "Path",
    // #13176 — `HostProgressRing` preserves the platform's native
    // determinate progress-ring role (`progressbar`, announced
    // `aria-valuenow`/value-changed live region), the real
    // accessibility surface a `Box`+`Box` CSS conic-gradient trick
    // can't provide on native backends at all — those backends
    // silently ignore a slot-bound `background` on a plain `Box`.
    "HostProgressRing",
    // UI29-6 (#15481) — `HostNavigationSplit`, the adaptive navigation
    // container. A pane beside a detail area is two `Column`s underneath,
    // and four layouts build it that way today with four different answers
    // to pane width. What composition cannot supply is the platform's own
    // collapse (`NavigationSplitView`, WinUI `NavigationView`,
    // `NavigationSuiteScaffold` adapt inside their own layout pass, which
    // UI48 §5.6 rules out routing through an event) and the pane landmark a
    // screen reader navigates by. Exactly two children, pane then detail.
    "HostNavigationSplit",
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

/// A dependency's version does not satisfy the requirement a package
/// declared for it (mosaic-package-publishing-v1 §7.4).
///
/// Shared by both locators' error types, so the diagnostic reads the same
/// whichever one found the package:
///
/// ```text
/// task-app requires mosaic-pkg-toolkit "0.15.0" (>=0.15.0, <0.16.0), but found
/// mosaic-pkg-toolkit 0.16.0 at …/mosaic-pkg-toolkit; help: update the
/// requirement in …/task-app/mosaic-package.toml
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMismatch {
    /// The requiring package's `[package].name`.
    pub dependent: String,
    /// The requiring package's `mosaic-package.toml`, where the fix goes.
    pub dependent_manifest: PathBuf,
    /// The dependency's name, as written under `[dependencies]`.
    pub dependency: String,
    /// The requirement as written there.
    pub requirement: VersionReq,
    /// The `[package].version` of the package the locator found.
    pub found: String,
    /// Where the locator found it.
    pub found_at: PathBuf,
}

impl std::fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} requires {} \"{}\" ({}), but found {} {} at {}; help: update the requirement in {}",
            self.dependent,
            self.dependency,
            self.requirement,
            self.requirement.range(),
            self.dependency,
            self.found,
            self.found_at.display(),
            self.dependent_manifest.display()
        )
    }
}

/// Why [`check_requirement`] refused a dependency.
enum RequirementFailure {
    /// The found version does not satisfy the requirement.
    Mismatch(Box<VersionMismatch>),
    /// A requirement or version string did not parse. Manifests from
    /// `mosaic_package_manifest::parse` are already validated, so this is
    /// reachable only through a hand-built [`MosaicPackage`]; each caller
    /// files it under its own "bad manifest" variant.
    Invalid(String),
}

/// Compare one `[dependencies]` entry with the package a locator found.
///
/// The caret and exact rules themselves live in
/// [`mosaic_package_manifest::VersionReq::matches`]; this only gathers what
/// a diagnostic needs to name both ends of the edge.
fn check_requirement(
    dependent: &MosaicPackage,
    dependent_manifest: &Path,
    dependency: &str,
    requirement: &str,
    found: &MosaicPackage,
    found_at: &Path,
) -> Result<(), RequirementFailure> {
    let parsed_requirement = VersionReq::parse(requirement).map_err(|e| {
        RequirementFailure::Invalid(format!(
            "{} declares an unusable requirement {dependency} = \"{requirement}\" ({e:?})",
            dependent.package.name
        ))
    })?;
    let found_version = Version::parse(&found.package.version).ok_or_else(|| {
        RequirementFailure::Invalid(format!(
            "{dependency} declares an invalid version `{}`",
            found.package.version
        ))
    })?;
    if parsed_requirement.matches(&found_version) {
        return Ok(());
    }
    Err(RequirementFailure::Mismatch(Box::new(VersionMismatch {
        dependent: dependent.package.name.clone(),
        dependent_manifest: dependent_manifest.to_path_buf(),
        dependency: dependency.to_string(),
        requirement: parsed_requirement,
        found: found.package.version.clone(),
        found_at: found_at.to_path_buf(),
    })))
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
    /// A dependency was found, but its `[package].version` does not satisfy
    /// the requirement the consumer declared for it. Boxed: it is the
    /// largest variant, and every `Result` would otherwise carry its size.
    VersionMismatch(Box<VersionMismatch>),
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
            Self::VersionMismatch(mismatch) => write!(f, "{mismatch}"),
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
        // Name order, so the first error reported is the same on every run
        // (a `HashMap` iterates in a different order each process).
        let mut dependencies: Vec<(&String, &String)> = manifest.dependencies.iter().collect();
        dependencies.sort();
        for (dep_name, requirement) in dependencies {
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

            // ----- check the version it declares (§7.4) -----
            check_requirement(
                &manifest,
                &manifest_path,
                dep_name,
                requirement,
                &dep_manifest,
                &dep_root,
            )
            .map_err(|failure| match failure {
                RequirementFailure::Mismatch(mismatch) => ResolveError::VersionMismatch(mismatch),
                RequirementFailure::Invalid(error) => ResolveError::BadDependencyManifest {
                    package: dep_name.clone(),
                    error,
                },
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
    InlineChildrenNotAccepted {
        package: String,
        component: String,
        child_count: usize,
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
    /// A `pkg::P::C` call site left a required slot of `C` unbound
    /// (UI34 §7, UI13 §1: a slot with no default is required).
    ///
    /// ```text
    /// // DraftEditor.mil                 // JournalApp.mll
    /// slot body-placeholder : text ;     pkg::mosaic-pkg-draft-editor::DraftEditor (
    ///                                       body-value: slot: body )   ← no body-placeholder
    /// ```
    ///
    /// Before this check existed, the unbound `slot: body-placeholder` was
    /// left in the inlined tree, where it named a slot of the *consumer*.
    /// Depending on luck that was a misleading `UnknownSlot` against the
    /// consumer, a silent capture of an unrelated same-named consumer slot,
    /// or an undeclared identifier in generated code
    /// (mosaic-unbound-slot-semantics-v1 §1.3, probes P1, P3 and P12).
    SlotBindingMissing {
        /// The component whose layout holds the call site: the layout being
        /// resolved (`JournalApp`), or `pkg::P::C` for a reference nested
        /// inside another package component's own layout.
        consumer: String,
        /// The package that declares the slot.
        package: String,
        /// The component that declares the slot.
        component: String,
        /// The unbound slot, as declared (kebab-case).
        slot: String,
        /// Where the call site is in `consumer`'s layout. The layout IR has
        /// no source spans, so this is a path of `Tag[part]` (named) or
        /// `Tag#n` (the n-th child, unnamed) segments from the root, e.g.
        /// `Column[app-root] > Row#2 > pkg::mosaic-pkg-lib::Card`.
        call_site: String,
        /// Whether the slot's type can declare a default today (`text`,
        /// `number`, `bool`; UI13 §5 rule 3). Decides whether the message
        /// offers "or give it a default" as a fix.
        defaultable: bool,
    },
    /// A package component's expression names something that is neither
    /// one of its slots nor a `For` binding in scope.
    ///
    /// ```text
    /// // Card.mil: no `hint`       // Card.mll
    /// component Card { … }         If ( when: ( hint > 0 ) ) { … }
    /// ```
    ///
    /// The layout validator treats an expression as opaque text, so this
    /// compiled. After inlining, the free `hint` sat in the consumer's
    /// scope and meant whatever the consumer called `hint`: a silent
    /// capture, and a way for a dependency to read a consumer value it was
    /// never given.
    UndeclaredExpressionIdentifier {
        package: String,
        component: String,
        /// The free name, as written.
        identifier: String,
        /// The whole expression it appears in.
        expression: String,
    },
    /// A package component reads its child-mount slot somewhere other than
    /// the `slot: <name>` mount itself.
    ///
    /// A mount slot is filled by the caller's children, which are spliced in
    /// at the mount, never by a prop, so it has no value anywhere else. Left
    /// in the tree, that reference would mean the consumer's slot of the same
    /// name. That is why the mount slot, alone among `node` and `list<node>`
    /// slots, needs no binding at the call site.
    ChildMountSlotReferenced {
        package: String,
        component: String,
        slot: String,
    },
    /// Several independent errors, reported together so one build shows
    /// every call site to fix rather than the first. Never nested, never
    /// empty, never a single error (that is reported bare).
    Multiple {
        errors: Vec<LayoutResolveError>,
        /// Errors found beyond the reporting cap ([`MAX_REPORTED_ERRORS`])
        /// and left out of `errors`.
        omitted: usize,
    },
    /// A package in the `pkg::` graph — or the consumer, through
    /// [`LayoutPackageResolver::check_dependencies`] — requires a version of
    /// a dependency that the search path does not hold. Boxed, as in
    /// [`ResolveError::VersionMismatch`].
    VersionMismatch(Box<VersionMismatch>),
}

/// At most this many errors are kept in one [`LayoutResolveError::Multiple`].
/// A layout that mounts a broken component in a large `For`-free tree can
/// produce one error per call site; past a hundred, more lines no longer
/// help anyone, and the rest are only counted.
pub const MAX_REPORTED_ERRORS: usize = 100;

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
            Self::InlineChildrenNotAccepted {
                package,
                component,
                child_count,
            } => write!(
                f,
                "component `{package}::{component}` received {child_count} inline child node(s) but its layout has no typed `slot: <name>` child mount"
            ),
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
            Self::SlotBindingMissing {
                consumer,
                package,
                component,
                slot,
                call_site,
                defaultable,
            } => {
                write!(
                    f,
                    "`{consumer}` mounts `pkg::{package}::{component}` at {call_site} without binding its required slot `{slot}`; bind `{slot}` at the call site"
                )?;
                if *defaultable {
                    write!(
                        f,
                        ", or give it a default in {component}.mil (`slot {slot} : <type> = <value> ;`)"
                    )
                } else {
                    write!(f, " (a slot of this type cannot have a default)")
                }
            }
            Self::UndeclaredExpressionIdentifier {
                package,
                component,
                identifier,
                expression,
            } => write!(
                f,
                "package `{package}` component `{component}` uses `{identifier}` in the expression `{expression}`, but `{identifier}` is neither a slot declared in {component}.mil nor a `For` binding in scope; declare the slot, or use a name that is one"
            ),
            Self::ChildMountSlotReferenced {
                package,
                component,
                slot,
            } => write!(
                f,
                "package `{package}` component `{component}` reads its child-mount slot `{slot}` outside the `slot: {slot}` mount; the caller's children fill that slot only at the mount, so it has no value anywhere else"
            ),
            Self::Multiple { errors, omitted } => {
                write!(
                    f,
                    "{} errors resolving package references:",
                    errors.len() + omitted
                )?;
                for (index, error) in errors.iter().enumerate() {
                    write!(f, "\n  {}. {error}", index + 1)?;
                }
                if *omitted > 0 {
                    write!(f, "\n  … and {omitted} more")?;
                }
                Ok(())
            }
            Self::VersionMismatch(mismatch) => write!(f, "{mismatch}"),
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
    cache: Mutex<HashMap<(String, String), ResolvedComponent>>,
    /// Components that failed to load, and why.
    failed: Mutex<HashMap<(String, String), LayoutResolveError>>,
    /// Interfaces (`.mil` slots) loaded for `check_callee_scope`.
    interfaces: Mutex<HashMap<(String, String), Option<Interface>>>,
    /// Package name → the directory `locate_package` found it in. Every
    /// lookup otherwise re-scans and re-parses every manifest on the search
    /// path, and version checks add one lookup per declared dependency.
    located: Mutex<HashMap<String, PathBuf>>,
    /// Packages whose own `[dependencies]` have already been checked, so a
    /// package inlined from many times is checked once.
    checked: Mutex<HashSet<String>>,
}

/// A component's declared slots, as its `.mil` lists them.
type Interface = Vec<mosmodel_compiler::SlotDecl>;

/// One part renamed because its component was mounted more than once
/// (UI34 §5 step 4). `from` is the authored name, `to` the name it has in
/// this mount. The artifact builder copies `from`'s style to `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRename {
    pub package: String,
    pub component: String,
    pub from: String,
    pub to: String,
}

/// State carried through one resolve: the in-flight `(P, C)` stack (cycle
/// detection), how many times each `(P, C)` has been mounted so far, and the
/// renames made.
struct ResolveState {
    visiting: Vec<(String, String)>,
    mounts: HashMap<(String, String), usize>,
    renames: Vec<PartRename>,
}

#[derive(Clone)]
struct ResolvedComponent {
    layout: LayoutDef,
    default_bindings: HashMap<String, LayoutPropValue>,
    /// The component's interface: every slot its `.mil` declares. The
    /// required-binding check reads this, never the layout (§ "Required
    /// bindings" below).
    slots: Vec<mosmodel_compiler::SlotDecl>,
    /// The names of the emits its `.mil` declares.
    emits: Vec<String>,
    /// The slot its layout's `slot: <name>` child mount names, if it has
    /// one. The only slot a call site fills with children instead of a prop.
    child_mount: Option<String>,
}

impl LayoutPackageResolver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            cache: Mutex::new(HashMap::new()),
            failed: Mutex::new(HashMap::new()),
            interfaces: Mutex::new(HashMap::new()),
            located: Mutex::new(HashMap::new()),
            checked: Mutex::new(HashSet::new()),
        }
    }

    /// Check a package's `[dependencies]` requirements against the packages
    /// this resolver's search path holds (mosaic-package-publishing-v1 §7.4).
    ///
    /// [`Self::resolve`] already does this for every package it inlines
    /// from. What it cannot see is the *consumer* — the package whose layout
    /// it was handed — so a package build calls this once with the
    /// consumer's own manifest before resolving any layout.
    ///
    /// `dependent_manifest` is where `dependent` was read from; it is named
    /// in the error as the file to fix.
    ///
    /// A declared dependency that the search path does not hold is skipped
    /// here, exactly as before this check existed: the build fails with
    /// [`LayoutResolveError::PackageNotFound`] if a layout actually
    /// references it. Requiring every declaration to resolve is a separate
    /// rule (spec §10, Phase 1b/3), not a version question.
    pub fn check_dependencies(
        &self,
        dependent: &MosaicPackage,
        dependent_manifest: &Path,
    ) -> Result<(), LayoutResolveError> {
        // Name order, so the first error is the same on every run.
        let mut dependencies: Vec<(&String, &String)> = dependent.dependencies.iter().collect();
        dependencies.sort();
        for (dependency, requirement) in dependencies {
            let found_at = match self.locate_package(dependency) {
                Ok(root) => root,
                Err(LayoutResolveError::PackageNotFound { .. }) => continue,
                Err(other) => return Err(other),
            };
            let found = self.read_manifest(dependency, &found_at)?;
            check_requirement(
                dependent,
                dependent_manifest,
                dependency,
                requirement,
                &found,
                &found_at,
            )
            .map_err(|failure| match failure {
                RequirementFailure::Mismatch(mismatch) => {
                    LayoutResolveError::VersionMismatch(mismatch)
                }
                RequirementFailure::Invalid(detail) => LayoutResolveError::ManifestParseError {
                    package: dependent.package.name.clone(),
                    path: dependent_manifest.to_path_buf(),
                    detail,
                },
            })?;
        }
        Ok(())
    }

    /// [`Self::check_dependencies`] for a package found on the search path,
    /// at most once per package per resolver.
    fn check_dependencies_once(
        &self,
        pkg: &str,
        pkg_root: &Path,
        manifest: &MosaicPackage,
    ) -> Result<(), LayoutResolveError> {
        if self.checked.lock().unwrap().contains(pkg) {
            return Ok(());
        }
        self.check_dependencies(manifest, &pkg_root.join("mosaic-package.toml"))?;
        self.checked.lock().unwrap().insert(pkg.to_string());
        Ok(())
    }

    /// Mutate `layout` in place, replacing every `pkg::P::C` node with the
    /// referenced component's compiled layout tree.
    pub fn resolve(&self, layout: &mut LayoutDef) -> Result<(), LayoutResolveError> {
        self.resolve_with_renames(layout).map(|_| ())
    }

    /// [`Self::resolve`], also returning the parts renamed because a
    /// component was mounted more than once. The first mount of each
    /// `(P, C)` keeps its authored names; the n-th (n ≥ 2) gets `-m<n>` on every
    /// inlined part (UI34 §5 step 4). Empty when every component is mounted
    /// once, which leaves the output exactly as before.
    pub fn resolve_with_renames(
        &self,
        layout: &mut LayoutDef,
    ) -> Result<Vec<PartRename>, LayoutResolveError> {
        // Every call site's required slots are checked before anything is
        // substituted: once substitution starts, an unbound callee slot is
        // indistinguishable from a reference to the consumer's own slot.
        self.check_required_bindings(layout)?;

        let mut state = ResolveState {
            visiting: Vec::new(),
            mounts: HashMap::new(),
            renames: Vec::new(),
        };
        self.resolve_node(&mut layout.root, &mut state)?;
        Ok(state.renames)
    }

    fn resolve_node(
        &self,
        node: &mut LayoutNode,
        state: &mut ResolveState,
    ) -> Result<(), LayoutResolveError> {
        let visiting = &mut state.visiting;
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
                let mount = {
                    let count = state.mounts.entry((pkg.clone(), comp.clone())).or_insert(0);
                    *count += 1;
                    *count
                };
                self.substitute(node, resolved, &pkg, &comp, mount, &mut state.renames)?;
            }
            pushed
        };

        let mut i = 0;
        while i < node.children.len() {
            self.resolve_node(&mut node.children[i], state)?;
            i += 1;
        }

        for _ in &pushed {
            state.visiting.pop();
        }
        Ok(())
    }

    fn substitute(
        &self,
        target: &mut LayoutNode,
        resolved: ResolvedComponent,
        pkg: &str,
        comp: &str,
        mount: usize,
        renames: &mut Vec<PartRename>,
    ) -> Result<(), LayoutResolveError> {
        let call_props = std::mem::take(&mut target.props);
        let consumer_part = target.part_name.take();
        let call_children = std::mem::take(&mut target.children);
        let caller_named_root = consumer_part.is_some();

        target.tag = resolved.layout.root.tag;
        target.part_name = consumer_part.or(resolved.layout.root.part_name);
        target.props = resolved.layout.root.props;
        target.children = resolved.layout.root.children;

        // Only props that name one of the callee's slots or emits become
        // bindings. Any other call-site prop names nothing in the callee, so
        // substituting it could only rewrite a name it does not own (a `For`
        // binding, say). Reporting such props is F2's `UnknownSlotBinding`.
        let declared: Vec<LayoutProp> = call_props
            .into_iter()
            .filter(|prop| {
                resolved.slots.iter().any(|slot| slot.name == prop.name)
                    || resolved.emits.iter().any(|emit| emit == &prop.name)
            })
            .collect();
        let mut bindings = resolved.default_bindings;
        bindings.extend(build_binding_map(&declared));
        let one_of = one_of_literals(&resolved.slots, &mut bindings);
        rewrite_bindings(target, &bindings, &one_of, &HashSet::new());
        fold_constant_expressions(target);
        fold_constant_conditionals(target);

        let exports = self.package_exports(pkg)?;
        qualify_local_refs(target, pkg, &exports);

        // A second (third, …) mount: suffix every inlined part so the tree
        // keeps unique part names. Before splicing, so the caller's own
        // children keep theirs; a root the caller named is theirs too.
        if mount >= 2 {
            // `-m2`, not `-2`: a Mosaic identifier segment cannot start with a digit,
            // and a consumer `.msl` must be able to name the renamed part.
            let suffix = format!("-m{mount}");
            let mut rename = |node: &mut LayoutNode| {
                if let Some(name) = node.part_name.as_mut() {
                    let from = name.clone();
                    name.push_str(&suffix);
                    renames.push(PartRename {
                        package: pkg.to_string(),
                        component: comp.to_string(),
                        from,
                        to: name.clone(),
                    });
                }
            };
            if !caller_named_root {
                rename(target);
            }
            for child in &mut target.children {
                visit_nodes_mut(child, &mut rename);
            }
        }

        let child_count = call_children.len();
        let accepted = splice_default_children(target, call_children);
        if child_count > 0 && !accepted {
            return Err(LayoutResolveError::InlineChildrenNotAccepted {
                package: pkg.to_string(),
                component: comp.to_string(),
                child_count,
            });
        }

        Ok(())
    }

    /// `pkg::comp`, compiled and checked, from the cache when it has been
    /// loaded before. A failure is cached too: a broken component mounted
    /// at many call sites is compiled once, not once per call site.
    fn resolve_component(
        &self,
        pkg: &str,
        comp: &str,
    ) -> Result<ResolvedComponent, LayoutResolveError> {
        let key = (pkg.to_string(), comp.to_string());
        if let Some(cached) = self.cache.lock().unwrap().get(&key).cloned() {
            return Ok(cached);
        }
        if let Some(failed) = self.failed.lock().unwrap().get(&key).cloned() {
            return Err(failed);
        }
        match self.load_component(pkg, comp) {
            Ok(resolved) => {
                self.cache.lock().unwrap().insert(key, resolved.clone());
                Ok(resolved)
            }
            Err(error) => {
                self.failed.lock().unwrap().insert(key, error.clone());
                Err(error)
            }
        }
    }

    /// The `.mil` and `.mll` paths of exported component `pkg::comp`,
    /// checked to lie inside the package (no symlink escape).
    fn component_paths(
        &self,
        pkg: &str,
        comp: &str,
    ) -> Result<(PathBuf, PathBuf), LayoutResolveError> {
        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;
        // Inlining from `pkg` means compiling against whatever `pkg` itself
        // inlines from, so its requirements must hold too (§7.4).
        self.check_dependencies_once(pkg, &pkg_root, &manifest)?;

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
        Ok((mil_path, mll_path))
    }

    /// The slots `pkg::comp` declares, from its `.mil` alone, or `None` when
    /// it cannot be loaded (substitution reports why). Compiling only the
    /// interface keeps this free of recursion, so a cycle cannot loop here.
    fn interface_slots(&self, pkg: &str, comp: &str) -> Option<Vec<mosmodel_compiler::SlotDecl>> {
        let key = (pkg.to_string(), comp.to_string());
        if let Some(cached) = self.interfaces.lock().unwrap().get(&key) {
            return cached.clone();
        }
        let slots = self
            .component_paths(pkg, comp)
            .ok()
            .and_then(|(mil_path, _)| std::fs::read_to_string(mil_path).ok())
            .and_then(|mil| mosmodel_compiler::compile(&mil).ok())
            .map(|model| model.component.slots);
        self.interfaces.lock().unwrap().insert(key, slots.clone());
        slots
    }

    /// The interfaces of every component `root` mounts, for
    /// `check_callee_scope`.
    fn nested_interfaces(
        &self,
        pkg: &str,
        root: &LayoutNode,
    ) -> Result<NestedInterfaces, LayoutResolveError> {
        let mut nested = NestedInterfaces {
            pkg: pkg.to_string(),
            exports: self.package_exports(pkg)?,
            slots: HashMap::new(),
        };
        let mut targets = Vec::new();
        let mut collect = |node: &LayoutNode| {
            if let Some(target) = nested.target(node) {
                targets.push(target);
            }
        };
        visit_nodes(root, &mut collect);
        for target in targets {
            if let std::collections::hash_map::Entry::Vacant(entry) = nested.slots.entry(target) {
                let (pkg, comp) = entry.key();
                if let Some(slots) = self.interface_slots(pkg, comp) {
                    entry.insert(slots);
                }
            }
        }
        Ok(nested)
    }

    fn load_component(
        &self,
        pkg: &str,
        comp: &str,
    ) -> Result<ResolvedComponent, LayoutResolveError> {
        let (mil_path, mll_path) = self.component_paths(pkg, comp)?;
        let mil_src = read_component_source(pkg, comp, &mil_path)?;
        let mll_src = read_component_source(pkg, comp, &mll_path)?;

        let mosmodel_out = mosmodel_compiler::compile(&mil_src).map_err(|errs| {
            LayoutResolveError::NestedCompileError {
                package: pkg.to_string(),
                component: comp.to_string(),
                detail: format!("{errs:?}"),
            }
        })?;
        let mut layout_out =
            moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json)).map_err(
                |errs| LayoutResolveError::NestedCompileError {
                    package: pkg.to_string(),
                    component: comp.to_string(),
                    detail: format!("{errs:?}"),
                },
            )?;

        // The layout validator treats an expression as opaque text, skips the
        // bare NAMEs at call sites (it cannot see the mounted component's
        // `one-of` values; outside call sites it rejects an unbound one as
        // `UnboundName`), and allows a mount slot to be read anywhere, so all
        // three are checked here, once per component, before the layout is
        // ever inlined.
        let child_mount = find_child_mount(&layout_out.def.root);
        let nested = self.nested_interfaces(pkg, &layout_out.def.root)?;
        let (scope_errors, omitted) = check_callee_scope(
            pkg,
            comp,
            &mut layout_out.def.root,
            &mosmodel_out.component.slots,
            child_mount.as_deref(),
            &nested,
        );
        if let Some(error) = combine_errors(scope_errors, omitted) {
            return Err(error);
        }

        let resolved = ResolvedComponent {
            layout: layout_out.def,
            default_bindings: build_default_binding_map(&mosmodel_out.component.slots),
            slots: mosmodel_out.component.slots,
            emits: mosmodel_out
                .component
                .emits
                .iter()
                .map(|emit| emit.name.clone())
                .collect(),
            child_mount,
        };
        Ok(resolved)
    }

    // -----------------------------------------------------------------------
    // Required bindings (UI34 §7 `SlotBindingMissing`, UI13 §1)
    // -----------------------------------------------------------------------
    //
    // UI13 §1: a slot with a default is optional; a slot without one is
    // required, and "the compiler rejects a component instantiation that
    // omits a required slot". This is that rejection.
    //
    // Why it must look at the interface and run before substitution. The
    // inliner works by text substitution: `slot: hint` inside the callee
    // becomes whatever the call site bound to `hint`. When the call site
    // bound nothing, `slot: hint` is left as it is, and after inlining it
    // sits in the *consumer's* tree, where it means the consumer's slot:
    //
    //   callee binds?   consumer has `hint`?   result before this check
    //   ─────────────   ────────────────────   ───────────────────────────
    //   yes             either                 correct
    //   no              no                     UnknownSlot, naming the consumer
    //   no              yes                    builds; consumer's value leaks in
    //   no (expr only)  no                     builds; undeclared identifier
    //                                          in the generated Swift and TS
    //
    // So the question has to be asked while the callee's names are still
    // the callee's, and about every slot the callee declares, not only the
    // slots its layout happens to read: a layout that reads `hint` only
    // inside `If ( when: hint > 0 )` would otherwise slip through, which is
    // the last row.
    //
    // The walk visits the consumer's tree and, once each, the layout of
    // every package component reachable from it, so a missing binding deep
    // inside a package is found in the same build. Every miss is collected
    // and reported together.

    /// Check every `pkg::P::C` call site reachable from `layout` binds every
    /// required slot of `C`. Returns all misses (and any package that could
    /// not be loaded along the way) as one error.
    fn check_required_bindings(&self, layout: &LayoutDef) -> Result<(), LayoutResolveError> {
        let mut walk = BindingCheck {
            errors: Vec::new(),
            omitted: 0,
            loads_reported: HashSet::new(),
            checked: HashSet::new(),
        };
        let scope = CallScope {
            consumer: layout.component_name.clone(),
            local_package: None,
        };
        let mut path = Vec::new();
        self.check_tree(&layout.root, None, &scope, &mut path, &mut walk);

        match combine_errors(walk.errors, walk.omitted) {
            None => Ok(()),
            Some(error) => Err(error),
        }
    }

    /// Visit `node` (the `position`-th child of its parent; `None` for a
    /// layout root) and its subtree, in the scope of `scope.consumer`.
    fn check_tree(
        &self,
        node: &LayoutNode,
        position: Option<usize>,
        scope: &CallScope,
        path: &mut Vec<String>,
        walk: &mut BindingCheck,
    ) {
        path.push(call_site_segment(node, position));

        if let Some((pkg, comp)) = scope.call_target(node) {
            match self.resolve_component(&pkg, &comp) {
                Ok(resolved) => {
                    let missing = missing_required_slots(
                        &resolved.slots,
                        resolved.child_mount.as_deref(),
                        &node.props,
                    );
                    for slot in missing {
                        walk.push(LayoutResolveError::SlotBindingMissing {
                            consumer: scope.consumer.clone(),
                            package: pkg.clone(),
                            component: comp.clone(),
                            slot: slot.name.clone(),
                            call_site: path.join(" > "),
                            defaultable: is_defaultable(&slot.r#type),
                        });
                    }
                    // The callee's own layout is checked once, however many
                    // times it is mounted: its call sites, and so its
                    // misses, are the same every time. The set also ends the
                    // walk on a cycle, which substitution then reports.
                    if walk.checked.insert((pkg.clone(), comp.clone())) {
                        self.check_component_layout(&pkg, &comp, &resolved.layout, walk);
                    }
                }
                Err(error) => walk.push_load(error, &pkg, &comp),
            }
        }

        // Children written at the call site are the consumer's, so they stay
        // in this scope even though they will be spliced into the callee.
        for (index, child) in node.children.iter().enumerate() {
            self.check_tree(child, Some(index), scope, path, walk);
        }
        path.pop();
    }

    /// Check the call sites inside package component `pkg::comp`'s own
    /// layout. Inside a package an unqualified tag naming one of the
    /// package's exports is a call site too: the inliner qualifies it
    /// (`qualify_local_refs`) only after substitution.
    fn check_component_layout(
        &self,
        pkg: &str,
        comp: &str,
        layout: &LayoutDef,
        walk: &mut BindingCheck,
    ) {
        let exports = match self.package_exports(pkg) {
            Ok(exports) => exports,
            Err(error) => {
                walk.push_load(error, pkg, comp);
                return;
            }
        };
        let scope = CallScope {
            consumer: format!("pkg::{pkg}::{comp}"),
            local_package: Some((pkg.to_string(), exports)),
        };
        let mut path = Vec::new();
        self.check_tree(&layout.root, None, &scope, &mut path, walk);
    }

    fn package_exports(&self, pkg: &str) -> Result<Vec<String>, LayoutResolveError> {
        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;
        Ok(manifest.components.exports)
    }

    fn locate_package(&self, pkg: &str) -> Result<PathBuf, LayoutResolveError> {
        if let Some(found) = self.located.lock().unwrap().get(pkg) {
            return Ok(found.clone());
        }
        let found = self.scan_for_package(pkg)?;
        self.located
            .lock()
            .unwrap()
            .insert(pkg.to_string(), found.clone());
        Ok(found)
    }

    fn scan_for_package(&self, pkg: &str) -> Result<PathBuf, LayoutResolveError> {
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

/// Replace the single validated UI29-2 child mount in `node` with the
/// caller-authored subtrees. The moslayout compiler guarantees that a
/// component layout contains at most one mount, so the first match is the only
/// match. Inserted caller nodes are deliberately not traversed here: their
/// bindings belong to the consuming component and must not be rewritten in the
/// dependency's slot scope.
/// Apply `f` to `node` and every node beneath it, depth-first.
fn visit_nodes(node: &LayoutNode, f: &mut impl FnMut(&LayoutNode)) {
    f(node);
    for child in &node.children {
        visit_nodes(child, f);
    }
}

/// Apply `f` to `node` and every node beneath it, depth-first.
fn visit_nodes_mut(node: &mut LayoutNode, f: &mut impl FnMut(&mut LayoutNode)) {
    f(node);
    for child in &mut node.children {
        visit_nodes_mut(child, f);
    }
}

fn splice_default_children(node: &mut LayoutNode, authored: Vec<LayoutNode>) -> bool {
    let mut authored = Some(authored);
    splice_default_children_inner(node, &mut authored)
}

fn splice_default_children_inner(
    node: &mut LayoutNode,
    authored: &mut Option<Vec<LayoutNode>>,
) -> bool {
    let mut index = 0;
    while index < node.children.len() {
        if node.children[index].is_child_slot_mount() {
            let replacements = authored.take().unwrap_or_default();
            node.children.splice(index..=index, replacements);
            return true;
        }
        if splice_default_children_inner(&mut node.children[index], authored) {
            return true;
        }
        index += 1;
    }
    false
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

fn build_default_binding_map(
    slots: &[mosmodel_compiler::SlotDecl],
) -> HashMap<String, LayoutPropValue> {
    let mut bindings = HashMap::new();
    for slot in slots {
        let Some(default) = &slot.default else {
            continue;
        };
        let value = match default {
            SlotDefault::Text(value) => LayoutPropValue::String(value.clone()),
            SlotDefault::Number(value) => LayoutPropValue::Number(*value),
            SlotDefault::Bool(value) => LayoutPropValue::Keyword(value.to_string()),
        };
        bindings.insert(slot.name.clone(), value.clone());
        let camel = to_camel_case_first_lower(&slot.name);
        if camel != slot.name {
            bindings.insert(camel, value);
        }
    }
    bindings
}

/// What the required-binding walk has found so far.
struct BindingCheck {
    /// The errors kept, in walk order, at most [`MAX_REPORTED_ERRORS`].
    errors: Vec<LayoutResolveError>,
    /// How many more were found past the cap.
    omitted: usize,
    /// The load failures already recorded, keyed by
    /// [`load_error_key`]: (variant, package, component).
    loads_reported: HashSet<(&'static str, String, String)>,
    /// The package components whose own layouts have been checked.
    checked: HashSet<(String, String)>,
}

impl BindingCheck {
    /// Record `error`, or count it once the cap is reached. A `Multiple`
    /// is flattened so the result is never nested.
    ///
    /// Misses need no deduplication: each component layout is walked once
    /// and each call-site path is unique within it, so the same miss
    /// cannot be found twice.
    fn push(&mut self, error: LayoutResolveError) {
        match error {
            LayoutResolveError::Multiple { errors, omitted } => {
                for error in errors {
                    self.push(error);
                }
                self.omitted += omitted;
            }
            error if self.errors.len() < MAX_REPORTED_ERRORS => self.errors.push(error),
            _ => self.omitted += 1,
        }
    }

    /// Record a failure to load `pkg::comp` (or its package), once. A
    /// package that cannot be found is reached from every call site that
    /// names it, and one report is enough. A set lookup, not a scan of
    /// `errors`, so the cost does not grow with the error count.
    fn push_load(&mut self, error: LayoutResolveError, pkg: &str, comp: &str) {
        if self
            .loads_reported
            .insert(load_error_key(&error, pkg, comp))
        {
            self.push(error);
        }
    }
}

/// The identity of a load failure: which variant, and the package and
/// component it is about. Variants that name a package or component use
/// their own fields, so a missing package is one failure whichever of its
/// components was asked for. Anything else (a component whose layout failed
/// its scope check, say) is keyed by the component that was requested.
fn load_error_key(
    error: &LayoutResolveError,
    pkg: &str,
    comp: &str,
) -> (&'static str, String, String) {
    use LayoutResolveError as E;
    match error {
        E::PackageNotFound { package } => ("PackageNotFound", package.clone(), String::new()),
        E::ManifestParseError { package, .. } => {
            ("ManifestParseError", package.clone(), String::new())
        }
        E::ComponentNotExported {
            package, component, ..
        } => ("ComponentNotExported", package.clone(), component.clone()),
        E::ComponentSourceMissing {
            package, component, ..
        } => ("ComponentSourceMissing", package.clone(), component.clone()),
        E::NestedCompileError {
            package, component, ..
        } => ("NestedCompileError", package.clone(), component.clone()),
        E::IoError {
            package, component, ..
        } => ("IoError", package.clone(), component.clone()),
        E::SymlinkEscape {
            package, component, ..
        } => ("SymlinkEscape", package.clone(), component.clone()),
        _ => ("Component", pkg.to_string(), comp.to_string()),
    }
}

/// `errors` as one error: `None` when empty, the error itself when there is
/// one and nothing was omitted, otherwise a `Multiple`.
fn combine_errors(
    mut errors: Vec<LayoutResolveError>,
    omitted: usize,
) -> Option<LayoutResolveError> {
    match (errors.len(), omitted) {
        (0, 0) => None,
        (1, 0) => Some(errors.remove(0)),
        _ => Some(LayoutResolveError::Multiple { errors, omitted }),
    }
}

/// Whose layout the walk is in, which decides two things: who a miss is
/// reported against, and whether a bare tag can be a call site.
struct CallScope {
    /// `JournalApp`, or `pkg::P::C` inside a package component.
    consumer: String,
    /// Inside a package component, that package and its exports.
    local_package: Option<(String, Vec<String>)>,
}

impl CallScope {
    /// The `(package, component)` that `node` mounts, if it mounts one.
    ///
    /// ```text
    /// pkg::mosaic-pkg-grid::Cell     → (mosaic-pkg-grid, Cell)   anywhere
    /// Cell                           → (mosaic-pkg-grid, Cell)   inside mosaic-pkg-grid,
    ///                                                            which exports Cell
    /// Column                         → None                      a kernel primitive
    /// ```
    fn call_target(&self, node: &LayoutNode) -> Option<(String, String)> {
        if let Some((pkg, comp)) = node.package_ref() {
            return Some((pkg.to_string(), comp.to_string()));
        }
        let (pkg, exports) = self.local_package.as_ref()?;
        exports
            .iter()
            .any(|export| export == &node.tag)
            .then(|| (pkg.clone(), node.tag.clone()))
    }
}

/// One step of a call-site path: `Tag[part]` for a named node, `Tag#n` for
/// the n-th (1-based) child of its parent, and the bare tag for an unnamed
/// root. A part name is the stable handle an author searches for; a
/// position is the fallback.
fn call_site_segment(node: &LayoutNode, position: Option<usize>) -> String {
    match (&node.part_name, position) {
        (Some(part), _) => format!("{}[{part}]", node.tag),
        (None, Some(index)) => format!("{}#{}", node.tag, index + 1),
        (None, None) => node.tag.clone(),
    }
}

/// The required slots of `slots` that `call_props` does not bind, in
/// declaration order.
///
/// A slot is required when it has no default (UI13 §1), except the one slot
/// the layout's child mount names (`child_mount`): that is filled by the
/// caller's children, never by a prop, and an unfilled mount is removed
/// (UI29-2). Any other `node` or `list<node>` slot is read as a prop
/// (`HostSurface ( content: slot: content-surface )`), so children cannot
/// fill it and it is required like any other. `check_callee_scope`
/// guarantees the mount slot is read nowhere but the mount.
///
/// A prop binds a slot only under the slot's declared (kebab-case) name.
/// `build_binding_map` also files each binding under its camelCase
/// spelling, but that is for identifiers inside expressions: a `slot:
/// body-value` prop is looked up as `body-value`, so a call site binding
/// `bodyValue:` would leave it unbound. Accepting that spelling here would
/// reopen the hole this check closes.
///
/// ```text
/// slot title : text ;         bound as  title: "Inbox"      → bound
/// slot hint  : text ;         not bound                     → MISSING
/// slot tone  : text = "" ;    not bound                     → default, fine
/// slot body  : list<node> ;   not bound, the mount's slot   → children, fine
/// slot icon  : node ;         not bound, read as a prop     → MISSING
/// ```
fn missing_required_slots<'a>(
    slots: &'a [mosmodel_compiler::SlotDecl],
    child_mount: Option<&str>,
    call_props: &[LayoutProp],
) -> Vec<&'a mosmodel_compiler::SlotDecl> {
    let bound: HashSet<&str> = call_props.iter().map(|prop| prop.name.as_str()).collect();
    slots
        .iter()
        .filter(|slot| {
            slot.required
                && Some(slot.name.as_str()) != child_mount
                && !bound.contains(slot.name.as_str())
        })
        .collect()
}

/// The slot types that can declare a default today (UI13 §5 rule 3;
/// `SlotDefault` has exactly these three variants).
fn is_defaultable(slot_type: &mosmodel_compiler::SlotType) -> bool {
    use mosmodel_compiler::SlotType;
    matches!(
        slot_type,
        SlotType::Text | SlotType::Number | SlotType::Bool
    )
}

/// Rewrite the callee's slot and emit references in `node`'s subtree to
/// the values in `bindings`. `shadowed` holds the `For` bindings (`as:`,
/// `index:`) in scope: a bare name in an expression that matches one is the
/// loop variable, not a slot, and is left alone ("For wins", UI29 §3.4).
fn rewrite_bindings(
    node: &mut LayoutNode,
    bindings: &HashMap<String, LayoutPropValue>,
    one_of: &HashMap<String, String>,
    shadowed: &HashSet<String>,
) {
    // Dependency events are private unless the call site forwards them. Leaving
    // an omitted emit here leaks its name into the consuming component's scope.
    // Caller-authored child content is spliced in after this traversal.
    node.props.retain(|prop| match &prop.value {
        LayoutPropValue::EmitRef(name) => bindings.contains_key(name),
        _ => true,
    });
    let tag = node.tag.clone();
    for prop in &mut node.props {
        match &prop.value {
            // A `one-of` value is an enum literal only where every backend
            // reads a bare NAME as one; anywhere else it is the string it
            // spells (`bindings` already holds it as one).
            LayoutPropValue::SlotRef(name)
                if one_of.get(name).is_some_and(|value| {
                    moslayout_compiler::is_keyword_literal(&tag, &prop.name, value)
                }) =>
            {
                prop.value = LayoutPropValue::Keyword(one_of[name].clone());
            }
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
                let rewritten = rewrite_expression_bindings(text, bindings, shadowed);
                if rewritten != *text {
                    prop.value = LayoutPropValue::Expr(rewritten);
                }
            }
            _ => {}
        }
    }
    let inner = for_scope(node, shadowed);
    for child in &mut node.children {
        rewrite_bindings(child, bindings, one_of, &inner);
    }
}

/// Make every `one-of` binding a string, and return the values by slot name
/// (both spellings) for `rewrite_bindings`.
///
/// A call site writes a `one-of` value bare (`tone: loud`), so it arrives as
/// a `Keyword`, and a `Keyword` substituted into the callee would be read
/// as a variable: React `{loud}` in `Text ( content: slot: tone )`, and
/// `( loud == "calm" )` in an expression, an undeclared identifier (P14).
/// Held as `String`, it renders as the text `loud` and compares as the
/// string `"loud"`. In a `KEYWORD_LITERAL_PROPS` position it goes back to a
/// `Keyword`, since an enum prop wants the bare value.
///
/// Only a value the slot declares is a literal. A bare NAME that is not
/// one (`tone: row` inside the consumer's `For ( as: row )`) is the
/// consumer's variable, and stays a `Keyword`. A `String` that spells a
/// declared value (forwarded from an outer `one-of` slot) is recorded too,
/// so it reaches an enum prop as a `Keyword` again.
fn one_of_literals(
    slots: &[mosmodel_compiler::SlotDecl],
    bindings: &mut HashMap<String, LayoutPropValue>,
) -> HashMap<String, String> {
    let mut literals = HashMap::new();
    for slot in slots {
        let mosmodel_compiler::SlotType::OneOf(values) = &slot.r#type else {
            continue;
        };
        let value = match bindings.get(&slot.name) {
            Some(LayoutPropValue::Keyword(value) | LayoutPropValue::String(value))
                if values.contains(value) =>
            {
                value.clone()
            }
            _ => continue,
        };
        let camel = to_camel_case_first_lower(&slot.name);
        for name in [slot.name.clone(), camel] {
            bindings.insert(name.clone(), LayoutPropValue::String(value.clone()));
            literals.insert(name, value.clone());
        }
    }
    literals
}

/// The names in scope for `node`'s children: `scope`, plus the `as:` and
/// `index:` names when `node` is a `For`. A `For`'s own props (`each:`) are
/// evaluated outside it, so only its children see them.
fn for_scope(node: &LayoutNode, scope: &HashSet<String>) -> HashSet<String> {
    let mut inner = scope.clone();
    if node.tag == "For" {
        for prop in &node.props {
            if let ("as" | "index", LayoutPropValue::Keyword(name)) =
                (prop.name.as_str(), &prop.value)
            {
                inner.insert(name.clone());
            }
        }
    }
    inner
}

// ---------------------------------------------------------------------------
// Constant expressions
// ---------------------------------------------------------------------------
//
// Substitution can leave an expression with no names in it: Badge's
// `( tone == "calm" )` with `tone: loud` bound becomes `( "loud" == "calm" )`.
// That is correct, but TypeScript rejects it: comparing two different string
// literal types is error TS2367 ("This comparison appears to be
// unintentional"), and React's output is type-checked. So the inliner
// evaluates what has become constant, the way `fold_constant_conditionals`
// already picks the branch of a literal `If`:
//
//   ( "loud" == "calm" )           →  false         (the whole prop becomes `false`)
//   ( "loud" == "calm" || open )   →  ( open )
//   ( "calm" == "calm" && open )   →  ( open )
//   ( row [ 0 ] == "calm" )        →  unchanged     (a name is left)
//
// Only what every backend agrees on is folded:
// - `==` and `!=` between two literals of the same kind;
// - `<`, `<=`, `>` and `>=` between two numbers;
// - `!` of a boolean;
// - `&&` and `||` whose left side is a boolean literal. `false && x` is
//   `false`, `true && x` is `x`, and the same for `||`. That is exact even
//   in JavaScript, where these return an operand rather than a boolean.
// Anything else, such as truthiness or comparisons across kinds, is left as
// written. An expression in which nothing folds keeps its text byte for
// byte.

/// A literal value inside an expression.
#[derive(Debug, Clone, PartialEq)]
enum Konst {
    Bool(bool),
    Num(f64),
    /// A string's contents as written between the quotes, escapes included.
    /// Two strings are equal exactly when these are, because the layout
    /// compiler and `js_string_literal` escape the same way.
    Str(String),
}

/// One expression node, evaluated as far as it goes.
struct Folded {
    /// Its value, when it is a constant.
    konst: Option<Konst>,
    /// Its text, re-printed with single spaces as the layout compiler does.
    text: String,
    /// Whether anything at or below this node was folded.
    changed: bool,
}

impl Folded {
    fn leaf(text: String, konst: Option<Konst>) -> Self {
        Self {
            konst,
            text,
            changed: false,
        }
    }

    fn bool(value: bool) -> Self {
        Self {
            konst: Some(Konst::Bool(value)),
            text: value.to_string(),
            changed: true,
        }
    }
}

/// Tokens of an expression, for the folder. `None` if it holds anything the
/// grammar does not (then nothing is folded).
fn fold_tokens(expr: &str) -> Option<Vec<String>> {
    let bytes = expr.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
        } else if b == b'"' {
            let start = i;
            i += 1;
            let mut escaped = false;
            loop {
                let c = *bytes.get(i)?;
                i += 1;
                if escaped {
                    escaped = false;
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    break;
                }
            }
            tokens.push(expr[start..i].to_string());
        } else if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            tokens.push(expr[start..i].to_string());
        } else if is_identifier_start(b) {
            let start = i;
            i += 1;
            loop {
                while i < bytes.len() && is_identifier_continue(bytes[i]) {
                    i += 1;
                }
                if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1].is_ascii_alphabetic() {
                    i += 1;
                    continue;
                }
                break;
            }
            tokens.push(expr[start..i].to_string());
        } else {
            let two = expr.get(i..i + 2).unwrap_or("");
            let op = if matches!(two, "==" | "!=" | "<=" | ">=" | "&&" | "||") {
                two
            } else {
                let one = expr.get(i..i + 1)?;
                if !matches!(one, "<" | ">" | "!" | "." | "[" | "]" | "(" | ")" | ":") {
                    return None;
                }
                one
            };
            i += op.len();
            tokens.push(op.to_string());
        }
    }
    Some(tokens)
}

/// A recursive-descent evaluator over `moslayout.grammar`'s `expr`.
struct ExprFolder {
    tokens: Vec<String>,
    at: usize,
}

impl ExprFolder {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.at).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn expect(&mut self, token: &str) -> Option<()> {
        (self.next()? == token).then_some(())
    }

    /// `or_expr` and `and_expr`: a left-associative chain of `op`.
    fn chain(&mut self, op: &str, operand: fn(&mut Self) -> Option<Folded>) -> Option<Folded> {
        let mut acc = operand(self)?;
        while self.peek() == Some(op) {
            self.next();
            let rhs = operand(self)?;
            // `&&` short-circuits on false, `||` on true.
            let stop = op == "||";
            acc = match acc.konst {
                Some(Konst::Bool(b)) if b == stop => Folded::bool(b),
                Some(Konst::Bool(_)) => Folded {
                    changed: true,
                    ..rhs
                },
                _ => Folded {
                    text: format!("{} {op} {}", acc.text, rhs.text),
                    konst: None,
                    changed: acc.changed || rhs.changed,
                },
            };
        }
        Some(acc)
    }

    fn or_expr(&mut self) -> Option<Folded> {
        self.chain("||", Self::and_expr)
    }

    fn and_expr(&mut self) -> Option<Folded> {
        self.chain("&&", Self::eq_expr)
    }

    /// `eq_expr` and `rel_expr`: one optional comparison.
    fn eq_expr(&mut self) -> Option<Folded> {
        let lhs = self.rel_expr()?;
        let Some(op) = self
            .peek()
            .filter(|t| matches!(*t, "==" | "!="))
            .map(str::to_string)
        else {
            return Some(lhs);
        };
        self.next();
        let rhs = self.rel_expr()?;
        let same = match (&lhs.konst, &rhs.konst) {
            (Some(Konst::Bool(a)), Some(Konst::Bool(b))) => Some(a == b),
            (Some(Konst::Num(a)), Some(Konst::Num(b))) => Some(a == b),
            (Some(Konst::Str(a)), Some(Konst::Str(b))) => Some(a == b),
            _ => None,
        };
        Some(match same {
            Some(same) => Folded::bool(same == (op == "==")),
            None => Folded {
                text: format!("{} {op} {}", lhs.text, rhs.text),
                konst: None,
                changed: lhs.changed || rhs.changed,
            },
        })
    }

    fn rel_expr(&mut self) -> Option<Folded> {
        let lhs = self.unary()?;
        let Some(op) = self
            .peek()
            .filter(|t| matches!(*t, "<" | "<=" | ">" | ">="))
            .map(str::to_string)
        else {
            return Some(lhs);
        };
        self.next();
        let rhs = self.unary()?;
        Some(match (&lhs.konst, &rhs.konst) {
            (Some(Konst::Num(a)), Some(Konst::Num(b))) => Folded::bool(match op.as_str() {
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                _ => a >= b,
            }),
            _ => Folded {
                text: format!("{} {op} {}", lhs.text, rhs.text),
                konst: None,
                changed: lhs.changed || rhs.changed,
            },
        })
    }

    fn unary(&mut self) -> Option<Folded> {
        if self.peek() != Some("!") {
            return self.postfix();
        }
        self.next();
        let operand = self.unary()?;
        Some(match operand.konst {
            Some(Konst::Bool(b)) => Folded::bool(!b),
            _ => Folded {
                text: format!("! {}", operand.text),
                konst: None,
                changed: operand.changed,
            },
        })
    }

    fn postfix(&mut self) -> Option<Folded> {
        let mut base = self.primary()?;
        loop {
            match self.peek() {
                Some(".") => {
                    self.next();
                    let member = self.next()?;
                    base = Folded {
                        text: format!("{} . {member}", base.text),
                        konst: None,
                        changed: base.changed,
                    };
                }
                Some("[") => {
                    self.next();
                    let index = self.or_expr()?;
                    self.expect("]")?;
                    base = Folded {
                        text: format!("{} [ {} ]", base.text, index.text),
                        konst: None,
                        changed: base.changed || index.changed,
                    };
                }
                _ => return Some(base),
            }
        }
    }

    fn primary(&mut self) -> Option<Folded> {
        let token = self.next()?;
        if token == "(" {
            let inner = self.or_expr()?;
            self.expect(")")?;
            return Some(Folded {
                text: format!("( {} )", inner.text),
                konst: inner.konst,
                changed: inner.changed,
            });
        }
        if (token == "slot" || token == "emit") && self.peek() == Some(":") {
            self.next();
            let name = self.next()?;
            return Some(Folded::leaf(format!("{token} : {name}"), None));
        }
        let first = token.as_bytes()[0];
        let konst = if first == b'"' {
            Some(Konst::Str(token[1..token.len() - 1].to_string()))
        } else if first.is_ascii_digit() {
            Some(Konst::Num(token.parse().ok()?))
        } else if token == "true" || token == "false" {
            Some(Konst::Bool(token == "true"))
        } else if is_identifier_start(first) {
            None
        } else {
            return None;
        };
        Some(Folded::leaf(token, konst))
    }
}

/// What folding did to one expression prop.
#[derive(Debug, PartialEq)]
enum FoldOutcome {
    /// Nothing was constant: keep the text exactly.
    Unchanged,
    /// Part of it folded.
    Text(String),
    /// All of it folded to a literal.
    Value(LayoutPropValue),
}

fn fold_expression(expr: &str) -> FoldOutcome {
    let Some(tokens) = fold_tokens(expr) else {
        return FoldOutcome::Unchanged;
    };
    let mut folder = ExprFolder { tokens, at: 0 };
    let Some(root) = folder.or_expr() else {
        return FoldOutcome::Unchanged;
    };
    if folder.at != folder.tokens.len() || !root.changed {
        return FoldOutcome::Unchanged;
    }
    match root.konst {
        Some(Konst::Bool(b)) => FoldOutcome::Value(LayoutPropValue::Keyword(b.to_string())),
        Some(Konst::Num(n)) => FoldOutcome::Value(LayoutPropValue::Number(n)),
        Some(Konst::Str(raw)) => FoldOutcome::Value(LayoutPropValue::String(unescape_js(&raw))),
        None => FoldOutcome::Text(root.text),
    }
}

/// The text a JavaScript-style string literal's contents (between the
/// quotes) stands for, for the escapes `js_string_literal` writes.
fn unescape_js(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Fold every constant expression in an inlined callee subtree (see the
/// section comment). Runs before caller children are spliced in, so only
/// the callee's own expressions are touched.
fn fold_constant_expressions(node: &mut LayoutNode) {
    for prop in &mut node.props {
        if let LayoutPropValue::Expr(text) = &prop.value {
            match fold_expression(text) {
                FoldOutcome::Unchanged => {}
                FoldOutcome::Text(text) => prop.value = LayoutPropValue::Expr(text),
                FoldOutcome::Value(value) => prop.value = value,
            }
        }
    }
    for child in &mut node.children {
        fold_constant_expressions(child);
    }
}

/// Resolve `If`/`Else` pairs whose condition became a literal during binding.
///
/// A component that branches on a bool slot (`If ( when: slot: vertical )`)
/// and a caller that passes a literal (`vertical : false`) leave an `If` whose
/// `when:` is the keyword `false`. The layout validator only accepts a slot
/// or an expression there, so the consumer failed to compile (#15432, found
/// adopting SegmentedControl in Engram). Rewriting the keyword into an
/// expression is not safe either: the static-HTML runtime resolves a bare
/// `true` as a data path, which is undefined, so the branch would invert.
///
/// Instead the choice is made here, once: the branch that applies is spliced
/// into the parent in the `If`'s place and the other is dropped.
///
/// ```text
/// Column { If (when: false) { A }  Else { B } }   →   Column { B }
/// Column { If (when: true)  { A }  Else { B } }   →   Column { A }
/// Column { If (when: false) { A } }               →   Column { }
/// ```
///
/// Splicing is sound because `If`/`Else` are not containers: every emitter
/// lays a branch's children out in the enclosing container. An `If` at the
/// root of the component is folded only when the chosen branch is exactly
/// one node, since a layout must keep a single root.
fn fold_constant_conditionals(node: &mut LayoutNode) {
    fold_constant_children(node);

    // Only the component root is folded in place. Doing this at every level
    // would replace a nested `If` before its parent could pair it with its
    // `Else`, leaving the `Else` orphaned.
    if let Some(condition) = literal_condition(node) {
        if condition && node.children.len() == 1 {
            *node = node.children.remove(0);
        }
    }
}

fn fold_constant_children(node: &mut LayoutNode) {
    for child in &mut node.children {
        fold_constant_children(child);
    }

    if node
        .children
        .iter()
        .any(|child| literal_condition(child).is_some())
    {
        let mut folded = Vec::with_capacity(node.children.len());
        let mut children = std::mem::take(&mut node.children).into_iter().peekable();
        while let Some(child) = children.next() {
            let Some(condition) = literal_condition(&child) else {
                folded.push(child);
                continue;
            };
            let else_branch = children.next_if(|next| next.tag == "Else");
            if condition {
                folded.extend(child.children);
            } else if let Some(else_branch) = else_branch {
                folded.extend(else_branch.children);
            }
        }
        node.children = folded;
    }
}

/// `Some(bool)` when `node` is an `If` whose `when:` is a literal boolean.
fn literal_condition(node: &LayoutNode) -> Option<bool> {
    if node.tag != "If" {
        return None;
    }
    let when = node.props.iter().find(|prop| prop.name == "when")?;
    let text = match &when.value {
        LayoutPropValue::Keyword(word) => word.as_str(),
        LayoutPropValue::Expr(expr) => expr.trim(),
        // A literal bound into a text or number slot the component gates on
        // (`title : "Inbox"` into `If ( when: slot: title )`). Truthiness is
        // the kernel's: a text is true when non-empty, a number when non-zero,
        // the same rule every emitter's runtime truthiness helper applies.
        LayoutPropValue::String(text) => return Some(!text.is_empty()),
        LayoutPropValue::Number(number) => return Some(*number != 0.0),
        _ => return None,
    };
    match text {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Expression names
// ---------------------------------------------------------------------------
//
// A layout expression reaches the resolver as text: the moslayout compiler
// joins its tokens with spaces (`reconstruct_expr_text`). The grammar
// (`moslayout.grammar`, `primary`) allows exactly these operands:
//
//   slot: NAME   emit: NAME   NAME   NUMBER   STRING   ( expr )
//
// and the operators `|| && == != < <= > >= !`, member access `.NAME` and
// index `[expr]`. There is no arithmetic, so a `-` can only be part of a
// kebab-case NAME (`selected-index`). There are no calls, and the only
// word literals are `true` and `false`.
//
// Both the binding rewrite and the scope check read the text through
// `expression_pieces`, so they always agree on what a name is.

/// What a name in an expression is, from where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameRole {
    /// After `.`: a member of the value before it (`item.title`).
    Member,
    /// The word `slot` or `emit` of a `slot: x` / `emit: x` operand.
    RefKeyword,
    /// The `x` of `slot: x`.
    SlotRef,
    /// The `x` of `emit: x`.
    EmitRef,
    /// A bare operand: a slot, a `For` binding, or `true`/`false`.
    Bare,
}

/// One piece of expression text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprPiece<'a> {
    /// A string literal, quotes included. Its contents are never names.
    Str(&'a str),
    /// A name and its role.
    Name(&'a str, NameRole),
    /// Anything else, one character at a time: operators, digits,
    /// brackets, whitespace.
    Other(&'a str),
}

/// Split expression text into pieces.
///
/// ```text
/// item.title == slot: selected-key && !done
/// └─┬┘│└─┬─┘    └┬─┘  └─────┬────┘     └┬─┘
///  Bare│ Member RefKeyword SlotRef     Bare
///      Other
/// ```
fn expression_pieces(expr: &str) -> Vec<ExprPiece<'_>> {
    let bytes = expr.as_bytes();
    let mut pieces = Vec::new();
    let mut i = 0;
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
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    break;
                }
            }
            pieces.push(ExprPiece::Str(&expr[start..i]));
            continue;
        }
        if is_identifier_start(b) {
            let start = i;
            i += 1;
            loop {
                while i < bytes.len() && is_identifier_continue(bytes[i]) {
                    i += 1;
                }
                // `-` followed by a letter continues a kebab-case NAME.
                if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1].is_ascii_alphabetic() {
                    i += 1;
                    continue;
                }
                break;
            }
            pieces.push(ExprPiece::Name(&expr[start..i], NameRole::Bare));
            continue;
        }
        // One whole character, so multi-byte text is never split.
        let width = expr[i..].chars().next().map_or(1, char::len_utf8);
        pieces.push(ExprPiece::Other(&expr[i..i + width]));
        i += width;
    }

    // Second pass: roles, from the significant (non-whitespace) neighbours.
    let significant: Vec<usize> = (0..pieces.len())
        .filter(|&k| !matches!(pieces[k], ExprPiece::Other(t) if t.trim().is_empty()))
        .collect();
    let text_at = |k: usize| match pieces[k] {
        ExprPiece::Str(t) | ExprPiece::Name(t, _) | ExprPiece::Other(t) => t,
    };
    let mut roles: Vec<(usize, NameRole)> = Vec::new();
    for (n, &k) in significant.iter().enumerate() {
        let ExprPiece::Name(text, _) = pieces[k] else {
            continue;
        };
        let prev = n.checked_sub(1).map(|m| text_at(significant[m]));
        let prev2 = n.checked_sub(2).map(|m| pieces[significant[m]]);
        let next = significant.get(n + 1).map(|&m| text_at(m));
        let after_next = significant.get(n + 2).map(|&m| text_at(m));
        let role = if prev == Some(".") {
            NameRole::Member
        } else if (text == "slot" || text == "emit") && next == Some(":") && after_next != Some(":")
        {
            NameRole::RefKeyword
        } else if prev == Some(":") {
            match prev2 {
                Some(ExprPiece::Name("slot", _)) => NameRole::SlotRef,
                Some(ExprPiece::Name("emit", _)) => NameRole::EmitRef,
                _ => NameRole::Bare,
            }
        } else {
            NameRole::Bare
        };
        roles.push((k, role));
    }
    for (k, role) in roles {
        if let ExprPiece::Name(text, _) = pieces[k] {
            pieces[k] = ExprPiece::Name(text, role);
        }
    }
    pieces
}

/// Rewrite the callee's names in `expr` to their bound values.
///
/// A bare name or the `x` of `slot: x` is replaced when it is bound; a bare
/// name that a `For` binding in `shadowed` covers is the loop variable and
/// is kept. Member names, string contents and everything else are copied.
fn rewrite_expression_bindings(
    expr: &str,
    bindings: &HashMap<String, LayoutPropValue>,
    shadowed: &HashSet<String>,
) -> String {
    let mut out = String::with_capacity(expr.len());
    for piece in expression_pieces(expr) {
        match piece {
            ExprPiece::Name(name, NameRole::Bare) if shadowed.contains(name) => out.push_str(name),
            ExprPiece::Name(name, NameRole::Bare | NameRole::SlotRef) => match bindings.get(name) {
                Some(value) => out.push_str(&layout_prop_value_as_expression(value)),
                None => out.push_str(name),
            },
            ExprPiece::Str(text) | ExprPiece::Name(text, _) | ExprPiece::Other(text) => {
                out.push_str(text)
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Callee scope check
// ---------------------------------------------------------------------------
//
// Substitution can only rewrite names it knows. Three kinds of name in a
// package layout would survive it and land in the consumer's scope:
//
// - a free name in an expression that is not one of the component's slots
//   (`If ( when: ( hint > 0 ) )` with no `slot hint`). The layout validator
//   does not look inside expressions, so it compiled;
// - a bare NAME prop value in a position the emitters lower to an
//   identifier (`Text ( content: authToken )` becomes React `{authToken}`).
//   It is a `Keyword` in the IR, which substitution never rewrote, so even
//   a component's own `content: title` showed the consumer's `title`;
// - a reference to the child-mount slot anywhere but the mount. Nothing
//   binds that slot by prop, so the reference is never rewritten.
//
// Any of them, left in the inlined tree, means the consumer's name: capture
// at best, a dependency reading a consumer value on purpose at worst. All
// are checked when the component is loaded, before it is ever inlined. A
// bare NAME that names one of the component's own slots is not an error: it
// is rewritten to `slot: <name>`, so substitution binds it like any other
// slot reference.
//
// Which bare NAMEs are literals:
//
//   position                                   bare NAME is
//   ─────────────────────────────────────────  ──────────────────────────
//   `true` / `false`, anywhere                 a literal
//   a KEYWORD_LITERAL_PROPS position           a literal enum value
//     (`axis: both`, `a11y-role: heading`)
//   a `For`'s `as:` / `index:`                 a binder
//   a call site's prop bound to a `one-of`     a literal, if it is one of
//     slot of the component it mounts          that slot's declared values;
//                                              otherwise an identifier, as
//                                              below (`tone: tone` forwards
//                                              the caller's own `tone`)
//   anywhere else                              an identifier: a `For`
//                                              binding in scope, or a slot
//                                              of this component, or an error

/// The slot named by `root`'s `slot: <name>` child mount, if it has one.
/// The moslayout compiler allows at most one mount per layout.
fn find_child_mount(root: &LayoutNode) -> Option<String> {
    if let Some(name) = root.child_slot_name() {
        return Some(name.to_string());
    }
    root.children.iter().find_map(find_child_mount)
}

/// What `check_callee_scope` needs to know about the components a layout
/// mounts: the package's own exports (a bare tag naming one is a call site,
/// as in `CallScope::call_target`) and the slots of each component mounted.
struct NestedInterfaces {
    pkg: String,
    exports: Vec<String>,
    slots: HashMap<(String, String), Vec<mosmodel_compiler::SlotDecl>>,
}

impl NestedInterfaces {
    /// The `(package, component)` that `node` mounts, if it is a call site.
    fn target(&self, node: &LayoutNode) -> Option<(String, String)> {
        if let Some((pkg, comp)) = node.package_ref() {
            return Some((pkg.to_string(), comp.to_string()));
        }
        self.exports
            .iter()
            .any(|export| export == &node.tag)
            .then(|| (self.pkg.clone(), node.tag.clone()))
    }

    /// Whether `word`, bound to `prop` at a call site of `target`, is one of
    /// the values of the `one-of` slot it binds. Only a declared value is a
    /// literal: any other bare NAME there is an identifier (`tone: tone`
    /// forwards the caller's own `tone`; `tone: authToken` reads nothing the
    /// caller declares). Unknown (an interface that failed to load) counts
    /// as no, so the value gets the identifier check: fail closed.
    fn is_one_of_value(&self, target: &(String, String), prop: &str, word: &str) -> bool {
        self.slots.get(target).is_some_and(|slots| {
            slots.iter().any(|slot| {
                slot.name == prop
                    && matches!(&slot.r#type, mosmodel_compiler::SlotType::OneOf(values)
                        if values.iter().any(|value| value == word))
            })
        })
    }
}

/// Check package component `pkg::comp`'s layout `root`, and rewrite each
/// bare NAME that names one of its slots to a `slot:` reference.
///
/// Returns the errors, at most [`MAX_REPORTED_ERRORS`] of them, and how
/// many more were found:
/// [`LayoutResolveError::UndeclaredExpressionIdentifier`] once per free
/// name, and [`LayoutResolveError::ChildMountSlotReferenced`] once.
fn check_callee_scope(
    pkg: &str,
    comp: &str,
    root: &mut LayoutNode,
    slots: &[mosmodel_compiler::SlotDecl],
    child_mount: Option<&str>,
    nested: &NestedInterfaces,
) -> (Vec<LayoutResolveError>, usize) {
    // Both spellings of every slot, built once: lookups are per name, and a
    // layout can hold thousands of names.
    let mut by_name = HashMap::with_capacity(slots.len() * 2);
    for slot in slots {
        by_name.insert(slot.name.clone(), slot.name.clone());
        by_name.insert(to_camel_case_first_lower(&slot.name), slot.name.clone());
    }
    let mut scope = CalleeScope {
        pkg,
        comp,
        by_name,
        child_mount,
        nested,
        errors: Vec::new(),
        omitted: 0,
        reported: HashSet::new(),
    };
    scope.visit(root, &HashSet::new());
    (scope.errors, scope.omitted)
}

struct CalleeScope<'a> {
    pkg: &'a str,
    comp: &'a str,
    /// Slot name, in kebab-case or camelCase, to its declared name.
    by_name: HashMap<String, String>,
    child_mount: Option<&'a str>,
    nested: &'a NestedInterfaces,
    errors: Vec<LayoutResolveError>,
    /// Errors found past [`MAX_REPORTED_ERRORS`].
    omitted: usize,
    /// Names already reported, so each is reported once per component.
    reported: HashSet<String>,
}

/// At most this many characters of an expression are quoted in an error.
const MAX_QUOTED_EXPRESSION: usize = 200;

/// `text`, cut to [`MAX_QUOTED_EXPRESSION`] characters (never inside one).
fn quote_expression(text: &str) -> String {
    match text.char_indices().nth(MAX_QUOTED_EXPRESSION) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

impl CalleeScope<'_> {
    /// The declared slot `name` refers to, matching the kebab-case name or
    /// its camelCase form (`selected-index` or `selectedIndex`).
    fn slot_named(&self, name: &str) -> Option<String> {
        self.by_name
            .get(name)
            .or_else(|| self.by_name.get(&to_camel_case_first_lower(name)))
            .cloned()
    }

    fn report(&mut self, error: LayoutResolveError) {
        if self.errors.len() < MAX_REPORTED_ERRORS {
            self.errors.push(error);
        } else {
            self.omitted += 1;
        }
    }

    fn mount_read(&mut self, slot: &str) {
        if self.reported.insert(format!("slot:{slot}")) {
            let error = LayoutResolveError::ChildMountSlotReferenced {
                package: self.pkg.to_string(),
                component: self.comp.to_string(),
                slot: slot.to_string(),
            };
            self.report(error);
        }
    }

    fn undeclared(&mut self, name: &str, expression: &str) {
        if self.reported.insert(name.to_string()) {
            let error = LayoutResolveError::UndeclaredExpressionIdentifier {
                package: self.pkg.to_string(),
                component: self.comp.to_string(),
                identifier: name.to_string(),
                expression: quote_expression(expression),
            };
            self.report(error);
        }
    }

    fn visit(&mut self, node: &mut LayoutNode, loops: &HashSet<String>) {
        let is_mount = node.is_child_slot_mount();
        let target = self.nested.target(node);
        let tag = node.tag.clone();
        for prop in &mut node.props {
            match &prop.value {
                // The mount's own `slot: <name>` is the one legal read.
                LayoutPropValue::SlotRef(_) if is_mount => {}
                LayoutPropValue::SlotRef(name) if Some(name.as_str()) == self.child_mount => {
                    let name = name.clone();
                    self.mount_read(&name)
                }
                LayoutPropValue::Expr(text) => {
                    let text = text.clone();
                    self.visit_expression(&text, loops)
                }
                LayoutPropValue::Keyword(word) => {
                    let literal = if let Some(target) = &target {
                        word == "true"
                            || word == "false"
                            || self.nested.is_one_of_value(target, &prop.name, word)
                    } else {
                        (tag == "For" && (prop.name == "as" || prop.name == "index"))
                            || moslayout_compiler::is_keyword_literal(&tag, &prop.name, word)
                    };
                    if literal || loops.contains(word) {
                        continue;
                    }
                    let word = word.clone();
                    match self.slot_named(&word) {
                        Some(slot) if Some(slot.as_str()) == self.child_mount => {
                            self.mount_read(&slot)
                        }
                        // The component's own slot: make it a reference that
                        // substitution binds, as `slot: <name>` would be.
                        Some(slot) => prop.value = LayoutPropValue::SlotRef(slot),
                        None => {
                            let written = format!("{}: {word}", prop.name);
                            self.undeclared(&word, &written)
                        }
                    }
                }
                _ => {}
            }
        }
        let inner = for_scope(node, loops);
        for child in &mut node.children {
            self.visit(child, &inner);
        }
    }

    fn visit_expression(&mut self, text: &str, loops: &HashSet<String>) {
        for piece in expression_pieces(text) {
            let ExprPiece::Name(name, role) = piece else {
                continue;
            };
            let slot = match role {
                NameRole::Member | NameRole::RefKeyword => continue,
                // `emit:` inside an expression is validated by the emitters
                // that accept it; it is not a value a consumer could supply.
                NameRole::EmitRef => continue,
                NameRole::Bare if name == "true" || name == "false" => continue,
                NameRole::Bare if loops.contains(name) => continue,
                NameRole::SlotRef | NameRole::Bare => self.slot_named(name),
            };
            match slot {
                Some(slot) if Some(slot.as_str()) == self.child_mount => self.mount_read(&slot),
                Some(_) => {}
                None => self.undeclared(name, text),
            }
        }
    }
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
        write_versioned_manifest(dir, name, "0.1.0", exports, deps);
    }

    /// [`write_manifest`] with an explicit `[package].version`.
    fn write_versioned_manifest(
        dir: &Path,
        name: &str,
        version: &str,
        exports: &[&str],
        deps: &[(&str, &str)],
    ) {
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
version = "{version}"
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

    fn tree_contains_child_mount(node: &LayoutNode) -> bool {
        node.is_child_slot_mount() || node.children.iter().any(tree_contains_child_mount)
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
        // The twenty-nine primitives — fifteen from UI29 §2.1, plus
        // HostDialog added in UI29-1 (#3846), plus HostCheckbox and
        // HostRadio added in UI29-2 (#3978), plus HostLink,
        // HostTooltip, and HostNumberInput added in UI29-4, plus the
        // five UI31 HostTable structural sub-tags (HostTableColGroup,
        // HostTableHead, HostTableBody, HostTableFoot, Col), plus the
        // typed HostSurface native composition boundary, plus the
        // UI29-3 HostSlider range-input primitive, plus the UI38
        // HostSwitch on/off primitive.
        let expected_29 = [
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
            "HostSurface",
            "HostCheckbox",
            "HostRadio",
            "HostLink",
            "HostTooltip",
            "HostNumberInput",
            "HostSlider",
            "HostSwitch",
            "HostTableColGroup",
            "HostTableHead",
            "HostTableBody",
            "HostTableFoot",
            "Col",
        ];
        for name in &expected_29 {
            assert!(
                KERNEL_PRIMITIVES.contains(name),
                "kernel must include `{name}`"
            );
        }
        // And `Else` because the parser treats it as its own tag.
        assert!(KERNEL_PRIMITIVES.contains(&"Else"));
        assert!(
            KERNEL_PRIMITIVES.contains(&"HostNavigationSplit"),
            "UI29-6 registered HostNavigationSplit as a kernel primitive"
        );
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

    #[test]
    fn kernel_set_covers_host_surface() {
        assert!(KERNEL_PRIMITIVES.contains(&"HostSurface"));
    }

    #[test]
    fn kernel_set_covers_path_drawing_primitive() {
        assert!(
            KERNEL_PRIMITIVES.contains(&"Path"),
            "UI39 kernel must include `Path`"
        );
    }

    #[test]
    fn kernel_set_covers_host_progress_ring_primitive() {
        assert!(
            KERNEL_PRIMITIVES.contains(&"HostProgressRing"),
            "#13176 kernel must include `HostProgressRing`"
        );
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
            r#"component Greet { slot label : text ; emit onClick ; emit onFocus ; }"#,
            r#"layout Greet {
  HostButton [ greet-button ] (
    label : slot: label ,
    onClick : emit: onClick,
    onFocus : emit: onFocus
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
        assert!(!layout.root.props.iter().any(|prop| prop.name == "onFocus"));
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
    fn layout_inliner_applies_omitted_slot_defaults_and_callers_override_them() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let mini = make_pkg(&pkgs, "mosaic-pkg-mini", "mosaic-pkg-mini", &["Input"]);
        write_component(
            &mini,
            "Input",
            r#"component Input {
  slot value : text = "" ;
  slot disabled : bool = false ;
  slot scale : number = 1 ;
}"#,
            r#"layout Input {
  HostInput (
    value: slot: value,
    disabled: slot: disabled,
    scale: slot: scale
  )
}"#,
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut defaults = consumer_layout(r#"layout Demo { pkg::mosaic-pkg-mini::Input }"#);
        resolver.resolve(&mut defaults).expect("defaults resolve");
        assert_eq!(
            defaults.root.props,
            vec![
                LayoutProp {
                    name: "value".to_string(),
                    value: LayoutPropValue::String(String::new()),
                },
                LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::Keyword("false".to_string()),
                },
                LayoutProp {
                    name: "scale".to_string(),
                    value: LayoutPropValue::Number(1.0),
                },
            ]
        );

        let mut overridden = consumer_layout(
            r#"layout Demo {
  pkg::mosaic-pkg-mini::Input (
    value: "hello",
    disabled: true,
    scale: 2
  )
}"#,
        );
        resolver
            .resolve(&mut overridden)
            .expect("overrides resolve");
        assert_eq!(
            overridden.root.props,
            vec![
                LayoutProp {
                    name: "value".to_string(),
                    value: LayoutPropValue::String("hello".to_string()),
                },
                LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::Keyword("true".to_string()),
                },
                LayoutProp {
                    name: "scale".to_string(),
                    value: LayoutPropValue::Number(2.0),
                },
            ]
        );
    }

    #[test]
    fn layout_inliner_splices_default_authored_children_without_rebinding_them() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let foundation = make_pkg(
            &pkgs,
            "mosaic-std-foundation",
            "mosaic-std-foundation",
            &["Surface"],
        );
        write_component(
            &foundation,
            "Surface",
            r#"component Surface {
  slot label : text ;
  slot children : list<node> ;
}"#,
            r#"layout Surface {
  Column [ surface-root ] {
    Text [ surface-label ] ( content: slot: label )
    Box [ surface-body ] { slot: children }
  }
}"#,
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout App {
  pkg::mosaic-std-foundation::Surface (
    label: slot: outer-label
  ) {
    Text [ caller-title ] ( content: slot: label )
    Row [ caller-actions ] {
      HostButton ( label: "Continue" )
    }
  }
}"#,
        );

        resolver.resolve(&mut layout).expect("surface resolves");

        assert_eq!(layout.root.tag, "Column");
        assert_eq!(layout.root.part_name.as_deref(), Some("surface-root"));
        assert_eq!(layout.root.children.len(), 2);
        assert!(layout.root.children[0].props.iter().any(|prop| {
            prop.name == "content"
                && prop.value == LayoutPropValue::SlotRef("outer-label".to_string())
        }));

        let body = &layout.root.children[1];
        assert_eq!(body.tag, "Box");
        assert_eq!(body.children.len(), 2);
        assert_eq!(body.children[0].part_name.as_deref(), Some("caller-title"));
        assert!(
            body.children[0].props.iter().any(|prop| {
                prop.name == "content"
                    && prop.value == LayoutPropValue::SlotRef("label".to_string())
            }),
            "caller-owned bindings must not be rewritten in the dependency scope"
        );
        assert_eq!(
            body.children[1].part_name.as_deref(),
            Some("caller-actions")
        );
        assert!(!tree_contains_child_mount(&layout.root));
    }

    #[test]
    fn layout_inliner_removes_an_unfilled_default_child_mount() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let foundation = make_pkg(
            &pkgs,
            "mosaic-std-foundation",
            "mosaic-std-foundation",
            &["Surface"],
        );
        write_component(
            &foundation,
            "Surface",
            "component Surface { slot children : list<node> ; }",
            "layout Surface { Box [ surface-root ] { slot: children } }",
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout("layout App { pkg::mosaic-std-foundation::Surface }");
        resolver
            .resolve(&mut layout)
            .expect("empty surface resolves");

        assert_eq!(layout.root.tag, "Box");
        assert!(layout.root.children.is_empty());
        assert!(!tree_contains_child_mount(&layout.root));
    }

    #[test]
    fn layout_inliner_rejects_children_when_component_has_no_mount() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let mini = make_pkg(&pkgs, "mosaic-pkg-mini", "mosaic-pkg-mini", &["Greet"]);
        write_component(
            &mini,
            "Greet",
            "component Greet { slot label : text ; }",
            "layout Greet { Text ( content: slot: label ) }",
        );

        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout App {
  pkg::mosaic-pkg-mini::Greet ( label: "Hello" ) {
    Text ( content: "must not disappear" )
  }
}"#,
        );
        let error = resolver
            .resolve(&mut layout)
            .expect_err("inline children must never be silently discarded");
        assert!(matches!(
            error,
            LayoutResolveError::InlineChildrenNotAccepted {
                package,
                component,
                child_count: 1,
            } if package == "mosaic-pkg-mini" && component == "Greet"
        ));
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

    /// #15432: a literal bool bound into a component's `If` used to leave
    /// `when: false`, which the validator rejects. The branch is now chosen at
    /// resolution time.
    #[test]
    fn literal_bool_binding_folds_the_branch_it_selects() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let mini = make_pkg(&pkgs, "mosaic-pkg-axis", "mosaic-pkg-axis", &["Axis"]);
        write_component(
            &mini,
            "Axis",
            r#"component Axis {
  slot vertical : bool ;
  slot label : text ;
}"#,
            r#"layout Axis {
  Column [ root ] {
    If ( when: slot: vertical ) {
      Column [ stacked ] { }
    }
    Else {
      Row [ inline ] { }
    }
    Text [ caption ] ( slot: label )
  }
}"#,
        );
        let resolver = LayoutPackageResolver::new(vec![pkgs]);

        for (literal, kept) in [("false", "inline"), ("true", "stacked")] {
            let mut layout = consumer_layout(&format!(
                r#"layout Demo {{
  pkg::mosaic-pkg-axis::Axis ( vertical : {literal} , label : slot: outer )
}}"#
            ));
            resolver.resolve(&mut layout).expect("layout resolves");
            let parts: Vec<_> = layout
                .root
                .children
                .iter()
                .map(|child| (child.tag.as_str(), child.part_name.as_deref()))
                .collect();
            let (tag, part) = if literal == "true" {
                ("Column", kept)
            } else {
                ("Row", kept)
            };
            assert_eq!(
                parts,
                vec![(tag, Some(part)), ("Text", Some("caption"))],
                "vertical: {literal}"
            );
        }

        // A slot binding is not a literal and must stay a runtime branch.
        let mut layout = consumer_layout(
            r#"layout Demo {
  pkg::mosaic-pkg-axis::Axis ( vertical : slot: stacked , label : slot: outer )
}"#,
        );
        resolver.resolve(&mut layout).expect("layout resolves");
        let tags: Vec<_> = layout
            .root
            .children
            .iter()
            .map(|c| c.tag.as_str())
            .collect();
        assert_eq!(tags, vec!["If", "Else", "Text"]);
    }

    #[test]
    fn constant_fold_drops_an_unmatched_false_branch_and_keeps_expressions() {
        let node =
            |tag: &str, when: Option<LayoutPropValue>, children: Vec<LayoutNode>| LayoutNode {
                tag: tag.to_string(),
                part_name: None,
                props: when
                    .into_iter()
                    .map(|value| LayoutProp {
                        name: "when".to_string(),
                        value,
                    })
                    .collect(),
                children,
            };
        let leaf = |tag: &str| node(tag, None, vec![]);
        let mut root = node(
            "Column",
            None,
            vec![
                node(
                    "If",
                    Some(LayoutPropValue::Keyword("false".into())),
                    vec![leaf("A")],
                ),
                leaf("B"),
                node(
                    "If",
                    Some(LayoutPropValue::Expr(" true ".into())),
                    vec![leaf("C"), leaf("D")],
                ),
                node("Else", None, vec![leaf("E")]),
                node(
                    "If",
                    Some(LayoutPropValue::Expr("i == 0".into())),
                    vec![leaf("F")],
                ),
            ],
        );
        fold_constant_conditionals(&mut root);
        let tags: Vec<_> = root.children.iter().map(|c| c.tag.as_str()).collect();
        assert_eq!(tags, vec!["B", "C", "D", "If"]);
    }

    /// A literal string or number bound into a gated slot folds by kernel
    /// truthiness: empty text and zero are false.
    #[test]
    fn literal_text_and_number_bindings_fold_by_truthiness() {
        let when = |value: LayoutPropValue| LayoutNode {
            tag: "If".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "when".to_string(),
                value,
            }],
            children: vec![],
        };
        assert_eq!(
            literal_condition(&when(LayoutPropValue::String(String::new()))),
            Some(false)
        );
        assert_eq!(
            literal_condition(&when(LayoutPropValue::String("Inbox".into()))),
            Some(true)
        );
        assert_eq!(
            literal_condition(&when(LayoutPropValue::Number(0.0))),
            Some(false)
        );
        assert_eq!(
            literal_condition(&when(LayoutPropValue::Number(3.0))),
            Some(true)
        );
        assert_eq!(
            literal_condition(&when(LayoutPropValue::SlotRef("s".into()))),
            None
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
                &bindings,
                &HashSet::new()
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
    // ---- Mounting a component more than once (UI34 §5 step 4) ----

    /// A package with `Empty` (two parts), `Card` (splices caller children)
    /// and `Pair` (mounts `Empty` itself).
    fn multi_mount_packages() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let kit = make_pkg(&pkgs, "kit", "kit", &["Empty", "Card", "Pair"]);
        write_component(
            &kit,
            "Empty",
            "component Empty { slot title : text ; }",
            "layout Empty { Column [ empty ] { Text [ empty-title ] ( content: slot: title ) } }",
        );
        write_component(
            &kit,
            "Card",
            "component Card { slot children : list<node> ; }",
            "layout Card { Box [ card ] { slot: children } }",
        );
        write_component(
            &kit,
            "Pair",
            "component Pair { }",
            "layout Pair { Row [ pair ] { Empty ( title: \"inner\" ) } }",
        );
        (tmp, pkgs)
    }

    fn part_names(node: &LayoutNode, out: &mut Vec<String>) {
        if let Some(name) = &node.part_name {
            out.push(name.clone());
        }
        for child in &node.children {
            part_names(child, out);
        }
    }

    #[test]
    fn a_second_mount_is_suffixed_and_the_first_keeps_its_names() {
        let (_tmp, pkgs) = multi_mount_packages();
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout App { Column [ app ] {
  pkg::kit::Empty ( title: "one" )
  pkg::kit::Empty ( title: "two" )
  pkg::kit::Empty ( title: "three" )
} }"#,
        );
        let renames = resolver
            .resolve_with_renames(&mut layout)
            .expect("resolves");
        let mut names = Vec::new();
        part_names(&layout.root, &mut names);
        assert_eq!(
            names,
            [
                "app",
                "empty",
                "empty-title",
                "empty-m2",
                "empty-title-m2",
                "empty-m3",
                "empty-title-m3"
            ]
        );
        let pairs: Vec<(&str, &str)> = renames
            .iter()
            .map(|r| (r.from.as_str(), r.to.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("empty", "empty-m2"),
                ("empty-title", "empty-title-m2"),
                ("empty", "empty-m3"),
                ("empty-title", "empty-title-m3")
            ]
        );
        assert!(renames
            .iter()
            .all(|r| r.package == "kit" && r.component == "Empty"));
        moslayout_compiler::validate(&layout, None).expect("unique part names");
    }

    #[test]
    fn a_single_mount_is_unchanged_and_reports_no_renames() {
        let (_tmp, pkgs) = multi_mount_packages();
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(r#"layout App { pkg::kit::Empty ( title: "one" ) }"#);
        assert!(resolver
            .resolve_with_renames(&mut layout)
            .unwrap()
            .is_empty());
        let mut names = Vec::new();
        part_names(&layout.root, &mut names);
        assert_eq!(names, ["empty", "empty-title"]);
    }

    #[test]
    fn caller_parts_are_never_suffixed() {
        let (_tmp, pkgs) = multi_mount_packages();
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout App { Column [ app ] {
  pkg::kit::Card { Text [ first-body ] ( content: "a" ) }
  pkg::kit::Card [ named-card ] { Text [ second-body ] ( content: "b" ) }
} }"#,
        );
        resolver.resolve(&mut layout).expect("resolves");
        let mut names = Vec::new();
        part_names(&layout.root, &mut names);
        // The second Card's root is caller-named and its child caller-authored.
        assert_eq!(
            names,
            ["app", "card", "first-body", "named-card", "second-body"]
        );
    }

    #[test]
    fn a_component_mounted_inside_another_counts_across_the_whole_tree() {
        let (_tmp, pkgs) = multi_mount_packages();
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout App { Column [ app ] {
  pkg::kit::Empty ( title: "top" )
  pkg::kit::Pair
  pkg::kit::Pair
} }"#,
        );
        resolver.resolve(&mut layout).expect("resolves");
        let mut names = Vec::new();
        part_names(&layout.root, &mut names);
        assert_eq!(
            names,
            [
                "app",
                "empty",
                "empty-title",
                "pair",
                "empty-m2",
                "empty-title-m2",
                "pair-m2",
                "empty-m3",
                "empty-title-m3"
            ]
        );
        moslayout_compiler::validate(&layout, None).expect("unique part names");
    }

    // ---- Required bindings: SlotBindingMissing (UI34 §7, E6 F1) ----
    //
    // Each test names the probe of mosaic-unbound-slot-semantics-v1 §1.3 it
    // pins. Every fixture is one package, `mosaic-pkg-lib`, exporting `Card`,
    // and a consumer layout that mounts it.

    /// `mosaic-pkg-lib` exporting `Card` with the given `.mil` and `.mll`.
    fn card_package(mil: &str, mll: &str) -> (TempDir, LayoutPackageResolver) {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let lib = make_pkg(&pkgs, "mosaic-pkg-lib", "mosaic-pkg-lib", &["Card"]);
        write_component(&lib, "Card", mil, mll);
        (tmp, LayoutPackageResolver::new(vec![pkgs]))
    }

    /// The `(component, slot)` of every `SlotBindingMissing` in `error`.
    fn missing_slots(error: &LayoutResolveError) -> Vec<(String, String)> {
        let errors = match error {
            LayoutResolveError::Multiple { errors, .. } => errors.as_slice(),
            single => std::slice::from_ref(single),
        };
        errors
            .iter()
            .map(|error| match error {
                LayoutResolveError::SlotBindingMissing {
                    component, slot, ..
                } => (component.clone(), slot.clone()),
                other => panic!("expected SlotBindingMissing, got {other:?}"),
            })
            .collect()
    }

    fn pair(component: &str, slot: &str) -> (String, String) {
        (component.to_string(), slot.to_string())
    }

    /// P1: a required slot the call site omits is an error naming the
    /// package's component and slot, not the consumer's `UnknownSlot`.
    #[test]
    fn an_unbound_required_slot_is_slot_binding_missing() {
        let (_tmp, resolver) = card_package(
            "component Card { slot title : text ; slot hint : text ; }",
            r#"layout Card { Column [ card ] {
  Text [ card-title ] ( content: slot: title )
  Text [ card-hint ] ( content: slot: hint )
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  pkg::mosaic-pkg-lib::Card ( title: slot: heading )
} }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("hint is unbound");
        assert_eq!(
            error,
            LayoutResolveError::SlotBindingMissing {
                consumer: "ProbeApp".to_string(),
                package: "mosaic-pkg-lib".to_string(),
                component: "Card".to_string(),
                slot: "hint".to_string(),
                call_site: "Column[app-root] > pkg::mosaic-pkg-lib::Card#1".to_string(),
                defaultable: true,
            }
        );
        // Reported before substitution: the consumer's tree is untouched.
        assert_eq!(
            first_qualified_tag(&layout.root),
            Some("pkg::mosaic-pkg-lib::Card")
        );
    }

    /// The message names the package, the component and the slot, says
    /// where the call site is, and gives both fixes.
    #[test]
    fn slot_binding_missing_message_names_the_package_and_component() {
        let (_tmp, resolver) = card_package(
            "component Card { slot hint : text ; }",
            "layout Card { Text [ card-hint ] ( content: slot: hint ) }",
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card [ hero ] }");

        let message = resolver
            .resolve(&mut layout)
            .expect_err("hint is unbound")
            .to_string();
        assert_eq!(
            message,
            "`ProbeApp` mounts `pkg::mosaic-pkg-lib::Card` at \
             pkg::mosaic-pkg-lib::Card[hero] without binding its required slot `hint`; \
             bind `hint` at the call site, or give it a default in Card.mil \
             (`slot hint : <type> = <value> ;`)"
        );
    }

    /// A slot whose type cannot have a default (here `list<text>`, P5b) is
    /// told only to bind it.
    #[test]
    fn a_non_defaultable_slot_is_only_told_to_bind() {
        let (_tmp, resolver) = card_package(
            "component Card { slot tags : list<text> ; }",
            r#"layout Card { Column [ card ] {
  For ( each: slot: tags , as: tag ) { Text ( content: tag ) }
} }"#,
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = resolver.resolve(&mut layout).expect_err("tags is unbound");
        assert!(matches!(
            &error,
            LayoutResolveError::SlotBindingMissing { slot, defaultable: false, .. }
                if slot == "tags"
        ));
        assert!(error
            .to_string()
            .ends_with("bind `tags` at the call site (a slot of this type cannot have a default)"));
    }

    /// Every miss is reported in one build, across slots and across call
    /// sites, in walk order.
    #[test]
    fn every_missing_binding_is_reported_together() {
        let (_tmp, resolver) = card_package(
            r#"component Card {
  slot title : text ;
  slot count : number ;
  slot compact : bool ;
}"#,
            r#"layout Card { Column [ card ] {
  Text ( content: slot: title )
  If ( when: slot: compact ) { Text ( content: "compact" ) }
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  pkg::mosaic-pkg-lib::Card ( title: "first" )
  Row [ second-row ] { pkg::mosaic-pkg-lib::Card }
} }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("four misses");
        assert_eq!(
            missing_slots(&error),
            vec![
                pair("Card", "count"),
                pair("Card", "compact"),
                pair("Card", "title"),
                pair("Card", "count"),
                pair("Card", "compact"),
            ]
        );
        let LayoutResolveError::Multiple { errors, .. } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        assert!(matches!(
            &errors[2],
            LayoutResolveError::SlotBindingMissing { call_site, .. }
                if call_site == "Column[app-root] > Row[second-row] > pkg::mosaic-pkg-lib::Card#1"
        ));
        let message = error.to_string();
        assert!(message.starts_with("5 errors resolving package references:\n  1. "));
        assert!(message.contains("\n  5. `ProbeApp` mounts `pkg::mosaic-pkg-lib::Card`"));
    }

    /// P2, P9b and D-default: an omitted slot with a default takes it, as
    /// before. P11: an unfilled `list<node>` mount needs no binding. P8: an
    /// unforwarded emit needs none either.
    #[test]
    fn defaulted_slots_child_mounts_and_emits_need_no_binding() {
        let (_tmp, resolver) = card_package(
            r#"component Card {
  slot title : text ;
  slot hint : text = "" ;
  slot compact : bool = false ;
  slot count : number = 0 ;
  slot children : list<node> ;
  emit onPick ;
}"#,
            r#"layout Card { Column [ card ] {
  Text [ card-title ] ( content: slot: title )
  Text [ card-hint ] ( content: slot: hint )
  If ( when: slot: compact ) { Text ( content: "compact" ) }
  Box [ card-body ] { slot: children }
} }"#,
        );
        let mut layout =
            consumer_layout(r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( title: "Inbox" ) }"#);

        resolver
            .resolve(&mut layout)
            .expect("only title is required");
        assert!(first_qualified_tag(&layout.root).is_none());
        assert_eq!(
            layout.root.children[1].props,
            vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String(String::new()),
            }]
        );
    }

    /// P3, the silent capture: the consumer happens to declare its own
    /// `hint`. Before, the package's unbound `slot: hint` became the
    /// consumer's `hint` after inlining and the build succeeded.
    #[test]
    fn an_unbound_slot_is_never_captured_by_a_same_named_consumer_slot() {
        let (_tmp, resolver) = card_package(
            "component Card { slot title : text ; slot hint : text ; }",
            r#"layout Card { Column [ card ] {
  Text [ card-title ] ( content: slot: title )
  Text [ card-hint ] ( content: slot: hint )
} }"#,
        );
        // The consumer's own `hint` slot is in scope here, and it is used.
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  Text [ app-hint ] ( content: slot: hint )
  pkg::mosaic-pkg-lib::Card ( title: slot: heading )
} }"#,
        );

        let error = resolver
            .resolve(&mut layout)
            .expect_err("the consumer's hint must not fill the package's");
        assert_eq!(missing_slots(&error), vec![pair("Card", "hint")]);
    }

    /// P12, the expression leak: `count` is read only inside an expression,
    /// so no `slot: count` prop was ever left behind to fail validation, and
    /// the generated code named an undeclared `count`.
    #[test]
    fn a_slot_read_only_in_an_expression_must_still_be_bound() {
        let (_tmp, resolver) = card_package(
            "component Card { slot count : number ; }",
            r#"layout Card { Column [ card ] {
  If ( when: ( count > 0 ) ) { Text ( content: "some" ) }
} }"#,
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = resolver.resolve(&mut layout).expect_err("count is unbound");
        assert_eq!(missing_slots(&error), vec![pair("Card", "count")]);
    }

    /// The interface is the contract (spec Q6): a required slot the layout
    /// never reads must still be bound.
    #[test]
    fn a_required_slot_the_layout_never_reads_must_still_be_bound() {
        let (_tmp, resolver) = card_package(
            "component Card { slot title : text ; slot unused : text ; }",
            "layout Card { Text ( content: slot: title ) }",
        );
        let mut layout =
            consumer_layout(r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( title: "x" ) }"#);

        let error = resolver
            .resolve(&mut layout)
            .expect_err("unused is unbound");
        assert_eq!(missing_slots(&error), vec![pair("Card", "unused")]);
    }

    /// A binding counts only under the slot's declared name. Substitution
    /// looks a `slot: body-value` prop up as `body-value`, so `bodyValue:`
    /// would leave it in the tree unbound; the check must not accept it.
    #[test]
    fn a_binding_counts_only_under_the_declared_slot_name() {
        let (_tmp, resolver) = card_package(
            "component Card { slot body-value : text ; }",
            "layout Card { Text ( content: slot: body-value ) }",
        );
        let mut camel =
            consumer_layout(r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( bodyValue: "x" ) }"#);
        let error = resolver
            .resolve(&mut camel)
            .expect_err("bodyValue is not body-value");
        assert_eq!(missing_slots(&error), vec![pair("Card", "body-value")]);

        let mut kebab =
            consumer_layout(r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( body-value: "x" ) }"#);
        resolver.resolve(&mut kebab).expect("body-value is bound");
        assert_eq!(
            kebab.root.props[0].value,
            LayoutPropValue::String("x".to_string())
        );
    }

    /// A miss inside a package component's own layout, through a sibling
    /// referenced without `pkg::`, is reported against that component, not
    /// against the app. Caller-authored children are checked in the
    /// caller's scope.
    #[test]
    fn nested_and_caller_authored_call_sites_are_reported_against_their_author() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let lib = make_pkg(
            &pkgs,
            "mosaic-pkg-lib",
            "mosaic-pkg-lib",
            &["Panel", "Badge"],
        );
        write_component(
            &lib,
            "Badge",
            "component Badge { slot label : text ; }",
            "layout Badge { Text [ badge ] ( content: slot: label ) }",
        );
        write_component(
            &lib,
            "Panel",
            "component Panel { slot children : list<node> ; }",
            // `Badge`, unqualified, mounted without its `label`.
            "layout Panel { Column [ panel ] { Badge  Box [ panel-body ] { slot: children } } }",
        );
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout ProbeApp { pkg::mosaic-pkg-lib::Panel {
  pkg::mosaic-pkg-lib::Badge
} }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("two misses");
        let LayoutResolveError::Multiple { errors, .. } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        let sites: Vec<(&str, &str)> = errors
            .iter()
            .map(|error| match error {
                LayoutResolveError::SlotBindingMissing {
                    consumer,
                    call_site,
                    ..
                } => (consumer.as_str(), call_site.as_str()),
                other => panic!("expected SlotBindingMissing, got {other:?}"),
            })
            .collect();
        assert_eq!(
            sites,
            vec![
                ("pkg::mosaic-pkg-lib::Panel", "Column[panel] > Badge#1"),
                (
                    "ProbeApp",
                    "pkg::mosaic-pkg-lib::Panel > pkg::mosaic-pkg-lib::Badge#1"
                ),
            ]
        );
    }

    /// A component mounted twice has its own layout checked once: the misses
    /// inside it are the same for every mount.
    #[test]
    fn a_package_layout_is_checked_once_however_often_it_is_mounted() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let lib = make_pkg(
            &pkgs,
            "mosaic-pkg-lib",
            "mosaic-pkg-lib",
            &["Panel", "Badge"],
        );
        write_component(
            &lib,
            "Badge",
            "component Badge { slot label : text ; }",
            "layout Badge { Text [ badge ] ( content: slot: label ) }",
        );
        write_component(
            &lib,
            "Panel",
            "component Panel { }",
            "layout Panel { Column [ panel ] { Badge } }",
        );
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column {
  pkg::mosaic-pkg-lib::Panel
  pkg::mosaic-pkg-lib::Panel
} }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("one miss");
        assert_eq!(missing_slots(&error), vec![pair("Badge", "label")]);
    }

    /// A package that cannot be found is reported once, beside any misses,
    /// however many call sites name it.
    #[test]
    fn load_errors_are_reported_once_beside_missing_bindings() {
        let (_tmp, resolver) = card_package(
            "component Card { slot title : text ; }",
            "layout Card { Text ( content: slot: title ) }",
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column {
  pkg::mosaic-pkg-gone::Thing
  pkg::mosaic-pkg-lib::Card
  pkg::mosaic-pkg-gone::Thing
} }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("two errors");
        let LayoutResolveError::Multiple { errors, .. } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0],
            LayoutResolveError::PackageNotFound {
                package: "mosaic-pkg-gone".to_string()
            }
        );
        assert!(matches!(
            &errors[1],
            LayoutResolveError::SlotBindingMissing { slot, .. } if slot == "title"
        ));
    }

    // ---- Security review of F1: the holes F1 left open ----

    /// The single error of `error`'s kind, when it is not a `Multiple`.
    fn only(error: LayoutResolveError) -> LayoutResolveError {
        assert!(
            !matches!(error, LayoutResolveError::Multiple { .. }),
            "expected one error, got {error:?}"
        );
        error
    }

    /// Finding 1: a `node` slot read as a prop is not filled by children,
    /// so it is required. Here the consumer has its own `content-surface`,
    /// the P3 shape: before, the package rendered the consumer's.
    #[test]
    fn a_node_slot_read_as_a_prop_must_be_bound() {
        let (_tmp, resolver) = card_package(
            "component Card { slot content-surface : node ; }",
            "layout Card { HostSurface [ card-surface ] ( content : slot: content-surface ) }",
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  HostSurface [ app-surface ] ( content : slot: content-surface )
  pkg::mosaic-pkg-lib::Card
} }"#,
        );

        let error = resolver
            .resolve(&mut layout)
            .expect_err("unbound node slot");
        assert_eq!(missing_slots(&error), vec![pair("Card", "content-surface")]);
    }

    /// Finding 1: only the slot the mount names is exempt. A second
    /// `list<node>` slot that is not mounted is required like any other.
    #[test]
    fn only_the_mounted_slot_is_exempt_from_binding() {
        let (_tmp, resolver) = card_package(
            r#"component Card {
  slot children : list<node> ;
  slot extras : list<node> ;
}"#,
            r#"layout Card { Column [ card ] {
  Box [ card-body ] { slot: children }
  For ( each: slot: extras , as: extra ) { HostSurface ( content : extra ) }
} }"#,
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = resolver
            .resolve(&mut layout)
            .expect_err("extras is unbound");
        assert_eq!(missing_slots(&error), vec![pair("Card", "extras")]);
    }

    /// Finding 1: the mount slot has no value outside the mount, so reading
    /// it as a prop is rejected when the component loads.
    #[test]
    fn reading_the_mount_slot_as_a_prop_is_rejected() {
        let (_tmp, resolver) = card_package(
            "component Card { slot children : list<node> ; }",
            r#"layout Card { Column [ card ] {
  If ( when: slot: children ) { Box [ card-body ] { slot: children } }
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card { Text ( content: "hi" ) } }"#,
        );

        let error = only(
            resolver
                .resolve(&mut layout)
                .expect_err("mount read as prop"),
        );
        assert_eq!(
            error,
            LayoutResolveError::ChildMountSlotReferenced {
                package: "mosaic-pkg-lib".to_string(),
                component: "Card".to_string(),
                slot: "children".to_string(),
            }
        );
        assert!(error
            .to_string()
            .contains("reads its child-mount slot `children` outside the `slot: children` mount"));
    }

    /// Finding 1: the same through an expression, in either spelling.
    #[test]
    fn reading_the_mount_slot_in_an_expression_is_rejected() {
        for expression in ["( !body-content )", "( slot: body-content || false )"] {
            let (_tmp, resolver) = card_package(
                "component Card { slot body-content : list<node> ; }",
                &format!(
                    "layout Card {{ Column [ card ] {{ If ( when: {expression} ) {{ Box {{ slot: body-content }} }} }} }}"
                ),
            );
            let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

            let error = only(resolver.resolve(&mut layout).expect_err(expression));
            assert!(
                matches!(&error, LayoutResolveError::ChildMountSlotReferenced { slot, .. }
                    if slot == "body-content"),
                "{expression}: {error:?}"
            );
        }
    }

    /// Finding 2: a free name in a package expression is rejected, so it can
    /// never resolve to the consumer's `hint` (here the consumer has one).
    #[test]
    fn an_undeclared_expression_identifier_is_rejected() {
        let (_tmp, resolver) = card_package(
            "component Card { slot title : text ; }",
            r#"layout Card { Column [ card ] {
  Text ( content: slot: title )
  If ( when: ( hint > 0 ) ) { Text ( content: "leak" ) }
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  Text [ app-hint ] ( content: slot: hint )
  pkg::mosaic-pkg-lib::Card ( title: "x" )
} }"#,
        );

        let error = only(resolver.resolve(&mut layout).expect_err("hint is free"));
        assert_eq!(
            error,
            LayoutResolveError::UndeclaredExpressionIdentifier {
                package: "mosaic-pkg-lib".to_string(),
                component: "Card".to_string(),
                identifier: "hint".to_string(),
                expression: "( hint > 0 )".to_string(),
            }
        );
        assert_eq!(
            error.to_string(),
            "package `mosaic-pkg-lib` component `Card` uses `hint` in the expression \
             `( hint > 0 )`, but `hint` is neither a slot declared in Card.mil nor a \
             `For` binding in scope; declare the slot, or use a name that is one"
        );
    }

    /// Finding 2: what an expression may name. Slots in either spelling and
    /// through `slot:`, `For` bindings of every enclosing loop (but not of a
    /// sibling loop), `true`/`false`, members after `.`, string contents and
    /// numbers. Each free name is reported once, in order.
    #[test]
    fn the_expression_scope_check_admits_exactly_the_grammar_s_names() {
        let (_tmp, resolver) = card_package(
            r#"component Card {
  slot rows : list<list<text>> ;
  slot selected-key : text ;
  slot count : number ;
}"#,
            r#"layout Card { Column [ card ] {
  For ( each: slot: rows , as: row , index: r ) {
    For ( each: slot: rows , as: inner ) {
      If ( when: ( row[0] == selectedKey && inner.size != "ghost" && r > 1.5 ) ) { Text ( content: "a" ) }
      If ( when: ( slot: selected-key == "x" || selected-key == count || true ) ) { Text ( content: "b" ) }
    }
  }
  If ( when: ( inner == ghost || ghost != false || row.title ) ) { Text ( content: "c" ) }
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( rows: slot: r , selected-key: "k" , count: 1 ) }"#,
        );

        let error = resolver.resolve(&mut layout).expect_err("three free names");
        let LayoutResolveError::Multiple { errors, omitted: 0 } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        let names: Vec<&str> = errors
            .iter()
            .map(|error| match error {
                LayoutResolveError::UndeclaredExpressionIdentifier { identifier, .. } => {
                    identifier.as_str()
                }
                other => panic!("expected UndeclaredExpressionIdentifier, got {other:?}"),
            })
            .collect();
        // `inner` and `row` are out of their loops here; `ghost` is free
        // twice but reported once.
        assert_eq!(names, vec!["inner", "ghost", "row"]);
    }

    /// Finding 2: `slot: x` inside an expression must name a declared slot.
    #[test]
    fn a_slot_ref_to_an_undeclared_slot_inside_an_expression_is_rejected() {
        let (_tmp, resolver) = card_package(
            "component Card { }",
            r#"layout Card { If ( when: ( slot: hint == "x" ) ) { Text ( content: "y" ) } }"#,
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = only(resolver.resolve(&mut layout).expect_err("hint is free"));
        assert!(matches!(
            &error,
            LayoutResolveError::UndeclaredExpressionIdentifier { identifier, .. } if identifier == "hint"
        ));
    }

    /// Finding 3: a call-site prop that names no callee slot is not a
    /// binding. Before, `row: slot: x` turned the loop's `row.title` into
    /// `x.title`.
    #[test]
    fn an_undeclared_call_site_prop_is_not_substituted() {
        let (_tmp, resolver) = card_package(
            "component Card { slot rows : list<list<text>> ; }",
            r#"layout Card { Column [ card ] {
  For ( each: slot: rows , as: row ) {
    If ( when: ( row.title ) ) { Text ( content: "t" ) }
  }
} }"#,
        );
        let mut layout = consumer_layout(
            "layout ProbeApp { pkg::mosaic-pkg-lib::Card ( rows: slot: items , row: slot: x ) }",
        );

        resolver.resolve(&mut layout).expect("row: is ignored");
        let if_node = &layout.root.children[0].children[0];
        assert_eq!(
            if_node.props[0].value,
            LayoutPropValue::Expr("( row . title )".to_string())
        );
    }

    /// Finding 3: a `For` binding shadows a slot of the same name inside its
    /// body ("For wins", UI29 §3.4), so only the uses outside the loop are
    /// rewritten.
    #[test]
    fn a_for_binding_shadows_a_bound_slot_inside_its_body() {
        let (_tmp, resolver) = card_package(
            "component Card { slot rows : list<text> ; slot row : text ; }",
            r#"layout Card { Column [ card ] {
  If ( when: ( row == "a" ) ) { Text ( content: "outside" ) }
  For ( each: slot: rows , as: row ) {
    If ( when: ( row == "b" ) ) { Text ( content: "inside" ) }
  }
} }"#,
        );
        let mut layout = consumer_layout(
            "layout ProbeApp { pkg::mosaic-pkg-lib::Card ( rows: slot: items , row: slot: picked ) }",
        );

        resolver.resolve(&mut layout).expect("resolves");
        let outside = &layout.root.children[0];
        let inside = &layout.root.children[1].children[0];
        assert_eq!(
            outside.props[0].value,
            LayoutPropValue::Expr(r#"( picked == "a" )"#.to_string())
        );
        assert_eq!(
            inside.props[0].value,
            LayoutPropValue::Expr(r#"( row == "b" )"#.to_string())
        );
    }

    /// A kebab-case name in an expression is one name, as the grammar's
    /// NAME token is, not `selected - index`: there is no minus operator.
    #[test]
    fn expression_pieces_keep_kebab_names_whole_and_assign_roles() {
        let names: Vec<(&str, NameRole)> = expression_pieces(
            r#"item.title == slot: selected-key && !row-x2 && emit: onPick != "a-b" && 1.5 > x"#,
        )
        .into_iter()
        .filter_map(|piece| match piece {
            ExprPiece::Name(name, role) => Some((name, role)),
            _ => None,
        })
        .collect();
        assert_eq!(
            names,
            vec![
                ("item", NameRole::Bare),
                ("title", NameRole::Member),
                ("slot", NameRole::RefKeyword),
                ("selected-key", NameRole::SlotRef),
                ("row-x2", NameRole::Bare),
                ("emit", NameRole::RefKeyword),
                ("onPick", NameRole::EmitRef),
                ("x", NameRole::Bare),
            ]
        );
    }

    /// Finding 4: at most `MAX_REPORTED_ERRORS` errors are kept; the rest are
    /// counted and the message says how many.
    #[test]
    fn the_error_list_is_capped() {
        let (_tmp, resolver) = card_package(
            "component Card { slot a : text ; slot b : text ; slot c : text ; }",
            "layout Card { Column [ card ] { } }",
        );
        let calls = "pkg::mosaic-pkg-lib::Card\n".repeat(50);
        let mut layout = consumer_layout(&format!("layout ProbeApp {{ Column {{\n{calls}}} }}"));

        let error = resolver.resolve(&mut layout).expect_err("150 misses");
        let LayoutResolveError::Multiple { errors, omitted } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        assert_eq!((errors.len(), *omitted), (MAX_REPORTED_ERRORS, 50));
        let message = error.to_string();
        assert!(message.starts_with("150 errors resolving package references:"));
        assert!(message.ends_with("\n  … and 50 more"));
        assert_eq!(message.lines().count(), 1 + MAX_REPORTED_ERRORS + 1);
    }

    // ---- Security review round 2: bare NAME prop values ----

    /// The inlined props of the node with part `part`.
    fn props_of<'a>(node: &'a LayoutNode, part: &str) -> &'a [LayoutProp] {
        fn find<'a>(node: &'a LayoutNode, part: &str) -> Option<&'a LayoutNode> {
            if node.part_name.as_deref() == Some(part) {
                return Some(node);
            }
            node.children.iter().find_map(|child| find(child, part))
        }
        &find(node, part)
            .unwrap_or_else(|| panic!("no part {part}"))
            .props
    }

    /// Deliberate read: `content: authToken` with no such slot would lower to
    /// React `{authToken}`, the consumer's variable. It is rejected.
    ///
    /// Outside a call site, the layout validator now rejects it first, as
    /// `UnboundName` (UI29 §3.4, backlog 6f1p), so the component fails to
    /// compile before `check_callee_scope` runs. The callee-scope check still
    /// owns call-site values (see
    /// `bare_names_at_a_nested_call_site_follow_the_mounted_slot_type`).
    #[test]
    fn a_bare_name_naming_nothing_is_rejected() {
        let (_tmp, resolver) = card_package(
            "component Card { }",
            "layout Card { Text [ card-leak ] ( content: authToken ) }",
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = only(
            resolver
                .resolve(&mut layout)
                .expect_err("authToken is free"),
        );
        assert!(
            matches!(
                &error,
                LayoutResolveError::NestedCompileError { package, component, detail }
                    if package == "mosaic-pkg-lib"
                        && component == "Card"
                        && detail.contains("UnboundName")
                        && detail.contains("bare name `authToken` in `Text ( content: authToken )`")
            ),
            "{error:?}"
        );
    }

    /// Accidental capture: `content: title` names Card's own `title`. It was
    /// never rewritten, so it showed the consumer's `title` even though the
    /// call site bound one. It now takes the binding, or the default.
    #[test]
    fn a_bare_name_naming_an_own_slot_takes_the_binding() {
        let (_tmp, resolver) = card_package(
            r#"component Card { slot title : text ; slot hint : text = "none" ; }"#,
            r#"layout Card { Column [ card ] {
  Text [ card-title ] ( content: title )
  Text [ card-hint ] ( content: hint )
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column [ app-root ] {
  Text [ app-title ] ( content: slot: title )
  pkg::mosaic-pkg-lib::Card ( title: "Inbox" )
} }"#,
        );

        resolver.resolve(&mut layout).expect("resolves");
        assert_eq!(
            props_of(&layout.root, "card-title")[0].value,
            LayoutPropValue::String("Inbox".to_string())
        );
        assert_eq!(
            props_of(&layout.root, "card-hint")[0].value,
            LayoutPropValue::String("none".to_string())
        );
    }

    /// Mount bypass: a bare NAME is one more way to read the mount slot.
    #[test]
    fn a_bare_name_naming_the_mount_slot_is_rejected() {
        let (_tmp, resolver) = card_package(
            "component Card { slot children : list<node> ; }",
            r#"layout Card { Column [ card ] {
  Text [ card-leak ] ( content: children )
  Box { slot: children }
} }"#,
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = only(resolver.resolve(&mut layout).expect_err("mount read"));
        assert!(matches!(
            &error,
            LayoutResolveError::ChildMountSlotReferenced { slot, .. } if slot == "children"
        ));
    }

    /// What stays a literal: enum values in `KEYWORD_LITERAL_PROPS`
    /// positions, even when a slot or a `For` binding has the same name
    /// (VisiCalcStartup has `a11y-role: heading` beside `slot heading`);
    /// `true`/`false`; and `For` bindings, which win over a slot of the same
    /// name inside the loop.
    #[test]
    fn enum_values_booleans_and_loop_bindings_are_left_alone() {
        let (_tmp, resolver) = card_package(
            r#"component Card {
  slot heading : text ;
  slot rows : list<text> ;
  slot line : text ;
}"#,
            r#"layout Card { Column [ card ] {
  Text [ card-heading ] ( content: "h" , a11y-role: heading )
  HostScroll [ card-scroll ] ( axis: horizontal ) { Text ( content: "s" ) }
  HostInput [ card-input ] ( disabled: true )
  For ( each: slot: rows , as: line ) {
    Path [ card-line ] ( kind: line , x1: 0 , y1: 0 , x2: 1 , y2: 1 )
    HostButton [ card-button ] ( label: line )
  }
} }"#,
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { pkg::mosaic-pkg-lib::Card ( heading: "x" , rows: slot: items , line: "y" ) }"#,
        );

        resolver.resolve(&mut layout).expect("resolves");
        let kw = |part: &str, prop: &str| {
            props_of(&layout.root, part)
                .iter()
                .find(|p| p.name == prop)
                .map(|p| p.value.clone())
        };
        let keyword = |k: &str| Some(LayoutPropValue::Keyword(k.to_string()));
        assert_eq!(kw("card-heading", "a11y-role"), keyword("heading"));
        assert_eq!(kw("card-scroll", "axis"), keyword("horizontal"));
        assert_eq!(kw("card-input", "disabled"), keyword("true"));
        assert_eq!(kw("card-line", "kind"), keyword("line"));
        assert_eq!(kw("card-button", "label"), keyword("line"));
    }

    /// A component mounted inside a package: its call-site props bind the
    /// mounted component's slots. A bare NAME bound to a `one-of` slot is
    /// one of its values; any other is an identifier like everywhere else.
    #[test]
    fn bare_names_at_a_nested_call_site_follow_the_mounted_slot_type() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let lib = make_pkg(
            &pkgs,
            "mosaic-pkg-lib",
            "mosaic-pkg-lib",
            &["Panel", "Badge"],
        );
        write_component(
            &lib,
            "Badge",
            "component Badge { slot label : text ; slot tone : one-of calm loud ; }",
            "layout Badge { Text [ badge ] ( content: slot: label ) }",
        );
        write_component(
            &lib,
            "Panel",
            "component Panel { slot title : text ; }",
            "layout Panel { Column [ panel ] { Badge ( label: title , tone: calm ) } }",
        );
        let resolver = LayoutPackageResolver::new(vec![pkgs.clone()]);
        let mut layout =
            consumer_layout(r#"layout ProbeApp { pkg::mosaic-pkg-lib::Panel ( title: "T" ) }"#);
        resolver.resolve(&mut layout).expect("resolves");
        assert_eq!(
            props_of(&layout.root, "badge")[0].value,
            LayoutPropValue::String("T".to_string()),
            "Panel's own `title` reaches Badge's label"
        );

        write_component(
            &lib,
            "Panel",
            "component Panel { }",
            "layout Panel { Column [ panel ] { Badge ( label: secret , tone: calm ) } }",
        );
        let resolver = LayoutPackageResolver::new(vec![pkgs]);
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Panel }");
        let error = only(resolver.resolve(&mut layout).expect_err("secret is free"));
        assert!(matches!(
            &error,
            LayoutResolveError::UndeclaredExpressionIdentifier { identifier, .. } if identifier == "secret"
        ));
    }

    /// Round-2 LOW: callee-scope errors are capped like the others, and a
    /// long expression is quoted in part.
    #[test]
    fn callee_scope_errors_are_capped_and_expressions_quoted_in_part() {
        let names: Vec<String> = (0..150).map(|n| format!("free{n}")).collect();
        let long = format!("( {} )", names.join(" && "));
        let (_tmp, resolver) = card_package(
            "component Card { }",
            &format!("layout Card {{ If ( when: {long} ) {{ Text ( content: \"x\" ) }} }}"),
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");

        let error = resolver.resolve(&mut layout).expect_err("150 free names");
        let LayoutResolveError::Multiple { errors, omitted } = &error else {
            panic!("expected Multiple, got {error:?}");
        };
        assert_eq!((errors.len(), *omitted), (MAX_REPORTED_ERRORS, 50));
        let LayoutResolveError::UndeclaredExpressionIdentifier { expression, .. } = &errors[0]
        else {
            panic!(
                "expected UndeclaredExpressionIdentifier, got {:?}",
                errors[0]
            );
        };
        assert_eq!(expression.chars().count(), MAX_QUOTED_EXPRESSION + 1);
        assert!(expression.ends_with('…'));
    }

    /// Round-2 LOW: a component that fails to load is compiled once per
    /// resolver; later call sites get the cached error.
    #[test]
    fn a_failed_load_is_cached() {
        let (tmp, resolver) = card_package(
            "component Card { }",
            "layout Card { Text ( content: missing ) }",
        );
        let mut layout = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");
        let first = resolver.resolve(&mut layout).expect_err("missing is free");

        // Fixed on disk; the same resolver still answers from its cache.
        let lib = tmp.path().join("packages/mosaic-pkg-lib");
        write_component(
            &lib,
            "Card",
            "component Card { }",
            r#"layout Card { Text ( content: "ok" ) }"#,
        );
        let mut again = consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Card }");
        assert_eq!(resolver.resolve(&mut again).expect_err("cached"), first);
    }

    // ---- Security review round 4: `one-of` values ----

    /// `mosaic-pkg-lib` exporting `Panel` (given) and `Badge`, whose `tone`
    /// is `one-of calm loud`, shown as text, compared in expressions, inside
    /// a `For` whose binding is also called `loud`, and passed to the enum
    /// prop `HostTable ( dir: … )` through `dir`.
    fn badge_package(panel_mil: &str, panel_mll: &str) -> (TempDir, LayoutPackageResolver) {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        let lib = make_pkg(
            &pkgs,
            "mosaic-pkg-lib",
            "mosaic-pkg-lib",
            &["Panel", "Badge"],
        );
        write_component(
            &lib,
            "Badge",
            r#"component Badge {
  slot tone : one-of calm loud ;
  slot dir : one-of ltr rtl ;
  slot open : bool ;
  slot items : list<text> ;
}"#,
            r#"layout Badge { Column [ badge ] {
  Text [ badge-tone ] ( content: tone )
  If ( when: ( tone == "loud" ) ) { Text [ badge-loud ] ( content: "L" ) }
  If ( when: ( tone == "calm" ) ) { Text [ badge-calm ] ( content: "C" ) }
  HostButton [ badge-button ] ( label: "b" , selected: ( tone == "calm" || open ) )
  HostTable [ badge-table ] ( dir: slot: dir ) { HostTableBody { Row { Box ( table-cell-role: data ) { Text ( content: "x" ) } } } }
  For ( each: slot: items , as: loud ) { Text [ badge-looped ] ( content: slot: tone ) }
} }"#,
        );
        write_component(&lib, "Panel", panel_mil, panel_mll);
        (tmp, LayoutPackageResolver::new(vec![pkgs]))
    }

    fn prop_value(root: &LayoutNode, part: &str, prop: &str) -> Option<LayoutPropValue> {
        props_of(root, part)
            .iter()
            .find(|p| p.name == prop)
            .map(|p| p.value.clone())
    }

    fn has_part(node: &LayoutNode, part: &str) -> bool {
        node.part_name.as_deref() == Some(part) || node.children.iter().any(|c| has_part(c, part))
    }

    /// Exploit: `tone: authToken` at a call site inside a package is not a
    /// value of `tone`, so it is an identifier, and Panel declares nothing
    /// by that name. Before, the `one-of` exemption let it through and
    /// Badge read it as React `{authToken}`.
    #[test]
    fn a_one_of_binding_that_is_not_a_declared_value_is_an_identifier() {
        let (_tmp, resolver) = badge_package(
            "component Panel { slot rows : list<text> ; }",
            "layout Panel { Badge ( tone: authToken , dir: ltr , open: false , items: slot: rows ) }",
        );
        let mut layout =
            consumer_layout("layout ProbeApp { pkg::mosaic-pkg-lib::Panel ( rows: slot: names ) }");

        let error = only(
            resolver
                .resolve(&mut layout)
                .expect_err("authToken is free"),
        );
        assert!(
            matches!(&error,
                LayoutResolveError::UndeclaredExpressionIdentifier { identifier, expression, .. }
                    if identifier == "authToken" && expression == "tone: authToken"),
            "{error:?}"
        );
    }

    /// `tone: loud` is the string "loud" everywhere in Badge: text in
    /// `content:`, a string in comparisons (which then fold, so React never
    /// sees TS2367's `"loud" == "calm"`), the value under a `For` that binds
    /// a variable called `loud`, and a `Keyword` again in the enum prop
    /// `dir:`.
    #[test]
    fn a_one_of_value_is_a_string_and_never_a_variable() {
        let (_tmp, resolver) = badge_package(
            "component Panel { slot is-open : bool ; slot rows : list<text> ; }",
            "layout Panel { Badge ( tone: loud , dir: rtl , open: slot: is-open , items: slot: rows ) }",
        );
        let mut layout = consumer_layout(
            "layout ProbeApp { pkg::mosaic-pkg-lib::Panel ( is-open: slot: flag , rows: slot: names ) }",
        );

        resolver.resolve(&mut layout).expect("resolves");
        let root = &layout.root;
        let text = |v: &str| Some(LayoutPropValue::String(v.to_string()));
        assert_eq!(prop_value(root, "badge-tone", "content"), text("loud"));
        assert_eq!(prop_value(root, "badge-looped", "content"), text("loud"));
        assert!(
            has_part(root, "badge-loud"),
            "( \"loud\" == \"loud\" ) keeps its branch"
        );
        assert!(
            !has_part(root, "badge-calm"),
            "( \"loud\" == \"calm\" ) drops its branch"
        );
        assert_eq!(
            prop_value(root, "badge-button", "selected"),
            Some(LayoutPropValue::Expr("( flag )".to_string()))
        );
        assert_eq!(
            prop_value(root, "badge-table", "dir"),
            Some(LayoutPropValue::Keyword("rtl".to_string()))
        );
    }

    /// Forwarding: Panel's `Badge ( tone: tone )` names Panel's own `tone`,
    /// which is not a value of Badge's. It becomes `slot: tone`, so Panel's
    /// binding reaches Badge. Before, it stayed `Keyword("tone")` and Badge
    /// read the consumer's `tone`.
    #[test]
    fn forwarding_a_one_of_slot_binds_through() {
        let (_tmp, resolver) = badge_package(
            r#"component Panel {
  slot tone : one-of calm loud ;
  slot dir : one-of ltr rtl ;
  slot rows : list<text> ;
}"#,
            "layout Panel { Badge ( tone: tone , dir: dir , open: false , items: slot: rows ) }",
        );
        let mut layout = consumer_layout(
            "layout ProbeApp { pkg::mosaic-pkg-lib::Panel ( tone: calm , dir: rtl , rows: slot: names ) }",
        );
        resolver.resolve(&mut layout).expect("resolves");
        let root = &layout.root;
        assert_eq!(
            prop_value(root, "badge-tone", "content"),
            Some(LayoutPropValue::String("calm".to_string()))
        );
        assert!(has_part(root, "badge-calm") && !has_part(root, "badge-loud"));
        assert_eq!(
            prop_value(root, "badge-table", "dir"),
            Some(LayoutPropValue::Keyword("rtl".to_string()))
        );
    }

    /// Tie-break: a declared value that also names one of the caller's
    /// slots is the value. Panel has a slot called `loud`; `tone: loud`
    /// still means the value `loud`.
    #[test]
    fn a_declared_value_wins_over_a_caller_slot_of_the_same_name() {
        let (_tmp, resolver) = badge_package(
            "component Panel { slot loud : text ; slot rows : list<text> ; }",
            "layout Panel { Badge ( tone: loud , dir: ltr , open: true , items: slot: rows ) }",
        );
        let mut layout = consumer_layout(
            r#"layout ProbeApp { pkg::mosaic-pkg-lib::Panel ( loud: "SLOT" , rows: slot: names ) }"#,
        );
        resolver.resolve(&mut layout).expect("resolves");
        assert_eq!(
            prop_value(&layout.root, "badge-tone", "content"),
            Some(LayoutPropValue::String("loud".to_string()))
        );
    }

    /// In the consumer's own scope a bare NAME that is not a value is the
    /// consumer's variable (here its loop binding) and stays one.
    #[test]
    fn a_consumer_loop_variable_bound_to_a_one_of_slot_stays_a_variable() {
        let (_tmp, resolver) = badge_package("component Panel { }", "layout Panel { Box { } }");
        let mut layout = consumer_layout(
            r#"layout ProbeApp { Column { For ( each: slot: tones , as: t ) {
  pkg::mosaic-pkg-lib::Badge ( tone: t , dir: ltr , open: false , items: slot: names )
} } }"#,
        );
        resolver.resolve(&mut layout).expect("resolves");
        assert_eq!(
            prop_value(&layout.root, "badge-tone", "content"),
            Some(LayoutPropValue::Keyword("t".to_string()))
        );
    }

    #[test]
    fn constant_expressions_fold_only_what_every_backend_agrees_on() {
        let cases = [
            (
                r#"( "loud" == "calm" )"#,
                FoldOutcome::Value(LayoutPropValue::Keyword("false".into())),
            ),
            (
                r#"( "calm" == "calm" )"#,
                FoldOutcome::Value(LayoutPropValue::Keyword("true".into())),
            ),
            (r#"( "a" != "b" && x )"#, FoldOutcome::Text("( x )".into())),
            (
                r#"( "a" == "b" || x . y [ 0 ] )"#,
                FoldOutcome::Text("( x . y [ 0 ] )".into()),
            ),
            (
                r#"( x || "a" == "b" )"#,
                FoldOutcome::Text("( x || false )".into()),
            ),
            (
                "( ! ( 2 > 1 ) )",
                FoldOutcome::Value(LayoutPropValue::Keyword("false".into())),
            ),
            (r#"( "a" == 1 )"#, FoldOutcome::Unchanged),
            ("( x == 1 )", FoldOutcome::Unchanged),
            (r#"( "q\"uote" )"#, FoldOutcome::Unchanged),
            ("( slot : open && true )", FoldOutcome::Unchanged),
            (
                "( true && slot : open )",
                FoldOutcome::Text("( slot : open )".into()),
            ),
            ("( 1 = 2 )", FoldOutcome::Unchanged),
        ];
        for (expr, expected) in cases {
            assert_eq!(fold_expression(expr), expected, "{expr}");
        }
    }

    // ---- Version requirements (mosaic-package-publishing-v1 §7.4) ----

    /// A search path holding `mosaic-pkg-kit` at `kit_version`, and a
    /// consumer at `<tmp>/user` that requires it as `requirement`.
    fn versioned_world(kit_version: &str, requirement: &str) -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        let kit = pkgs.join("mosaic-pkg-kit");
        fs::create_dir_all(&kit).unwrap();
        write_versioned_manifest(&kit, "mosaic-pkg-kit", kit_version, &["Kit"], &[]);
        write_component(
            &kit,
            "Kit",
            "component Kit { }",
            "layout Kit { Text [ kit ] ( content: \"kit\" ) }",
        );
        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(&user, "user-app", &[], &[("mosaic-pkg-kit", requirement)]);
        (tmp, pkgs, user)
    }

    fn build_mismatch(kit_version: &str, requirement: &str) -> VersionMismatch {
        let (_tmp, pkgs, user) = versioned_world(kit_version, requirement);
        match build(&user, &[pkgs]) {
            Err(ResolveError::VersionMismatch(mismatch)) => *mismatch,
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn build_accepts_a_caret_compatible_dependency() {
        for (found, requirement) in [
            ("0.16.0", "0.16.0"),
            ("0.16.3", "0.16.0"),
            ("1.9.0", "1.2.3"),
            ("0.16.0", "=0.16.0"),
            ("1.0.0", "1.0.0-rc.1"),
        ] {
            let (_tmp, pkgs, user) = versioned_world(found, requirement);
            let r = build(&user, &[pkgs])
                .unwrap_or_else(|e| panic!("{requirement:?} should accept {found}: {e}"));
            assert!(r.knows("Kit"));
        }
    }

    #[test]
    fn build_rejects_a_caret_incompatible_dependency_and_names_both_ends() {
        // The exact shape of the stale toolkit pins this check exists for.
        let mismatch = build_mismatch("0.16.0", "0.11.0");
        assert_eq!(mismatch.dependent, "user-app");
        assert_eq!(mismatch.dependency, "mosaic-pkg-kit");
        assert_eq!(mismatch.requirement.to_string(), "0.11.0");
        assert_eq!(mismatch.found, "0.16.0");
        assert!(mismatch.found_at.ends_with("mosaic-pkg-kit"));
        assert!(mismatch.found_at.is_absolute());
        assert!(mismatch
            .dependent_manifest
            .ends_with("user/mosaic-package.toml"));

        let message = ResolveError::VersionMismatch(Box::new(mismatch)).to_string();
        for needle in [
            "user-app requires mosaic-pkg-kit \"0.11.0\" (>=0.11.0, <0.12.0)",
            "found mosaic-pkg-kit 0.16.0",
            "update the requirement in",
        ] {
            assert!(
                message.contains(needle),
                "{needle:?} missing from: {message}"
            );
        }

        // An older dependency than required fails too, and so does crossing
        // a major version.
        assert_eq!(build_mismatch("0.15.9", "0.16.0").found, "0.15.9");
        assert_eq!(build_mismatch("2.0.0", "1.2.3").found, "2.0.0");
    }

    #[test]
    fn build_rejects_an_exact_mismatch() {
        let mismatch = build_mismatch("0.16.1", "=0.16.0");
        assert_eq!(mismatch.requirement.range(), "=0.16.0");
        assert!(mismatch.to_string().contains("\"=0.16.0\" (=0.16.0)"));
    }

    #[test]
    fn build_never_matches_an_unrequested_pre_release() {
        let mismatch = build_mismatch("0.17.0-rc.1", "0.16.0");
        assert_eq!(mismatch.found, "0.17.0-rc.1");
        build_mismatch("0.16.1-beta", "0.16.0");
    }

    #[test]
    fn build_still_reports_a_missing_dependency_as_not_found() {
        let (_tmp, _pkgs, user) = versioned_world("0.1.0", "0.1.0");
        let err = build(&user, &[]).expect_err("nothing on the search path");
        assert!(
            matches!(err, ResolveError::DependencyNotFound { ref package, .. } if package == "mosaic-pkg-kit"),
            "got {err:?}"
        );
    }

    #[test]
    fn check_dependencies_covers_the_consumer() {
        for (found, requirement, ok) in [
            ("0.16.3", "0.16.0", true),
            ("0.16.0", "=0.16.0", true),
            ("0.16.0", "0.11.0", false),
            ("0.16.1", "=0.16.0", false),
        ] {
            let (_tmp, pkgs, user) = versioned_world(found, requirement);
            let manifest_path = user.join("mosaic-package.toml");
            let manifest = mosaic_package_manifest::parse_path(&manifest_path).unwrap();
            let result = LayoutPackageResolver::new(vec![pkgs])
                .check_dependencies(&manifest, &manifest_path);
            match (ok, result) {
                (true, Ok(())) => {}
                (false, Err(LayoutResolveError::VersionMismatch(mismatch))) => {
                    assert_eq!(mismatch.dependent, "user-app");
                    assert_eq!(mismatch.found, found);
                    assert_eq!(mismatch.dependent_manifest, manifest_path);
                }
                (ok, other) => panic!("{requirement:?} vs {found}: ok={ok}, got {other:?}"),
            }
        }
    }

    #[test]
    fn check_dependencies_leaves_a_missing_dependency_to_the_reference() {
        // Not on the search path: no version to compare, and no error until a
        // layout actually references it (that is Phase 1b's rule to change).
        let (_tmp, _pkgs, user) = versioned_world("0.1.0", "0.1.0");
        let manifest_path = user.join("mosaic-package.toml");
        let manifest = mosaic_package_manifest::parse_path(&manifest_path).unwrap();
        LayoutPackageResolver::new(vec![])
            .check_dependencies(&manifest, &manifest_path)
            .expect("a missing dependency is not a version mismatch");

        let mut layout = consumer_layout("layout Demo { pkg::mosaic-pkg-kit::Kit }");
        let err = LayoutPackageResolver::new(vec![])
            .resolve(&mut layout)
            .expect_err("the reference still fails");
        assert!(
            matches!(err, LayoutResolveError::PackageNotFound { ref package } if package == "mosaic-pkg-kit"),
            "got {err:?}"
        );
    }

    /// `mosaic-pkg-mid` (exports `Mid`, which mounts `pkg::mosaic-pkg-leaf::Leaf`)
    /// requires `mosaic-pkg-leaf` as `requirement`; the leaf is at
    /// `leaf_version`.
    fn transitive_world(leaf_version: &str, requirement: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        let leaf = pkgs.join("mosaic-pkg-leaf");
        fs::create_dir_all(&leaf).unwrap();
        write_versioned_manifest(&leaf, "mosaic-pkg-leaf", leaf_version, &["Leaf"], &[]);
        write_component(
            &leaf,
            "Leaf",
            "component Leaf { }",
            "layout Leaf { Text [ leaf ] ( content: \"leaf\" ) }",
        );
        let mid = pkgs.join("mosaic-pkg-mid");
        fs::create_dir_all(&mid).unwrap();
        write_manifest(
            &mid,
            "mosaic-pkg-mid",
            &["Mid"],
            &[("mosaic-pkg-leaf", requirement)],
        );
        write_component(
            &mid,
            "Mid",
            "component Mid { }",
            "layout Mid { Column [ mid ] { pkg::mosaic-pkg-leaf::Leaf } }",
        );
        (tmp, pkgs)
    }

    #[test]
    fn the_layout_inliner_checks_every_package_it_inlines_from() {
        let (_tmp, pkgs) = transitive_world("0.2.0", "0.2.0");
        let mut layout = consumer_layout("layout Demo { pkg::mosaic-pkg-mid::Mid }");
        LayoutPackageResolver::new(vec![pkgs])
            .resolve(&mut layout)
            .expect("a satisfied requirement inlines as before");
        assert!(first_qualified_tag(&layout.root).is_none());

        for (leaf_version, requirement) in [("0.3.0", "0.2.0"), ("0.2.1", "=0.2.0")] {
            let (_tmp, pkgs) = transitive_world(leaf_version, requirement);
            let mut layout = consumer_layout("layout Demo { pkg::mosaic-pkg-mid::Mid }");
            let err = LayoutPackageResolver::new(vec![pkgs])
                .resolve(&mut layout)
                .expect_err("the middle package's requirement is not met");
            match err {
                LayoutResolveError::VersionMismatch(mismatch) => {
                    assert_eq!(mismatch.dependent, "mosaic-pkg-mid");
                    assert_eq!(mismatch.dependency, "mosaic-pkg-leaf");
                    assert_eq!(mismatch.found, leaf_version);
                    assert!(mismatch
                        .dependent_manifest
                        .ends_with("mosaic-pkg-mid/mosaic-package.toml"));
                }
                other => panic!("expected VersionMismatch, got {other:?}"),
            }
        }
    }

    // ---- The repository's own packages, apps and manifests ----

    /// `code/` in this checkout: this crate is `code/packages/rust/<crate>`.
    fn repository_code_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("code/ exists")
    }

    /// Every `mosaic-package.toml` under `dir`, skipping build output and
    /// installed dependencies (which may hold copies of the same files).
    fn collect_manifests(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if name.starts_with('.')
                    || name == "target"
                    || name.starts_with("target-")
                    || name == "node_modules"
                    || name == "build"
                    || name == "dist"
                {
                    continue;
                }
                collect_manifests(&path, out);
            } else if file_type.is_file() && name == "mosaic-package.toml" {
                out.push(path);
            }
        }
    }

    /// Every layout of every exported component in the repository binds
    /// every required slot of every package component it mounts, and every
    /// exported component passes the callee scope check (no free names in
    /// its expressions, no read of its child-mount slot outside the mount).
    ///
    /// The latent-capture guard: before `SlotBindingMissing`, a consumer
    /// could leave a required slot unbound and still build, if it happened
    /// to have a slot of the same name (P3) or the package read the slot
    /// only in an expression (P12). This runs the check on each layout
    /// (`<C>.mll` and every `<C>.<variant>.mll`) as a package build sees it:
    /// local sibling references qualified first, then searched for in the
    /// package's parent directory, `code/packages` and `code/packages/mosaic`.
    /// The check descends into every package layout it reaches, so packages
    /// are covered as callees as well as consumers.
    ///
    /// When it was added, it found no layout in the repository relying on
    /// an unbound required slot, and no export with a scope error.
    #[test]
    fn every_repository_layout_binds_every_required_slot() {
        let code = repository_code_root();
        let mut manifests = Vec::new();
        collect_manifests(&code, &mut manifests);
        manifests.sort();
        assert!(
            manifests.len() >= 40,
            "expected the repository's ~50 manifests, found {} under {}",
            manifests.len(),
            code.display()
        );

        let packages = code.join("packages");
        let mut resolved_layouts = 0;
        let mut call_sites = 0;
        let mut scoped_components = 0;
        let mut failures = Vec::new();
        for manifest_path in &manifests {
            let Ok(manifest) = mosaic_package_manifest::parse_path(manifest_path) else {
                // Unparseable manifests are another test's business.
                continue;
            };
            let package_root = manifest_path.parent().unwrap();
            let src = package_root.join("src");
            let resolver = LayoutPackageResolver::new(vec![
                package_root.parent().unwrap().to_path_buf(),
                packages.clone(),
                packages.join("mosaic"),
            ]);
            for component in &manifest.components.exports {
                // Every export, mounted or not, must pass the callee scope
                // check: it is what any future consumer would inline.
                match resolver.resolve_component(&manifest.package.name, component) {
                    Ok(_) => scoped_components += 1,
                    Err(e @ LayoutResolveError::UndeclaredExpressionIdentifier { .. })
                    | Err(e @ LayoutResolveError::ChildMountSlotReferenced { .. })
                    | Err(e @ LayoutResolveError::Multiple { .. }) => {
                        failures.push(format!("{}:\n  {e}", package_root.display()));
                    }
                    // Not compiling on its own is the package build's business.
                    Err(_) => {}
                }
                // A component that does not compile on its own is its package
                // build's failure to report; here it is skipped. The count
                // assertions below catch a walk that skips everything.
                let Ok(mil) = fs::read_to_string(src.join(format!("{component}.mil"))) else {
                    continue;
                };
                let Ok(model) = mosmodel_compiler::compile(&mil) else {
                    continue;
                };
                let mut layouts: Vec<PathBuf> = fs::read_dir(&src)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| {
                        let name = path.file_name().unwrap().to_string_lossy();
                        name == format!("{component}.mll")
                            || (name.starts_with(&format!("{component}."))
                                && name.ends_with(".mll")
                                && name.matches('.').count() == 2)
                    })
                    .collect();
                layouts.sort();
                for layout_path in layouts {
                    let source = fs::read_to_string(&layout_path).unwrap();
                    let Ok(mut out) =
                        moslayout_compiler::compile(&source, Some(&model.descriptor_json))
                    else {
                        continue;
                    };
                    qualify_local_refs_except(
                        &mut out.def.root,
                        component,
                        &manifest.package.name,
                        &manifest.components.exports,
                    );
                    resolved_layouts += 1;
                    call_sites += count_package_refs(&out.def.root);
                    if let Err(e) = resolver.check_required_bindings(&out.def) {
                        failures.push(format!("{}:\n  {e}", layout_path.display()));
                    }
                }
            }
        }

        assert!(
            resolved_layouts >= 80,
            "expected the repository's ~90 exported layouts, checked {resolved_layouts}"
        );
        assert!(
            call_sites >= 50,
            "expected the repository's ~57 package call sites, found {call_sites}"
        );
        assert!(
            scoped_components >= 80,
            "expected the repository's ~89 exports to load, loaded {scoped_components}"
        );
        assert!(
            failures.is_empty(),
            "layouts with unbound required slots, or components with scope errors:\n{}",
            failures.join("\n")
        );
    }

    /// The `pkg::` call sites written directly in `node`'s tree.
    fn count_package_refs(node: &LayoutNode) -> usize {
        usize::from(node.package_ref().is_some())
            + node.children.iter().map(count_package_refs).sum::<usize>()
    }

    /// The artifact builder's `qualify_local_component_references`: a
    /// package's own layout names its sibling exports without `pkg::`.
    fn qualify_local_refs_except(
        node: &mut LayoutNode,
        current: &str,
        pkg: &str,
        exports: &[String],
    ) {
        if node.package_ref().is_none()
            && node.tag != current
            && exports.iter().any(|export| export == &node.tag)
        {
            node.tag = format!("pkg::{pkg}::{}", node.tag);
        }
        for child in &mut node.children {
            qualify_local_refs_except(child, current, pkg, exports);
        }
    }

    /// Every `[dependencies]` requirement in the repository is satisfied by
    /// the package a build would find, under both locators.
    ///
    /// The stale-pin guard: before §7.4 a requirement was never compared, and
    /// five drifted (`mosaic-pkg-toolkit = "0.11.0"` against 0.16.0). A
    /// dependency release that breaks the caret range, or a requirement that
    /// names a version not on disk, now fails here with the file to fix.
    ///
    /// Search paths are the ones `mosaic-package-artifact-builder` derives
    /// for a package: its parent directory, `code/packages` and
    /// `code/packages/mosaic`.
    #[test]
    fn every_repository_requirement_is_satisfied() {
        let code = repository_code_root();
        let mut manifests = Vec::new();
        collect_manifests(&code, &mut manifests);
        manifests.sort();
        assert!(
            manifests.len() >= 40,
            "expected the repository's ~50 manifests, found {} under {}",
            manifests.len(),
            code.display()
        );

        let packages = code.join("packages");
        let mut checked_requirements = 0;
        let mut failures = Vec::new();
        for manifest_path in &manifests {
            let manifest = match mosaic_package_manifest::parse_path(manifest_path) {
                Ok(manifest) => manifest,
                Err(e) => {
                    failures.push(format!("{}: {e}", manifest_path.display()));
                    continue;
                }
            };
            if manifest.dependencies.is_empty() {
                continue;
            }
            checked_requirements += manifest.dependencies.len();
            let package_root = manifest_path.parent().unwrap();
            let search_paths = vec![
                package_root.parent().unwrap().to_path_buf(),
                packages.clone(),
                packages.join("mosaic"),
            ];

            // Directory-name locator (`build`, every declared dependency).
            if let Err(e) = build(package_root, &search_paths) {
                failures.push(format!("build({}): {e}", package_root.display()));
            }
            // Manifest-name locator (what a package build inlines through).
            // It skips a dependency it cannot find, so also require that
            // every declared dependency is found by name.
            let layout = LayoutPackageResolver::new(search_paths);
            if let Err(e) = layout.check_dependencies(&manifest, manifest_path) {
                failures.push(format!("layout({}): {e}", package_root.display()));
            }
            for dependency in manifest.dependencies.keys() {
                if let Err(e) = layout.locate_package(dependency) {
                    failures.push(format!("layout({}): {e}", package_root.display()));
                }
            }
        }

        assert!(
            checked_requirements >= 20,
            "expected the repository's ~26 requirements, checked {checked_requirements}"
        );
        assert!(
            failures.is_empty(),
            "unsatisfied or unparseable manifests:\n{}",
            failures.join("\n")
        );
    }
}
