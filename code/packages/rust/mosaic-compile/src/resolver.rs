//! # `mosaic-compile`'s package-reference resolver — UI34 §5 (PR-3).
//!
//! ## What it does
//!
//! Walks a `moslayout_compiler::LayoutDef` after analysis and substitutes
//! every `pkg::P::C` reference node with the *resolved* sub-tree compiled
//! from package `P`'s component `C`.  After resolution, no `tag` in the
//! tree starts with `"pkg::"` — every node is either a kernel primitive
//! (UI29) or a same-file local component reference (UI14 §6).  The
//! backend emitter never sees a qualified tag.
//!
//! ## Two-stage algorithm — substitute then rewire
//!
//! For each `pkg::P::C` node found on a depth-first walk:
//!
//!   1. **Locate** package `P` by scanning every configured
//!      `--package-search-path` directory for a child that contains a
//!      `mosaic-package.toml` whose `package.name == P`.
//!   2. **Compile** `P`'s component `C` — read `<P-root>/src/<C>.mil`,
//!      `<P-root>/src/<C>.mll`, optionally `<P-root>/src/<C>.dark.msl`,
//!      and run them through the same `mosmodel_compiler` /
//!      `moslayout_compiler` pipeline the consumer uses.
//!   3. **Substitute** the resolved `LayoutDef.root` for the consumer's
//!      `pkg::P::C` node — its `tag`, `part_name`, `props`, and
//!      `children` replace the qualified node's fields wholesale.
//!   4. **Rewire** every `slot:` / `emit:` reference inside the
//!      substituted sub-tree using the call-site's prop bindings.  If
//!      the consumer wrote `viewport-rows: slot: my-rows`, every
//!      `slot: viewport-rows` in `C`'s body becomes `slot: my-rows` in
//!      the substituted output.
//!   5. **Qualify-or-pass-through unqualified children.**  Inside `C`'s
//!      body, sibling components are written by bare name (the package
//!      "self-reference" convention from UI28-1 §3).  The resolver
//!      rewrites every bare reference to a sibling component into a
//!      `pkg::P::<sibling>` reference so the next recursion pass
//!      resolves it.  Kernel primitives and non-exported tags pass
//!      through unchanged.
//!
//! ## Cycle detection
//!
//! A package may reference its siblings but not itself transitively.
//! The resolver tracks an in-flight `HashSet<(P, C)>` and errors
//! [`ResolveError::CircularPackageReference`] on the second visit.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mosaic_package_manifest::MosaicPackage;
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp, LayoutPropValue};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All ways the resolver can fail.  Each variant carries enough context
/// for an editor or language-server to underline the offending
/// `pkg::P::C` reference in the consumer's source.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// No `mosaic-package.toml` found under any `--package-search-path`
    /// with a `package.name` matching the qualified reference's package.
    PackageNotFound { package: String },
    /// The package was found but does not export the requested component.
    ComponentNotExported {
        package: String,
        component: String,
        available: Vec<String>,
    },
    /// One of the three source files (`.mil` / `.mll` / `.dark.msl`)
    /// that a component is expected to ship is missing.  The `.msl` is
    /// optional today — only the `.mil` and `.mll` are required.
    ComponentSourceMissing {
        package: String,
        component: String,
        file: PathBuf,
    },
    /// A cycle was detected — `A → B → A` is not a legal package graph.
    /// The `cycle` vector lists the `(package, component)` pairs in the
    /// order they were visited.
    CircularPackageReference {
        cycle: Vec<(String, String)>,
    },
    /// The package's manifest file failed to parse.  Path + error
    /// detail surface for human debugging.
    ManifestParseError {
        package: String,
        path: PathBuf,
        detail: String,
    },
    /// The consumer-side compiler error during recursive compilation of
    /// a package's component.  Surfaces the upstream compiler's error
    /// list verbatim.
    NestedCompileError {
        package: String,
        component: String,
        detail: String,
    },
    /// I/O failure while reading a component's source file.
    IoError {
        package: String,
        component: String,
        path: PathBuf,
        detail: String,
    },
    /// Defence against the symlink-escape attack: a component source
    /// path canonicalises to a location outside the package's own
    /// directory.  This protects build hosts where an attacker can
    /// place a package on the search path — without this check, a
    /// `src/Foo.mll` symlink pointing at `~/.aws/credentials` would
    /// be read by the resolver and the contents echoed through the
    /// parser's error messages.  See UI34 §5.2 plus the PR-3 security
    /// review.
    SymlinkEscape {
        package: String,
        component: String,
        attempted: PathBuf,
        canonical: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Resolver — the public type
// ---------------------------------------------------------------------------

/// Walks a `LayoutDef`, substituting every `pkg::P::C` reference with
/// the resolved package source.  Wired into `mosaic-compile`'s pipeline
/// right after `moslayout_compiler::compile()` and before the backend
/// emitter runs.
///
/// **Caching.**  Each `(package, component)` pair resolves to the same
/// `LayoutDef` regardless of how many call sites reference it — the
/// resolver memoises results in a `Mutex<HashMap>` so a `mosaic-pkg-grid`
/// `Grid` referenced from twenty consumer layouts is compiled exactly
/// once per `mosaic-compile` invocation.
pub struct PackageResolver {
    search_paths: Vec<PathBuf>,
    cache: Mutex<HashMap<(String, String), LayoutDef>>,
}

impl PackageResolver {
    /// Build a resolver with the given package-search roots.  Each
    /// element is a directory that contains zero-or-more package
    /// directories (the latter being directories with a
    /// `mosaic-package.toml` at the root).
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Mutate the layout in place, substituting every qualified
    /// reference.  Returns `Ok(())` on success or the first error
    /// encountered.
    ///
    /// Idempotent — running the resolver twice on an already-resolved
    /// layout is a no-op.
    pub fn resolve(&self, layout: &mut LayoutDef) -> Result<(), ResolveError> {
        let mut visiting: Vec<(String, String)> = Vec::new();
        self.resolve_node(&mut layout.root, &mut visiting)
    }

    // -----------------------------------------------------------------
    // Internal recursion
    // -----------------------------------------------------------------

    fn resolve_node(
        &self,
        node: &mut LayoutNode,
        visiting: &mut Vec<(String, String)>,
    ) -> Result<(), ResolveError> {
        // First, peel off any chain of qualified references rooted at
        // this node.  `while` (not `if`) — a resolved root might
        // itself be a `pkg::Q::Y` reference if the package's component
        // delegates straight to another package's component.
        //
        // The visiting stack holds every in-flight (package, component)
        // pair from the consumer-tree root down to the current node.
        // Cycle = the same pair re-appearing.  Push happens BEFORE
        // recursing into substituted children (so cross-package cycles
        // are caught), and pop only AFTER the entire substituted
        // subtree finishes resolving.
        let pushed: Vec<(String, String)> = {
            let mut pushed = Vec::new();
            while let Some((pkg, comp)) =
                node.package_ref().map(|(p, c)| (p.to_string(), c.to_string()))
            {
                if visiting.iter().any(|p| p == &(pkg.clone(), comp.clone())) {
                    let mut cycle = visiting.clone();
                    cycle.push((pkg, comp));
                    return Err(ResolveError::CircularPackageReference { cycle });
                }
                visiting.push((pkg.clone(), comp.clone()));
                pushed.push((pkg.clone(), comp.clone()));
                let resolved = self.resolve_component(&pkg, &comp)?;
                self.substitute(node, resolved, &pkg)?;
            }
            pushed
        };

        // Recurse into the (possibly substituted) children with the
        // visiting stack still populated so descendants see the
        // ancestor packages.
        let mut i = 0;
        while i < node.children.len() {
            self.resolve_node(&mut node.children[i], visiting)?;
            i += 1;
        }

        // Pop everything we pushed for this node.
        for _ in &pushed {
            visiting.pop();
        }
        Ok(())
    }

    /// Substitute `target`'s fields with `resolved.root`, transferring
    /// call-site props through the slot/emit rewire step.
    ///
    /// The substitution preserves a small set of consumer-side
    /// metadata:
    ///
    ///   * the consumer's `part_name`, if any, overrides the resolved
    ///     root's part_name — this lets a consumer style the
    ///     resolved sub-tree as a single addressable unit.
    ///
    /// The substitution drops:
    ///
    ///   * the qualified node's `props` (they are consumed by the
    ///     rewire step — they don't survive as props of the resolved
    ///     root).
    ///   * the qualified node's `children` (qualified components are
    ///     leaf-like today; UI28-1's children-passthrough is a
    ///     follow-up).  The children are dropped silently — a
    ///     consumer that passes children to a `pkg::P::C` reference
    ///     today will not see them rendered.  If this masks too
    ///     many real bugs, a future PR can promote this to a hard
    ///     error.
    fn substitute(
        &self,
        target: &mut LayoutNode,
        resolved: LayoutDef,
        pkg: &str,
    ) -> Result<(), ResolveError> {
        let call_props = std::mem::take(&mut target.props);
        let consumer_part = target.part_name.take();
        let _ = std::mem::take(&mut target.children); // dropped — see doc

        // Move the resolved root's fields into `target`.
        target.tag = resolved.root.tag;
        target.part_name = consumer_part.or(resolved.root.part_name);
        target.props = resolved.root.props;
        target.children = resolved.root.children;

        // Rewire slot / emit references throughout the substituted
        // sub-tree using the call-site's prop bindings.
        let bindings = build_binding_map(&call_props);
        rewrite_bindings(target, &bindings);

        // Qualify the substituted sub-tree's unqualified non-primitive
        // tags so a follow-up resolver pass finds them.  The set of
        // "valid sibling exports" comes from the package's own
        // manifest — kernel primitives and unrelated tags are left
        // alone.
        let exports = self.package_exports(pkg)?;
        qualify_local_refs(target, pkg, &exports);

        Ok(())
    }

    /// Compile a package's component into a `LayoutDef`.
    ///
    /// Cached on `(package, component)` so a single
    /// `mosaic-compile` invocation never compiles the same component
    /// twice.
    fn resolve_component(
        &self,
        pkg: &str,
        comp: &str,
    ) -> Result<LayoutDef, ResolveError> {
        let key = (pkg.to_string(), comp.to_string());
        // Cache check.
        if let Some(cached) = self.cache.lock().unwrap().get(&key).cloned() {
            return Ok(cached);
        }

        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;

        // The manifest must export this component.
        if !manifest.components.exports.iter().any(|e| e == comp) {
            return Err(ResolveError::ComponentNotExported {
                package: pkg.to_string(),
                component: comp.to_string(),
                available: manifest.components.exports.clone(),
            });
        }

        // Canonicalise the package root once.  Every component source
        // file is then required to canonicalise to a path within this
        // prefix — symlinks pointing outside the package directory
        // are rejected as `SymlinkEscape`.  See the PR-3 security
        // review (Finding 1) for the rationale.
        let canon_root = std::fs::canonicalize(&pkg_root).map_err(|e| {
            ResolveError::IoError {
                package: pkg.to_string(),
                component: comp.to_string(),
                path: pkg_root.clone(),
                detail: format!("canonicalize package root failed: {e}"),
            }
        })?;

        // The component's source files live at `src/<C>.{mil,mll,…}`.
        let src_dir = pkg_root.join("src");
        let mil_path = src_dir.join(format!("{comp}.mil"));
        let mll_path = src_dir.join(format!("{comp}.mll"));
        // Reject symlink-escapes before we read anything.
        verify_inside_package(pkg, comp, &mil_path, &canon_root)?;
        verify_inside_package(pkg, comp, &mll_path, &canon_root)?;

        let mil_src = std::fs::read_to_string(&mil_path).map_err(|e| {
            // Distinguish "missing file" from other I/O errors so the
            // diagnostic guides the user to the right fix.
            if e.kind() == std::io::ErrorKind::NotFound {
                ResolveError::ComponentSourceMissing {
                    package: pkg.to_string(),
                    component: comp.to_string(),
                    file: mil_path.clone(),
                }
            } else {
                ResolveError::IoError {
                    package: pkg.to_string(),
                    component: comp.to_string(),
                    path: mil_path.clone(),
                    detail: e.to_string(),
                }
            }
        })?;
        let mll_src = std::fs::read_to_string(&mll_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ResolveError::ComponentSourceMissing {
                    package: pkg.to_string(),
                    component: comp.to_string(),
                    file: mll_path.clone(),
                }
            } else {
                ResolveError::IoError {
                    package: pkg.to_string(),
                    component: comp.to_string(),
                    path: mll_path.clone(),
                    detail: e.to_string(),
                }
            }
        })?;

        // Compile mosmodel first so moslayout has the interface JSON.
        let mosmodel_out = mosmodel_compiler::compile(&mil_src).map_err(|errs| {
            ResolveError::NestedCompileError {
                package: pkg.to_string(),
                component: comp.to_string(),
                detail: format!("{errs:?}"),
            }
        })?;
        let layout_out = moslayout_compiler::compile(
            &mll_src,
            Some(&mosmodel_out.descriptor_json),
        )
        .map_err(|errs| ResolveError::NestedCompileError {
            package: pkg.to_string(),
            component: comp.to_string(),
            detail: format!("{errs:?}"),
        })?;

        let def = layout_out.def;
        self.cache.lock().unwrap().insert(key, def.clone());
        Ok(def)
    }

    /// Cheap lookup — `[components].exports` from the package's
    /// manifest.  Used by `qualify_local_refs` to decide whether an
    /// unqualified tag is a sibling reference (rewrite it) or a kernel
    /// primitive (leave it alone).
    fn package_exports(&self, pkg: &str) -> Result<Vec<String>, ResolveError> {
        let pkg_root = self.locate_package(pkg)?;
        let manifest = self.read_manifest(pkg, &pkg_root)?;
        Ok(manifest.components.exports)
    }

    /// Locate a package directory by scanning every search path for a
    /// child containing a `mosaic-package.toml` with the matching
    /// name.
    ///
    /// Today this is a linear scan — small numbers of packages in
    /// `code/packages/` make this acceptable.  A future
    /// optimisation can build an index from the search paths.
    fn locate_package(&self, pkg: &str) -> Result<PathBuf, ResolveError> {
        for search_root in &self.search_paths {
            let candidate_dirs = match std::fs::read_dir(search_root) {
                Ok(it) => it,
                Err(_) => continue, // a missing search root is not fatal
            };
            for entry in candidate_dirs.flatten() {
                let candidate = entry.path();
                let manifest_path = candidate.join("mosaic-package.toml");
                if !manifest_path.exists() {
                    continue;
                }
                // Read just enough to check the name.
                if let Ok(src) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(m) = mosaic_package_manifest::parse(&src) {
                        if m.package.name == pkg {
                            return Ok(candidate);
                        }
                    }
                }
            }
        }
        Err(ResolveError::PackageNotFound {
            package: pkg.to_string(),
        })
    }

    fn read_manifest(
        &self,
        pkg: &str,
        pkg_root: &Path,
    ) -> Result<MosaicPackage, ResolveError> {
        let manifest_path = pkg_root.join("mosaic-package.toml");
        let src = std::fs::read_to_string(&manifest_path).map_err(|e| {
            ResolveError::IoError {
                package: pkg.to_string(),
                component: String::new(),
                path: manifest_path.clone(),
                detail: e.to_string(),
            }
        })?;
        mosaic_package_manifest::parse(&src).map_err(|e| ResolveError::ManifestParseError {
            package: pkg.to_string(),
            path: manifest_path,
            detail: format!("{e:?}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers — small tree walkers
// ---------------------------------------------------------------------------

/// Build a map from call-site prop names to their values.  Used by
/// `rewrite_bindings` to plug call-site values into the resolved
/// sub-tree's `slot: X` / `emit: X` references.
fn build_binding_map(call_props: &[LayoutProp]) -> HashMap<String, LayoutPropValue> {
    let mut m = HashMap::with_capacity(call_props.len());
    for p in call_props {
        m.insert(p.name.clone(), p.value.clone());
    }
    m
}

/// Walk `node` and rewrite every `LayoutPropValue::SlotRef` /
/// `LayoutPropValue::EmitRef` whose name matches a call-site binding.
///
/// This is the function-call inlining step — it transports the
/// consumer's slot bindings into the package's source so the resolved
/// sub-tree references the consumer's slots, not the package's slots.
///
/// `LayoutPropValue::Expr` deliberately is not rewritten today: the
/// expression text references bare identifiers (e.g. `editRow`,
/// `selectedRow`) that are also the camelCased slot names.  When the
/// consumer's bindings preserve those names — as the canonical
/// `pkg::mosaic-pkg-grid::Grid` call shape in UI28-1 §3 does — the
/// expressions resolve in the call-site's scope automatically.
/// Non-trivial rename handling can be added in a follow-up if a
/// consumer ever needs it.
fn rewrite_bindings(node: &mut LayoutNode, bindings: &HashMap<String, LayoutPropValue>) {
    for prop in &mut node.props {
        match &prop.value {
            LayoutPropValue::SlotRef(name) => {
                if let Some(v) = bindings.get(name) {
                    prop.value = v.clone();
                }
            }
            LayoutPropValue::EmitRef(name) => {
                if let Some(v) = bindings.get(name) {
                    prop.value = v.clone();
                }
            }
            // Keyword / Number / String / Expr — no rewrite (today).
            _ => {}
        }
    }
    for child in &mut node.children {
        rewrite_bindings(child, bindings);
    }
}

/// Walk `node`'s sub-tree and rewrite every unqualified tag that
/// matches one of `pkg_exports` into `pkg::<pkg>::<tag>`.
///
/// This is what lets `mosaic-pkg-grid::Grid.mll` reference `Cell` by
/// bare name — after resolution, the `Cell` references become
/// `pkg::mosaic-pkg-grid::Cell` and the next recursion pass resolves
/// them too.  Tags that are not in `pkg_exports` (kernel primitives
/// like `HostTable`, `For`, `Row`, structural metaprimitives like
/// `If`/`Else`, or unrelated user-defined names) are left untouched.
fn qualify_local_refs(node: &mut LayoutNode, pkg: &str, pkg_exports: &[String]) {
    // Already qualified — don't double-qualify.  This is the guard
    // that prevents `pkg::A::pkg::A::X` shapes if the rewriter is
    // accidentally run twice.
    if node.package_ref().is_none() && pkg_exports.iter().any(|e| e == &node.tag) {
        node.tag = format!("pkg::{}::{}", pkg, node.tag);
    }
    for child in &mut node.children {
        qualify_local_refs(child, pkg, pkg_exports);
    }
}

/// Defence against symlink-escape attacks — Finding 1 of the PR-3
/// security review.  After joining the package root with a
/// grammar-constrained component name, the file path is canonicalised
/// and required to start with the canonicalised package root.  This
/// rejects `<pkg>/src/Foo.mll` symlinks that point outside the package
/// directory (e.g., at `~/.aws/credentials`), preventing both data
/// exfiltration via parse-error messages and accidental contamination
/// of the build artefact.
fn verify_inside_package(
    pkg: &str,
    comp: &str,
    attempted: &Path,
    canon_root: &Path,
) -> Result<(), ResolveError> {
    // `canonicalize` returns an `Err` if the file does not exist —
    // that is not a symlink escape, it's a missing-source case that
    // the next `read_to_string` call will surface with the
    // appropriate `ComponentSourceMissing` error.  So we propagate
    // `NotFound` as `Ok(())` here and let the read step handle it.
    let canon = match std::fs::canonicalize(attempted) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(ResolveError::IoError {
                package: pkg.to_string(),
                component: comp.to_string(),
                path: attempted.to_path_buf(),
                detail: format!("canonicalize failed: {e}"),
            });
        }
    };
    if !canon.starts_with(canon_root) {
        return Err(ResolveError::SymlinkEscape {
            package: pkg.to_string(),
            component: comp.to_string(),
            attempted: attempted.to_path_buf(),
            canonical: canon,
        });
    }
    Ok(())
}

/// Walk a tree and check that no `tag` is qualified — used by the
/// `mosaic-compile` driver as a sanity assertion after `resolve()`
/// finishes.  Returns the first qualified tag found, or `None`.
pub fn first_qualified_tag(root: &LayoutNode) -> Option<&str> {
    if root.package_ref().is_some() {
        return Some(&root.tag);
    }
    for c in &root.children {
        if let Some(t) = first_qualified_tag(c) {
            return Some(t);
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

    /// Create a throwaway package under `tmp/pkg-<name>/` and return its
    /// root path.  Each test gets its own scratch directory under
    /// `/tmp/mosaic-resolver-test-<unique>/` so parallel cargo test
    /// processes don't stomp each other.
    fn scratch_package(
        name: &str,
        components: &[(&str, &str, &str)], // (component_name, mil_src, mll_src)
    ) -> PathBuf {
        // A small process-unique counter — std::time and Math::random
        // aren't usable here so we pull the test name from the call
        // site and append the package name + an atomic counter.
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(1);
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("mosaic-resolver-{n}-{name}"));
        let _ = fs::remove_dir_all(&base);
        let pkg_root = base.join(format!("pkg-{name}"));
        let src = pkg_root.join("src");
        fs::create_dir_all(&src).unwrap();

        let exports: Vec<String> = components.iter().map(|(c, _, _)| c.to_string()).collect();
        let manifest = format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
description = "test scratch package"
license = "MIT"

[components]
exports = {exports:?}

[dependencies]

[kernel]
version = "1"
"#
        );
        fs::write(pkg_root.join("mosaic-package.toml"), manifest).unwrap();
        for (comp, mil, mll) in components {
            fs::write(src.join(format!("{comp}.mil")), mil).unwrap();
            fs::write(src.join(format!("{comp}.mll")), mll).unwrap();
        }
        base // return the search root (containing the pkg-<name>/ dir)
    }

    /// Helper — analyze a consumer `.mll` into a `LayoutDef`.  The
    /// consumer doesn't need a `.mil` for these unit tests because we
    /// pass `None` as the interface descriptor.  The slot / emit
    /// references in the consumer's source are then accepted verbatim
    /// (the validator skips the descriptor cross-check).
    fn consumer_layout(src: &str) -> LayoutDef {
        let ast = moslayout_compiler::parse_layout(src).expect("parse");
        moslayout_compiler::analyze(&ast).expect("analyze")
    }

    // ── PackageNotFound diagnostic ────────────────────────────────

    #[test]
    fn package_not_found_returns_diagnostic() {
        let resolver = PackageResolver::new(vec![]);
        let mut layout = consumer_layout(
            "layout Demo { pkg::does-not-exist::Foo { } }",
        );
        let err = resolver.resolve(&mut layout).unwrap_err();
        assert!(
            matches!(err, ResolveError::PackageNotFound { ref package } if package == "does-not-exist"),
            "expected PackageNotFound, got {err:?}"
        );
    }

    // ── ComponentNotExported diagnostic ───────────────────────────

    #[test]
    fn component_not_exported_returns_diagnostic() {
        let search_root = scratch_package(
            "mini",
            &[(
                "Real",
                r#"component Real { }"#,
                r#"layout Real { Box { } }"#,
            )],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout =
            consumer_layout("layout Demo { pkg::mini::Imposter { } }");
        let err = resolver.resolve(&mut layout).unwrap_err();
        assert!(
            matches!(&err, ResolveError::ComponentNotExported { package, component, .. }
                if package == "mini" && component == "Imposter"),
            "expected ComponentNotExported, got {err:?}"
        );
    }

    // ── Happy-path substitution + tag clearing ────────────────────

    #[test]
    fn qualified_node_substitutes_with_resolved_root() {
        let search_root = scratch_package(
            "mini",
            &[(
                "Greet",
                r#"component Greet { }"#,
                r#"layout Greet { Box [ root ] { Text } }"#,
            )],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        // `layout Demo { pkg::mini::Greet { } }` — moslayout requires
        // exactly ONE root node per layout, so the qualified
        // reference IS the root.  After resolution, the root itself
        // becomes the resolved sub-tree.
        let mut layout =
            consumer_layout("layout Demo { pkg::mini::Greet { } }");
        resolver.resolve(&mut layout).expect("resolve");
        assert_eq!(layout.root.tag, "Box");
        assert_eq!(layout.root.part_name.as_deref(), Some("root"));
        assert_eq!(layout.root.children[0].tag, "Text");
        // Sanity: no `pkg::` anywhere in the tree.
        assert!(first_qualified_tag(&layout.root).is_none());
    }

    // ── Sibling-self-reference auto-qualification ─────────────────

    #[test]
    fn unqualified_sibling_in_package_resolves_recursively() {
        // Package exports two components: a parent that references the
        // child by bare name.  The resolver must auto-qualify the
        // bare reference so the child also gets substituted.
        let search_root = scratch_package(
            "sib",
            &[
                (
                    "Parent",
                    r#"component Parent { }"#,
                    // Parent.mll uses unqualified `Child` — typical
                    // package self-reference shape.
                    r#"layout Parent { Box { Child } }"#,
                ),
                (
                    "Child",
                    r#"component Child { }"#,
                    r#"layout Child { Text }"#,
                ),
            ],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout =
            consumer_layout("layout Demo { pkg::sib::Parent { } }");
        resolver.resolve(&mut layout).expect("resolve");
        // Parent inlined → Box { Child auto-resolved → Text }
        assert_eq!(layout.root.tag, "Box");
        // The auto-qualified Child was resolved to Text.
        assert_eq!(layout.root.children[0].tag, "Text");
        assert!(first_qualified_tag(&layout.root).is_none());
    }

    // ── Slot-binding rewrite ──────────────────────────────────────

    #[test]
    fn call_site_slot_binding_rewrites_inlined_slot_refs() {
        // The package's component has a slot reference; the consumer
        // calls it with a different slot name.  After resolution,
        // every internal `slot: rows` becomes `slot: my-data`.
        let search_root = scratch_package(
            "rw",
            &[(
                "Table",
                r#"component Table { slot rows : text ; }"#,
                r#"layout Table { Box { Text ( slot: rows ) } }"#,
            )],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout = consumer_layout(
            "layout Demo { pkg::rw::Table ( rows: slot: my-data ) }",
        );
        resolver.resolve(&mut layout).expect("resolve");
        // The root is now Box (Table.mll's root); its child is Text.
        let text_node = &layout.root.children[0];
        assert_eq!(text_node.tag, "Text");
        let slot_prop = text_node
            .props
            .iter()
            .find_map(|p| match &p.value {
                LayoutPropValue::SlotRef(s) => Some(s.as_str()),
                _ => None,
            })
            .expect("Text must have a slot prop");
        assert_eq!(slot_prop, "my-data");
    }

    // ── Cycle detection ───────────────────────────────────────────

    #[test]
    fn cycle_in_package_graph_reports_diagnostic() {
        // Two components that reference each other — A → B → A.
        let search_root = scratch_package(
            "cyc",
            &[
                (
                    "A",
                    r#"component A { }"#,
                    r#"layout A { Box { B } }"#,
                ),
                (
                    "B",
                    r#"component B { }"#,
                    r#"layout B { Box { A } }"#,
                ),
            ],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout =
            consumer_layout("layout Demo { pkg::cyc::A { } }");
        let err = resolver.resolve(&mut layout).unwrap_err();
        match err {
            ResolveError::CircularPackageReference { cycle } => {
                assert!(
                    cycle.iter().any(|(p, _)| p == "cyc"),
                    "cycle must mention `cyc`: {cycle:?}"
                );
            }
            other => panic!("expected CircularPackageReference, got {other:?}"),
        }
    }

    // ── Idempotency ───────────────────────────────────────────────

    #[test]
    fn resolving_twice_is_a_noop() {
        let search_root = scratch_package(
            "noop",
            &[(
                "C",
                r#"component C { }"#,
                r#"layout C { Box }"#,
            )],
        );
        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout =
            consumer_layout("layout Demo { pkg::noop::C { } }");
        resolver.resolve(&mut layout).expect("first resolve");
        let snapshot_tag = layout.root.tag.clone();
        resolver.resolve(&mut layout).expect("second resolve");
        assert_eq!(layout.root.tag, snapshot_tag);
        assert!(first_qualified_tag(&layout.root).is_none());
    }

    // ── Symlink escape rejected (security review Finding 1) ──────

    #[test]
    #[cfg(unix)]
    fn symlink_pointing_outside_package_is_rejected() {
        use std::os::unix::fs::symlink;
        // Build a real package with a legitimate Foo.mil and Foo.mll —
        // and then redirect Foo.mll via a symlink to a sensitive
        // file outside the package directory.  The resolver must
        // refuse to follow the symlink.
        let search_root = scratch_package(
            "secret",
            &[(
                "Foo",
                r#"component Foo { }"#,
                // The .mll content is irrelevant; we overwrite this
                // file with a symlink below.
                r#"layout Foo { Box }"#,
            )],
        );
        let foo_mll = search_root.join("pkg-secret").join("src").join("Foo.mll");
        // Replace Foo.mll with a symlink pointing OUTSIDE the
        // package — at /etc/passwd or any other file the test
        // process can read.  We use the search_root's parent dir
        // as the escape target so the test works without privilege.
        std::fs::remove_file(&foo_mll).unwrap();
        let outside_target = search_root.join("outside.txt");
        std::fs::write(&outside_target, "secret-data").unwrap();
        symlink(&outside_target, &foo_mll).unwrap();

        let resolver = PackageResolver::new(vec![search_root]);
        let mut layout =
            consumer_layout("layout Demo { pkg::secret::Foo { } }");
        let err = resolver.resolve(&mut layout).unwrap_err();
        assert!(
            matches!(
                &err,
                ResolveError::SymlinkEscape { package, component, .. }
                    if package == "secret" && component == "Foo"
            ),
            "expected SymlinkEscape, got {err:?}"
        );
    }

    // ── No-op when there are no qualified tags ────────────────────

    #[test]
    fn pure_unqualified_layout_is_unchanged() {
        let resolver = PackageResolver::new(vec![]);
        let mut layout =
            consumer_layout("layout Demo { Box { Text } }");
        resolver.resolve(&mut layout).expect("resolve");
        assert_eq!(layout.root.tag, "Box");
        assert_eq!(layout.root.children[0].tag, "Text");
    }
}
