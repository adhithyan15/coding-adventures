//! # Import resolution — composing an Adj-Lang program across files (M3).
//!
//! `import "<path>"` lets a `dictionary`, a `rulebook`, and a case live in
//! separate checked-in `.adj` files: a rulebook file imports its dictionary, a
//! case file imports the rulebook. This module walks the import graph and
//! splices every imported file's declarations into one [`Program`], which then
//! lowers exactly like a single-file program.
//!
//! ## Why the library never touches the filesystem
//!
//! The import *graph policy* — relative resolution, idempotency, cycle
//! detection, depth / fan-out bounds — is security-sensitive and must be
//! unit-testable without a real disk. So this module owns the policy but does no
//! I/O: it drives an injected [`ImportProvider`]. The `adj-lang-cli` binary
//! supplies a real, sandbox-checked filesystem provider; tests supply an
//! in-memory map. The provider is the *only* trust boundary that resolves a
//! literal import string to a real file, so that is where path-traversal and
//! symlink defenses live (see the CLI's provider).
//!
//! ## The three guarantees, and how each is enforced
//!
//! | guarantee     | mechanism                                                  |
//! |---------------|------------------------------------------------------------|
//! | relative      | `provider.resolve(importer_id, literal)` → canonical id     |
//! | idempotent    | a `visited` set keyed by canonical id — a file merges once  |
//! | acyclic       | a DFS `stack`; re-entering a stacked id is [`ImportError::Cycle`] |
//! | bounded depth | an explicit depth counter vs [`ImportLimits::max_depth`]    |
//! | bounded fan   | a file counter vs [`ImportLimits::max_files`]               |
//!
//! Because depth is checked on every descent, the recursion that walks the graph
//! cannot exceed `max_depth` frames regardless of how adversarial the import
//! graph is — there is no unbounded-recursion path on untrusted input.

use std::collections::HashSet;

use crate::ast::{Program, Statement};

/// Resolves a literal import string (relative to the file doing the importing)
/// to a *canonical, stable id*, and loads source text for a canonical id. The
/// canonical id is opaque to this module — it only needs equality (for the
/// `visited` / cycle sets) and round-tripping through `load`. A filesystem
/// provider would canonicalize to an absolute real path; an in-memory test
/// provider can use the map key directly.
///
/// **This trait is the trust boundary.** `resolve` is where an implementation
/// must reject path traversal / escapes outside its sandbox root — this module
/// treats whatever id `resolve` returns as already-authorized.
pub trait ImportProvider {
    /// Resolve `literal` (the verbatim `import "<literal>"` string) against
    /// `importer` (the canonical id of the importing file) to a canonical id.
    /// Return `Err(reason)` if the target cannot be resolved or is not allowed
    /// (e.g. escapes the sandbox root).
    fn resolve(&self, importer: &str, literal: &str) -> Result<String, String>;

    /// Load the source text for a canonical id produced by [`resolve`].
    fn load(&self, canonical: &str) -> Result<String, String>;
}

/// Bounds on the import graph — defenses against a hostile or accidental
/// fan-out / deep-chain (a "zip-bomb"-shaped import graph). Past either bound,
/// resolution stops with a clean error rather than exhausting memory or stack.
#[derive(Debug, Clone, Copy)]
pub struct ImportLimits {
    /// Maximum import nesting depth (root = depth 0).
    pub max_depth: usize,
    /// Maximum number of *distinct* files merged (including the root).
    pub max_files: usize,
}

impl Default for ImportLimits {
    /// Generous for real rulebooks (a dictionary ← rulebook ← case chain is
    /// depth 2), tight enough to stop a runaway graph early.
    fn default() -> Self {
        ImportLimits {
            max_depth: 32,
            max_files: 256,
        }
    }
}

/// What can go wrong while resolving an import graph.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// An import cycle: `path` is the chain of canonical ids from the first
    /// re-entered file back to itself (e.g. `[a, b, a]`).
    Cycle { path: Vec<String> },
    /// Import nesting exceeded [`ImportLimits::max_depth`].
    DepthExceeded { limit: usize },
    /// More than [`ImportLimits::max_files`] distinct files were reached.
    TooManyFiles { limit: usize },
    /// The provider could not resolve `literal` (relative to `importer`).
    Resolve {
        importer: String,
        literal: String,
        detail: String,
    },
    /// The provider could not load a resolved canonical id.
    Load { canonical: String, detail: String },
    /// A file failed to parse. `detail` is the rendered [`crate::CompileError`].
    Parse { canonical: String, detail: String },
}

/// Resolve the import graph rooted at `root_id` into a single [`Program`] whose
/// statements are every reachable file's non-`import` declarations, in
/// depth-first post-order (an imported file's declarations precede the
/// declarations of the file that imported it, so a dictionary is in scope by the
/// time the rulebook that `use`s it is merged). The returned program contains no
/// `Import` statements and lowers like any single-file program.
pub fn resolve_imports(
    root_id: &str,
    provider: &dyn ImportProvider,
    limits: ImportLimits,
) -> Result<Program, ImportError> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();
    let mut statements: Vec<Statement> = Vec::new();
    visit(
        root_id,
        0,
        provider,
        &limits,
        &mut visited,
        &mut stack,
        &mut statements,
    )?;
    Ok(Program { statements })
}

/// Depth-first visit of one file. Appends the file's resolved declarations to
/// `out`. Order of guards matters: the **cycle** check (is this id on the active
/// DFS stack?) must precede the **idempotency** check (have we fully merged this
/// id already?) — a file in a cycle is both on the stack *and* in `visited`, and
/// only the stack check distinguishes "currently being processed" (a cycle) from
/// "already done" (a harmless repeat import).
#[allow(clippy::too_many_arguments)]
fn visit(
    canonical: &str,
    depth: usize,
    provider: &dyn ImportProvider,
    limits: &ImportLimits,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
    out: &mut Vec<Statement>,
) -> Result<(), ImportError> {
    if depth > limits.max_depth {
        return Err(ImportError::DepthExceeded {
            limit: limits.max_depth,
        });
    }
    // Cycle BEFORE idempotency (see fn doc).
    if stack.iter().any(|s| s == canonical) {
        let mut path = stack.clone();
        path.push(canonical.to_string());
        return Err(ImportError::Cycle { path });
    }
    // Already fully merged via another path — idempotent no-op.
    if visited.contains(canonical) {
        return Ok(());
    }
    if visited.len() >= limits.max_files {
        return Err(ImportError::TooManyFiles {
            limit: limits.max_files,
        });
    }
    visited.insert(canonical.to_string());

    let src = provider
        .load(canonical)
        .map_err(|detail| ImportError::Load {
            canonical: canonical.to_string(),
            detail,
        })?;
    let program = crate::parse(&src).map_err(|e| ImportError::Parse {
        canonical: canonical.to_string(),
        detail: format!("{e:?}"),
    })?;

    stack.push(canonical.to_string());
    for stmt in program.statements {
        match stmt {
            Statement::Import(literal) => {
                let child = provider.resolve(canonical, &literal).map_err(|detail| {
                    ImportError::Resolve {
                        importer: canonical.to_string(),
                        literal: literal.clone(),
                        detail,
                    }
                })?;
                // Post-order: the imported file's declarations are spliced in
                // *before* the rest of this file's statements.
                visit(&child, depth + 1, provider, limits, visited, stack, out)?;
            }
            other => out.push(other),
        }
    }
    stack.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// An in-memory provider: canonical id == the map key. `resolve` treats the
    /// literal as the canonical id directly (the test graphs use flat ids), so
    /// the graph policy is what's under test, not path arithmetic.
    struct MemProvider {
        files: HashMap<String, String>,
        /// If true, `resolve` joins via a trivial "dir/" prefix so we can test
        /// importer-relative behavior.
        relative: bool,
    }

    impl ImportProvider for MemProvider {
        fn resolve(&self, importer: &str, literal: &str) -> Result<String, String> {
            let id = if self.relative {
                // importer "a/b.adj", literal "c.adj" → "a/c.adj"
                match importer.rfind('/') {
                    Some(i) => format!("{}/{}", &importer[..i], literal),
                    None => literal.to_string(),
                }
            } else {
                literal.to_string()
            };
            if self.files.contains_key(&id) {
                Ok(id)
            } else {
                Err(format!("no such file: {id}"))
            }
        }
        fn load(&self, canonical: &str) -> Result<String, String> {
            self.files
                .get(canonical)
                .cloned()
                .ok_or_else(|| format!("no such file: {canonical}"))
        }
    }

    fn mem(files: &[(&str, &str)], relative: bool) -> MemProvider {
        MemProvider {
            files: files
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            relative,
        }
    }

    #[test]
    fn a_three_file_chain_merges_dictionary_then_rulebook_then_case() {
        let dict =
            "dictionary v { define bacterial : hypothesis  define csf : finding values [low] }\n";
        let rb = "import \"dict.adj\"\n\
                  rulebook meningitis { use v\n contributes 3 from csf(low) to bacterial\n  source \"x\" trust empirical\n }\n";
        let case = "import \"rulebook.adj\"\nobserve csf(low)\n? bacterial\n";
        let provider = mem(
            &[("dict.adj", dict), ("rulebook.adj", rb), ("case.adj", case)],
            false,
        );
        let program = resolve_imports("case.adj", &provider, ImportLimits::default()).unwrap();
        // No Import statements survive.
        assert!(!program
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Import(_))));
        // The merged program lowers + decides.
        let lowered = crate::lower(&program).unwrap();
        assert_eq!(lowered.queries.len(), 1);
        let d = crate::decide(&lowered);
        assert!(d.ranked[0].posterior > 0.0);
    }

    #[test]
    fn a_diamond_import_includes_the_shared_file_once() {
        // root imports a and b; both import shared. `shared`'s prior must appear
        // once (a duplicate prior would be a DuplicatePrior lowering error).
        let shared = "dictionary v { define dx : hypothesis }\nprior 0.20 for dx\n  source \"s\" trust empirical\n";
        let a = "import \"shared.adj\"\n";
        let b = "import \"shared.adj\"\n";
        let root = "import \"a.adj\"\nimport \"b.adj\"\n? dx\n";
        let provider = mem(
            &[
                ("shared.adj", shared),
                ("a.adj", a),
                ("b.adj", b),
                ("root.adj", root),
            ],
            false,
        );
        let program = resolve_imports("root.adj", &provider, ImportLimits::default()).unwrap();
        let priors = program
            .statements
            .iter()
            .filter(|s| matches!(s, Statement::Prior { .. }))
            .count();
        assert_eq!(priors, 1, "shared prior should be merged exactly once");
        crate::lower(&program).unwrap();
    }

    #[test]
    fn a_direct_cycle_is_a_clean_error() {
        let a = "import \"b.adj\"\n";
        let b = "import \"a.adj\"\n";
        let provider = mem(&[("a.adj", a), ("b.adj", b)], false);
        let err = resolve_imports("a.adj", &provider, ImportLimits::default()).unwrap_err();
        match err {
            ImportError::Cycle { path } => {
                assert_eq!(path.first().unwrap(), "a.adj");
                assert_eq!(path.last().unwrap(), "a.adj");
            }
            other => panic!("expected Cycle, got {other:?}"),
        }
    }

    #[test]
    fn a_self_import_is_a_cycle() {
        let a = "import \"a.adj\"\n? dx\n";
        let provider = mem(&[("a.adj", a)], false);
        let err = resolve_imports("a.adj", &provider, ImportLimits::default()).unwrap_err();
        assert!(matches!(err, ImportError::Cycle { .. }), "{err:?}");
    }

    #[test]
    fn depth_is_bounded() {
        // a chain a -> b -> c, with max_depth 1, must trip DepthExceeded.
        let a = "import \"b.adj\"\n";
        let b = "import \"c.adj\"\n";
        let c = "prior 0.1 for dx\n  source \"x\" trust empirical\n";
        let provider = mem(&[("a.adj", a), ("b.adj", b), ("c.adj", c)], false);
        let limits = ImportLimits {
            max_depth: 1,
            max_files: 256,
        };
        let err = resolve_imports("a.adj", &provider, limits).unwrap_err();
        assert!(
            matches!(err, ImportError::DepthExceeded { limit: 1 }),
            "{err:?}"
        );
    }

    #[test]
    fn fan_out_is_bounded() {
        // root imports three leaves; max_files 2 (root + 1) must trip.
        let root = "import \"x.adj\"\nimport \"y.adj\"\nimport \"z.adj\"\n";
        let leaf = "prior 0.1 for dx\n  source \"s\" trust empirical\n";
        let provider = mem(
            &[
                ("root.adj", root),
                ("x.adj", leaf),
                ("y.adj", leaf),
                ("z.adj", leaf),
            ],
            false,
        );
        let limits = ImportLimits {
            max_depth: 32,
            max_files: 2,
        };
        let err = resolve_imports("root.adj", &provider, limits).unwrap_err();
        assert!(
            matches!(err, ImportError::TooManyFiles { limit: 2 }),
            "{err:?}"
        );
    }

    #[test]
    fn an_unresolvable_path_is_reported_not_panicked() {
        let root = "import \"missing.adj\"\n";
        let provider = mem(&[("root.adj", root)], false);
        let err = resolve_imports("root.adj", &provider, ImportLimits::default()).unwrap_err();
        assert!(matches!(err, ImportError::Resolve { .. }), "{err:?}");
    }

    #[test]
    fn imports_resolve_relative_to_the_importing_file() {
        // root at "pkg/root.adj" imports "lib.adj" → "pkg/lib.adj".
        let root = "import \"lib.adj\"\n? dx\n";
        let lib = "dictionary v { define dx : hypothesis }\nprior 0.3 for dx\n  source \"s\" trust empirical\n";
        let provider = mem(&[("pkg/root.adj", root), ("pkg/lib.adj", lib)], true);
        let program = resolve_imports("pkg/root.adj", &provider, ImportLimits::default()).unwrap();
        crate::lower(&program).unwrap();
    }
}
