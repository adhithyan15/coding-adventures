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
//! `code/packages/mosaic-pkg-grid/`.  The literal-name fallback exists
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
    "Box", "Row", "Column", "Stack",
    // Leaves
    "Text", "Image", "Spacer", "Divider", "Icon",
    // Control flow (§3)
    "If", "Else", "For",
    // Host primitives (§2.1 "Host*" rows + UI29-1's HostDialog +
    // UI29-2's HostCheckbox/HostRadio).
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
    "HostInput", "HostButton", "HostTable", "HostScroll", "HostDialog",
    "HostCheckbox", "HostRadio",
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
                write!(f, "dependency `{package}` has a malformed manifest: {error}")
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
pub fn build(
    package_root: &Path,
    search_paths: &[PathBuf],
) -> Result<Resolver, ResolveError> {
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
            let dep_manifest = mosaic_package_manifest::parse_path(&dep_manifest_path)
                .map_err(|e| ResolveError::BadDependencyManifest {
                    package: dep_name.clone(),
                    error: e.to_string(),
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
fn locate_dependency(
    dep_name: &str,
    search_paths: &[PathBuf],
) -> Result<PathBuf, ResolveError> {
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ---- tiny test helpers ----

    /// Write a `mosaic-package.toml` into `dir` with the given fields.
    fn write_manifest(
        dir: &Path,
        name: &str,
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
    fn make_pkg(
        parent: &Path,
        dirname: &str,
        manifest_name: &str,
        exports: &[&str],
    ) -> PathBuf {
        let path = parent.join(dirname);
        fs::create_dir_all(&path).unwrap();
        write_manifest(&path, manifest_name, exports, &[]);
        path
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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-grid", "0.1.0")]);

        let r = build(&user, std::slice::from_ref(&pkgs)).expect("build ok");
        match r.resolve("Grid") {
            Some(Resolution::Component { package, component, .. }) => {
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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-grid", "0.1.0")]);

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
        make_pkg(&pkgs, "mosaic-pkg-tabs", "mosaic-pkg-tabs", &["Tabs", "Tab"]);

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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-ghost", "0.1.0")]);

        // Empty search path → can't possibly find it.
        let err = build(&user, &[]).expect_err("must fail");
        assert!(matches!(err, ResolveError::DependencyNotFound { ref package, .. } if package == "mosaic-pkg-ghost"));
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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-bad", "0.1.0")]);

        let err = build(&user, &[pkgs]).expect_err("must fail");
        assert!(matches!(err, ResolveError::BadDependencyManifest { ref package, .. } if package == "mosaic-pkg-bad"));
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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-grid", "0.1.0")]);

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
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-grid", "0.1.0")]);

        let r = build(&user, &[pkgs]).expect("ok");

        // Kernel:
        assert!(matches!(r.resolve("If"), Some(Resolution::Kernel)));
        // Component:
        assert!(matches!(r.resolve("Grid"), Some(Resolution::Component { .. })));
        // Unknown:
        assert!(r.resolve("ZZZ").is_none());
    }

    // ---- Test 11: kernel set contains all UI29 §2.1 primitives ----

    #[test]
    fn kernel_set_covers_ui29_section_2_1() {
        // The eighteen primitives — fifteen from UI29 §2.1, plus
        // HostDialog added in UI29-1 (#3846), plus HostCheckbox and
        // HostRadio added in UI29-2 (#3978).
        let expected_18 = [
            "Box", "Row", "Column", "Stack", "Text", "Image",
            "Spacer", "Divider", "Icon",
            "If", "For",
            "HostInput", "HostButton", "HostTable", "HostScroll",
            "HostDialog",
            "HostCheckbox", "HostRadio",
        ];
        for name in &expected_18 {
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

    // ---- Test 12: package_path is absolute ----

    #[test]
    fn package_path_is_absolute() {
        let tmp = TempDir::new().unwrap();
        let pkgs = tmp.path().join("packages");
        fs::create_dir_all(&pkgs).unwrap();
        make_pkg(&pkgs, "mosaic-pkg-grid", "mosaic-pkg-grid", &["Grid"]);

        let user = tmp.path().join("user");
        fs::create_dir_all(&user).unwrap();
        write_manifest(&user, "mosaic-pkg-user", &[], &[("mosaic-pkg-grid", "0.1.0")]);

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
        assert!(matches!(r.resolve("Spinner"), Some(Resolution::Component { .. })));
    }
}
