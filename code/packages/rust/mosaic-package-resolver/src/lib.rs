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
    /// Several independent errors, reported together so one build shows
    /// every call site to fix rather than the first. Never nested, never
    /// empty, never a single error (that is reported bare).
    Multiple {
        errors: Vec<LayoutResolveError>,
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
            Self::Multiple { errors } => {
                write!(f, "{} errors resolving package references:", errors.len())?;
                for (index, error) in errors.iter().enumerate() {
                    write!(f, "\n  {}. {error}", index + 1)?;
                }
                Ok(())
            }
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
}

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

        let mut bindings = resolved.default_bindings;
        bindings.extend(build_binding_map(&call_props));
        rewrite_bindings(target, &bindings);
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

    fn resolve_component(
        &self,
        pkg: &str,
        comp: &str,
    ) -> Result<ResolvedComponent, LayoutResolveError> {
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

        let resolved = ResolvedComponent {
            layout: layout_out.def,
            default_bindings: build_default_binding_map(&mosmodel_out.component.slots),
            slots: mosmodel_out.component.slots,
        };
        self.cache.lock().unwrap().insert(key, resolved.clone());
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
            checked: HashSet::new(),
        };
        let scope = CallScope {
            consumer: layout.component_name.clone(),
            local_package: None,
        };
        let mut path = Vec::new();
        self.check_tree(&layout.root, None, &scope, &mut path, &mut walk);

        let mut errors = walk.errors;
        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.remove(0)),
            _ => Err(LayoutResolveError::Multiple { errors }),
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
                    for slot in missing_required_slots(&resolved.slots, &node.props) {
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
                Err(error) => walk.push(error),
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
                walk.push(error);
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
    /// Every error, in walk order, without duplicates.
    errors: Vec<LayoutResolveError>,
    /// The package components whose own layouts have been checked.
    checked: HashSet<(String, String)>,
}

impl BindingCheck {
    /// Record `error` unless it is already recorded. A package that cannot
    /// be found is reached from every call site that names it, and one
    /// report is enough.
    fn push(&mut self, error: LayoutResolveError) {
        if !self.errors.contains(&error) {
            self.errors.push(error);
        }
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
/// A slot is required when it has no default (UI13 §1), except a `node` or
/// `list<node>` slot: that is filled by the caller's children, never by a
/// prop, and an unfilled child mount is removed (UI29-2).
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
/// slot body  : list<node> ;   not bound                     → mount, fine
/// ```
fn missing_required_slots<'a>(
    slots: &'a [mosmodel_compiler::SlotDecl],
    call_props: &[LayoutProp],
) -> Vec<&'a mosmodel_compiler::SlotDecl> {
    let bound: HashSet<&str> = call_props.iter().map(|prop| prop.name.as_str()).collect();
    slots
        .iter()
        .filter(|slot| {
            slot.required
                && !is_child_content_slot(&slot.r#type)
                && !bound.contains(slot.name.as_str())
        })
        .collect()
}

/// `node` and `list<node>`: slots the caller fills with children.
fn is_child_content_slot(slot_type: &mosmodel_compiler::SlotType) -> bool {
    use mosmodel_compiler::{ListInnerType, SlotType};
    match slot_type {
        SlotType::Node => true,
        SlotType::List(inner) => matches!(**inner, ListInnerType::Node),
        _ => false,
    }
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

fn rewrite_bindings(node: &mut LayoutNode, bindings: &HashMap<String, LayoutPropValue>) {
    // Dependency events are private unless the call site forwards them. Leaving
    // an omitted emit here leaks its name into the consuming component's scope.
    // Caller-authored child content is spliced in after this traversal.
    node.props.retain(|prop| match &prop.value {
        LayoutPropValue::EmitRef(name) => bindings.contains_key(name),
        _ => true,
    });
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

    if node.children.iter().any(|child| literal_condition(child).is_some()) {
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
            let (tag, part) = if literal == "true" { ("Column", kept) } else { ("Row", kept) };
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
        let tags: Vec<_> = layout.root.children.iter().map(|c| c.tag.as_str()).collect();
        assert_eq!(tags, vec!["If", "Else", "Text"]);
    }

    #[test]
    fn constant_fold_drops_an_unmatched_false_branch_and_keeps_expressions() {
        let node = |tag: &str, when: Option<LayoutPropValue>, children: Vec<LayoutNode>| LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props: when
                .into_iter()
                .map(|value| LayoutProp { name: "when".to_string(), value })
                .collect(),
            children,
        };
        let leaf = |tag: &str| node(tag, None, vec![]);
        let mut root = node(
            "Column",
            None,
            vec![
                node("If", Some(LayoutPropValue::Keyword("false".into())), vec![leaf("A")]),
                leaf("B"),
                node("If", Some(LayoutPropValue::Expr(" true ".into())), vec![leaf("C"), leaf("D")]),
                node("Else", None, vec![leaf("E")]),
                node("If", Some(LayoutPropValue::Expr("i == 0".into())), vec![leaf("F")]),
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
            props: vec![LayoutProp { name: "when".to_string(), value }],
            children: vec![],
        };
        assert_eq!(literal_condition(&when(LayoutPropValue::String(String::new()))), Some(false));
        assert_eq!(literal_condition(&when(LayoutPropValue::String("Inbox".into()))), Some(true));
        assert_eq!(literal_condition(&when(LayoutPropValue::Number(0.0))), Some(false));
        assert_eq!(literal_condition(&when(LayoutPropValue::Number(3.0))), Some(true));
        assert_eq!(literal_condition(&when(LayoutPropValue::SlotRef("s".into()))), None);
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
            LayoutResolveError::Multiple { errors } => errors.as_slice(),
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
        let LayoutResolveError::Multiple { errors } = &error else {
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
        let LayoutResolveError::Multiple { errors } = &error else {
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
        let LayoutResolveError::Multiple { errors } = &error else {
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

    // ---- The repository's own packages and apps ----

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
    /// every required slot of every package component it mounts.
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
    /// an unbound required slot.
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
            failures.is_empty(),
            "layouts with unbound required slots:\n{}",
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
}
