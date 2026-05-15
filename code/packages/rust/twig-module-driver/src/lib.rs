//! # twig-module-driver — Multi-File Module Resolver (LANG56)
//!
//! This crate implements the file-level driver that turns a root `.tw` source
//! file into a single, fully-linked [`IIRModule`] ready for execution.
//!
//! ## Motivation
//!
//! Every previous LANG milestone compiled one source string into one
//! `IIRModule`.  The self-hosted Twig compiler is too large for a single
//! file; it will be split across `compiler/lexer.tw`, `compiler/parser.tw`,
//! `compiler/codegen.tw`, etc.  This crate bridges the gap:
//!
//! 1. **Resolve** — convert `(import compiler/lexer)` to an absolute path.
//! 2. **Compile** — run `twig-ir-compiler` on each source file.
//! 3. **Link** — merge all `IIRModule`s with `iir_linker::link`.
//!
//! ## Module naming
//!
//! Import names use slash-separated paths relative to a search root.
//! The `.tw` extension is implicit:
//!
//! | Import name | File |
//! |------------|------|
//! | `compiler/lexer` | `<root>/compiler/lexer.tw` |
//! | `stdlib/io` | `<root>/stdlib/io.tw` |
//! | `utils` | `<root>/utils.tw` |
//!
//! ## Entry points
//!
//! ```rust
//! use twig_module_driver::compile_module_tree;
//! use std::path::Path;
//!
//! // Compile a single-file Twig program (no imports, backward-compat).
//! let module = compile_module_tree(
//!     Path::new("main.tw"),
//!     &[],
//! );
//! ```
//!
//! ## Design: BFS, visit-once
//!
//! The driver performs a **breadth-first traversal** starting from the root
//! file.  Each module path is visited at most once — if two modules both
//! import `stdlib/io`, `stdlib/io.tw` is compiled exactly once.  Circular
//! imports are detected by checking whether a path is already in the
//! *in-progress* set before it has been fully compiled.
//!
//! ## Linking strategy (LANG56 v1: global merge)
//!
//! LANG56 v1 uses `iir_linker::link` which merges all functions into one
//! module.  Per-function import checking (a module may only call functions
//! it explicitly imported) is deferred to LANG57.  The `(export …)` clause
//! is honoured — the `IIRExport` list is populated by `twig-ir-compiler`
//! for each module that declares it — but the linker does not enforce that
//! callers only use exported names.

use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};

use interpreter_ir::IIRModule;
use iir_linker::LinkError;
use twig_parser::{Expr, Form, Program};

// ---------------------------------------------------------------------------
// ModuleDriverError
// ---------------------------------------------------------------------------

/// Errors the module driver can surface.
///
/// Each variant carries enough context to pinpoint the problem without
/// re-reading any source files.
#[derive(Debug)]
#[non_exhaustive]
pub enum ModuleDriverError {
    /// Could not read a source file from disk.
    Io {
        /// Absolute or relative path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        error: std::io::Error,
    },

    /// A source file failed to parse.
    Parse {
        /// Path of the file that contained the syntax error.
        path: PathBuf,
        /// Parser diagnostic.
        error: twig_parser::TwigParseError,
    },

    /// A source file compiled without syntax errors but the IR compiler
    /// rejected it.
    Compile {
        /// Path of the file that triggered the compile error.
        path: PathBuf,
        /// Compiler diagnostic.
        error: twig_ir_compiler::TwigCompileError,
    },

    /// An `(import …)` name could not be resolved to a `.tw` file under
    /// any of the provided search roots.
    UnresolvedImport {
        /// The module path as it appeared in the source (e.g. `"stdlib/io"`).
        import_name: String,
        /// All search-root directories that were tried.
        searched: Vec<PathBuf>,
    },

    /// A module directly or transitively imports itself.
    ///
    /// Detected during the BFS when a module in the *pending* (not yet
    /// fully compiled) set is encountered again.
    CircularImport {
        /// The module name that closes the cycle.
        cycle_member: String,
    },

    /// The `iir_linker::link` step failed.
    ///
    /// This normally means two modules define functions with the same name —
    /// a name collision that the Twig type-checker would have caught in
    /// strict mode.
    Link(Vec<LinkError>),

    /// The import graph exceeds [`MAX_MODULES`].
    ///
    /// Prevents denial-of-service via artificially large or procedurally
    /// generated import graphs that would exhaust memory or CPU.
    TooManyModules {
        /// Number of modules discovered before the limit was hit.
        count: usize,
    },
}

impl fmt::Display for ModuleDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleDriverError::Io { path, error } => {
                write!(f, "I/O error reading {:?}: {error}", path)
            }
            ModuleDriverError::Parse { path, error } => {
                write!(f, "parse error in {:?}: {error}", path)
            }
            ModuleDriverError::Compile { path, error } => {
                write!(f, "compile error in {:?}: {error}", path)
            }
            ModuleDriverError::UnresolvedImport { import_name, searched } => {
                write!(
                    f,
                    "unresolved import {import_name:?} — searched: {:?}",
                    searched
                )
            }
            ModuleDriverError::CircularImport { cycle_member } => {
                write!(f, "circular import detected at module {cycle_member:?}")
            }
            ModuleDriverError::Link(errors) => {
                write!(f, "linker error(s): {:?}", errors)
            }
            ModuleDriverError::TooManyModules { count } => {
                write!(
                    f,
                    "import graph too large: {count} modules exceeds the limit of {MAX_MODULES}"
                )
            }
        }
    }
}

impl std::error::Error for ModuleDriverError {}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Maximum number of modules allowed in a single `compile_module_tree` call.
///
/// Guards against DoS via artificially large import graphs (OOM / CPU
/// exhaustion when each discovered module triggers its own parse + compile).
/// 1 000 modules is several orders of magnitude beyond any real Twig project.
pub const MAX_MODULES: usize = 1_000;

/// Convert an import name (e.g. `"compiler/lexer"`) to an absolute path.
///
/// Tries each root in `search_roots` in order.  The root containing
/// `requesting_file` is implicitly prepended.  Returns the first path that
/// exists on disk with a `.tw` extension, **provided that the resolved
/// canonical path remains inside one of the valid search roots**.
///
/// Returns `None` if:
/// - `import_name` contains a traversal component (`..`, `.`, empty string,
///   or a raw OS path separator), preventing path-traversal attacks.
/// - No file matching the name exists under any root.
/// - The resolved file, after following symlinks, escapes every valid root
///   (prevents symlink-based sandbox escapes).
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// // (only works if the file actually exists on disk)
/// // let p = twig_module_driver::resolve_import("stdlib/io", &[], Path::new("main.tw"));
/// ```
pub fn resolve_import(
    import_name: &str,
    search_roots: &[&Path],
    requesting_file: &Path,
) -> Option<PathBuf> {
    // ── Security: validate path components ───────────────────────────────────
    //
    // Reject any component that could escape the search-root sandbox:
    //   - `..`  — parent-directory traversal
    //   - `.`   — current-directory reference (harmless but disallowed for clarity)
    //   - empty — double-slash in import name, meaningless and rejected
    //   - contains OS path separator — e.g. backslash on Windows
    //
    // This prevents `(import ../../etc/passwd)` or similar crafted names
    // from assembling a path that escapes the project tree.
    let raw_components: Vec<&str> = import_name.split('/').collect();
    for component in &raw_components {
        if component.is_empty()
            || *component == ".."
            || *component == "."
            || component.contains(std::path::MAIN_SEPARATOR)
            || component.contains('\\')  // reject Windows-style separator on all platforms
        {
            return None; // silently reject traversal attempts
        }
    }

    // Build the relative path: "compiler/lexer" → compiler/lexer.tw
    // (OS-native separators via push-component-by-component).
    let relative: PathBuf = raw_components.iter().fold(PathBuf::new(), |mut p, c| {
        p.push(c);
        p
    });
    let relative = relative.with_extension("tw");

    // Build the full ordered list of valid roots (requesting_dir first).
    let requesting_dir = requesting_file.parent().unwrap_or(Path::new("."));
    let all_roots: Vec<&Path> = std::iter::once(requesting_dir)
        .chain(search_roots.iter().copied())
        .collect();

    // Canonicalize each valid root once so we can use starts_with for the
    // symlink-escape check below.  Roots that fail canonicalization are skipped.
    let canonical_roots: Vec<PathBuf> = all_roots
        .iter()
        .filter_map(|r| std::fs::canonicalize(r).ok())
        .collect();

    all_roots
        .iter()
        .map(|root| root.join(&relative))
        .find(|candidate| {
            if !candidate.exists() {
                return false;
            }
            // ── Security: symlink-escape check ───────────────────────────────
            //
            // After `canonicalize` follows all symlinks, verify the resolved
            // path still lives inside one of the valid roots.  A symlink
            // inside a root that points outside it would otherwise bypass the
            // component-validation above.
            match std::fs::canonicalize(candidate) {
                Ok(canonical_candidate) => {
                    canonical_roots
                        .iter()
                        .any(|canon_root| canonical_candidate.starts_with(canon_root))
                }
                Err(_) => false,
            }
        })
}

// ---------------------------------------------------------------------------
// compile_module_tree
// ---------------------------------------------------------------------------

/// Compile a multi-file Twig program rooted at `root_path`.
///
/// Reads `root_path`, compiles it, walks its `(import …)` declarations
/// recursively (BFS, each module visited once), and links all
/// `IIRModule`s into a single self-contained module ready for
/// `twig-vm::run`.
///
/// `search_roots` is the ordered list of additional directories to search
/// for `<module-name>.tw` files.  The directory containing `root_path` is
/// always searched first, implicitly.
///
/// # Root vs library modules
///
/// The root module keeps `entry_point = Some("main")`.  Every imported
/// module has its `entry_point` cleared to `None` so the linker treats
/// it as a library.  (The `twig-ir-compiler` always emits `entry_point =
/// Some("main")` regardless; we overwrite it here.)
///
/// # Errors
///
/// Returns `Err` on the first problem encountered — I/O failure, parse
/// error, compile error, unresolved import, circular import, or linker
/// error.
///
/// # Example
///
/// ```rust,no_run
/// use twig_module_driver::compile_module_tree;
/// use std::path::Path;
///
/// let module = compile_module_tree(Path::new("main.tw"), &[]).unwrap();
/// // module is ready to pass to twig_vm::run(&module)
/// ```
pub fn compile_module_tree(
    root_path: &Path,
    search_roots: &[&Path],
) -> Result<IIRModule, ModuleDriverError> {
    let root_canonical = canonicalize_best_effort(root_path);

    // ── Phase 1: Discovery ────────────────────────────────────────────────────
    //
    // BFS over the import graph.  For each file we:
    //  - Read and parse (to follow imports and collect function defs)
    //  - Record it in `discovered` (path, parsed AST, module name)
    //  - Build `adjacency` map (path → [(import_name, resolved_path)])
    //  - Deduplicate via `visited` (each canonical path seen at most once)
    //
    // Cycle detection is done in Phase 2 (DFS coloring).  We intentionally
    // do NOT mix cycle detection into the BFS — the BFS `pending` set would
    // give false positives for shared dependencies (e.g. both lib_a and
    // lib_b importing shared.tw).
    let mut discovered: Vec<(PathBuf, Program, String)> = Vec::new();
    // Maps canonical path → direct resolved imports: [(import_name, resolved)]
    let mut adjacency: std::collections::HashMap<PathBuf, Vec<(String, PathBuf)>> =
        std::collections::HashMap::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();

    queue.push_back(root_canonical.clone());
    visited.insert(root_canonical.clone());

    while let Some(current_path) = queue.pop_front() {
        // Read source.
        let source = std::fs::read_to_string(&current_path).map_err(|e| {
            ModuleDriverError::Io {
                path: current_path.clone(),
                error: e,
            }
        })?;

        let module_name = derive_module_name(&current_path, search_roots);

        // Parse.
        let program = twig_parser::parse(&source).map_err(|e| ModuleDriverError::Parse {
            path: current_path.clone(),
            error: e,
        })?;

        // Collect import names declared in `(module … (import …))`.
        let import_names: Vec<String> = program
            .module_info
            .as_ref()
            .map(|mi| mi.imports.clone())
            .unwrap_or_default();

        // Resolve imports and build the adjacency entry for this module.
        let mut adj_entry: Vec<(String, PathBuf)> = Vec::new();
        for import_name in &import_names {
            let resolved: PathBuf = resolve_import(import_name, search_roots, &current_path)
                .map(|p| canonicalize_best_effort(&p))
                .ok_or_else(|| {
                    let searched: Vec<PathBuf> = std::iter::once(
                        current_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
                    )
                    .chain(search_roots.iter().map(|r| r.to_path_buf()))
                    .collect();
                    ModuleDriverError::UnresolvedImport {
                        import_name: import_name.clone(),
                        searched,
                    }
                })?;

            adj_entry.push((import_name.clone(), resolved.clone()));

            // Enqueue the import if not yet visited (dedup guard).
            if visited.insert(resolved.clone()) {
                // Security: cap the total number of modules to prevent
                // denial-of-service via huge or procedurally generated graphs.
                if visited.len() > MAX_MODULES {
                    return Err(ModuleDriverError::TooManyModules {
                        count: visited.len(),
                    });
                }
                queue.push_back(resolved);
            }
        }

        adjacency.insert(current_path.clone(), adj_entry);
        discovered.push((current_path, program, module_name));
    }

    // ── Phase 2: Cycle detection ──────────────────────────────────────────────
    //
    // DFS coloring on the adjacency graph.  Three colours:
    //   0 = White (unvisited)
    //   1 = Grey  (in the current DFS path — a back-edge to Grey is a cycle)
    //   2 = Black (fully explored, no cycle reachable)
    //
    // If any module imports a Grey ancestor, we report CircularImport.
    let mut colors: std::collections::HashMap<PathBuf, u8> =
        std::collections::HashMap::new();

    // `dfs_stack` entries: (path, adj_index, total_adj_len)
    // — iterative DFS to avoid stack overflow on deep graphs.
    let mut dfs_stack: Vec<(PathBuf, usize)> = Vec::new();
    dfs_stack.push((root_canonical.clone(), 0));
    colors.insert(root_canonical.clone(), 1); // Grey

    'dfs: while let Some((top_path, adj_idx)) = dfs_stack.last_mut() {
        let top_path = top_path.clone();
        let adj = adjacency.get(&top_path).map(|v| v.as_slice()).unwrap_or(&[]);
        let idx = *adj_idx;

        if idx >= adj.len() {
            // All children explored — colour Black.
            colors.insert(top_path.clone(), 2);
            dfs_stack.pop();
            continue 'dfs;
        }

        *adj_idx += 1;
        let (import_name, child_path) = &adj[idx];
        let import_name = import_name.clone();
        let child_path = child_path.clone();

        match colors.get(&child_path).copied().unwrap_or(0) {
            1 => {
                // Back-edge to a Grey node — cycle!
                return Err(ModuleDriverError::CircularImport {
                    cycle_member: import_name,
                });
            }
            2 => {
                // Already fully explored — safe to skip.
            }
            _ => {
                // White — push onto DFS stack.
                colors.insert(child_path.clone(), 1); // Grey
                dfs_stack.push((child_path, 0));
            }
        }
    }

    // ── Phase 3: Collect all function names from all modules ──────────────────
    //
    // LANG56 v1 uses global-merge linking: the linker merges all functions
    // into one module.  For the compiler to accept cross-module calls
    // (e.g. `(double 21)` in main.tw when `double` is defined in lib.tw),
    // we pre-register every function name from every module as an "extern".
    //
    // We collect names from three sources:
    //
    // 1. `(define (fn …) …)` — user-defined lambda functions.
    //
    // 2. `(record Name (f0 : T) …)` — the compiler auto-generates:
    //      • constructor `Name(f0, …)`
    //      • accessors   `<lowercase(Name)>-<fi>` for each field i
    //      • predicate   `<lowercase(Name)>?`
    //
    // 3. `(union Name (V0 (g0 : T) …) …)` — per-variant:
    //      • constructor `V0(g0, …)`
    //      • predicate   `V0?`
    //      • accessors   `<lowercase(V0)>-<gj>` for each field j
    //
    // Without pre-registering these names, calling e.g. `(span-start sp)`
    // from an importing module fails with "unbound name" during compilation.
    let mut all_fn_names: Vec<String> = Vec::new();

    for (_, program, _) in &discovered {
        for form in &program.forms {
            match form {
                // ── (define (fn …) …) ──────────────────────────────────
                Form::Define(def) => {
                    if matches!(def.expr, Expr::Lambda(_)) {
                        all_fn_names.push(def.name.clone());
                    }
                }
                // ── (record Name (f0 : T) …) ───────────────────────────
                Form::RecordDef(rec) => {
                    let prefix = rec.name.to_lowercase();
                    // Constructor: exact record name (CamelCase).
                    all_fn_names.push(rec.name.clone());
                    // Predicate: <lowercase(name)>?
                    all_fn_names.push(format!("{prefix}?"));
                    // Accessors: <lowercase(name)>-<field>
                    for field in &rec.fields {
                        all_fn_names.push(format!("{prefix}-{}", field.name));
                    }
                }
                // ── (union Name (V0 …) …) ──────────────────────────────
                //
                // Naming rules (must mirror `emit_union_def` in twig-ir-compiler):
                //   • Constructor: exact variant name (e.g. `TkInteger`)
                //   • Predicate:   variant name + "?" (exact case, e.g. `TkInteger?`)
                //   • Accessors:   lowercase(variant) + "-" + field  (e.g. `tkinteger-value`)
                //
                // Note: union predicates keep their original case (`TkInteger?`)
                // whereas record predicates are fully lowercased (`span?`).
                Form::UnionDef(union) => {
                    for variant in &union.variants {
                        let vprefix = variant.name.to_lowercase();
                        // Constructor: exact variant name (CamelCase).
                        all_fn_names.push(variant.name.clone());
                        // Predicate: <variant.name>? (original case — mirrors the compiler).
                        all_fn_names.push(format!("{}?", variant.name));
                        // Accessors: <lowercase(variant)>-<field>
                        for field in &variant.fields {
                            all_fn_names.push(format!("{vprefix}-{}", field.name));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let extern_refs: Vec<&str> = all_fn_names.iter().map(|s| s.as_str()).collect();

    // ── Phase 4: Compile each module with all extern names pre-registered ─────
    let mut modules: Vec<IIRModule> = Vec::new();

    for (path, program, module_name) in &discovered {
        // Use `compile_program_with_externs` so that calls to functions defined
        // in other modules emit `call` instructions instead of "unbound name".
        let mut iir_mod =
            twig_ir_compiler::compile_program_with_externs(program, module_name, &extern_refs)
                .map_err(|e| ModuleDriverError::Compile {
                    path: path.clone(),
                    error: e,
                })?;

        // Only the root module keeps `entry_point = Some("main")`.
        // Library modules have their entry_point cleared so the linker knows
        // that only the root provides the program's starting function.
        if path.as_path() != root_canonical.as_path() {
            iir_mod.entry_point = None;
        }

        modules.push(iir_mod);
    }

    // ── Phase 5: Link ─────────────────────────────────────────────────────────
    //
    // `iir_linker::link` merges all functions into one self-contained module.
    // It returns `Err` if two modules define the same public function name
    // (a name collision that would make the final call target ambiguous).
    iir_linker::link(&modules).map_err(ModuleDriverError::Link)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Derive a Twig module name from a file path.
///
/// Strategy (in order):
/// 1. For the nearest matching search root, return the relative path
///    stem (slash-joined, no extension).
/// 2. Otherwise return the file stem.
///
/// Examples:
///   `/project/compiler/lexer.tw` with root `/project` → `"compiler/lexer"`
///   `/project/utils.tw` with root `/project` → `"utils"`
///   `/standalone.tw` with no roots → `"standalone"`
fn derive_module_name(path: &Path, search_roots: &[&Path]) -> String {
    // Try each search root; also try the path's own parent as the implicit root.
    let parent = path.parent().unwrap_or(Path::new("."));
    let effective_roots: Vec<&Path> = std::iter::once(parent)
        .chain(search_roots.iter().copied())
        .collect();

    for root in effective_roots {
        let root_canonical = canonicalize_best_effort(root);
        let path_canonical = canonicalize_best_effort(path);
        if let Ok(rel) = path_canonical.strip_prefix(&root_canonical) {
            // Convert path components to "/"-separated string without extension.
            // We bind `no_ext` first so it lives long enough for the borrow.
            let no_ext = rel.with_extension("");
            let components: Vec<&str> = no_ext
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            if !components.is_empty() {
                return components.join("/");
            }
        }
    }

    // Fallback: just the file stem.
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Canonicalize a path, falling back to the original if canonicalization
/// fails (e.g. the file doesn't exist yet in tests).
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Write a temp file and return its path.  The file is written to a
    /// directory created by `tempdir_for_test` (a simple per-test directory
    /// under the system temp dir).
    fn write_temp(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn make_tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("twig_module_driver_test_{tag}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── resolve_import ───────────────────────────────────────────────────────

    #[test]
    fn resolve_import_finds_file_in_sibling_dir() {
        let dir = make_tempdir("resolve_sibling");
        let sub = dir.join("stdlib");
        fs::create_dir_all(&sub).unwrap();
        let lib_path = sub.join("io.tw");
        fs::write(&lib_path, "(define (noop) nil)").unwrap();

        let requesting = dir.join("main.tw");
        let found = resolve_import("stdlib/io", &[], &requesting).unwrap();
        assert_eq!(found, lib_path);
    }

    #[test]
    fn resolve_import_uses_explicit_search_root() {
        let dir = make_tempdir("resolve_root");
        let root = dir.join("myroot");
        let sub = root.join("utils");
        fs::create_dir_all(&sub).unwrap();
        let lib_path = sub.join("math.tw");
        fs::write(&lib_path, "(define (add x y) (+ x y))").unwrap();

        let requesting = dir.join("elsewhere").join("main.tw");
        let found = resolve_import("utils/math", &[root.as_path()], &requesting).unwrap();
        assert_eq!(found, lib_path);
    }

    #[test]
    fn resolve_import_returns_none_for_missing() {
        let dir = make_tempdir("resolve_missing");
        let requesting = dir.join("main.tw");
        assert!(resolve_import("nonexistent/module", &[], &requesting).is_none());
    }

    // ── compile_module_tree ──────────────────────────────────────────────────

    #[test]
    fn single_file_no_imports() {
        // A plain Twig program with no module declaration compiles correctly.
        let dir = make_tempdir("single_no_imports");
        let root = write_temp(&dir, "main.tw", "(+ 1 2)");
        let m = compile_module_tree(&root, &[]).unwrap();
        assert_eq!(m.entry_point.as_deref(), Some("main"));
    }

    #[test]
    fn single_file_with_module_decl_exports_populates_iirexport() {
        // (module mymod (export sq)) should populate IIRExport for "sq" in
        // the compiled module.  Note: iir_linker::link clears exports on the
        // merged output (the merged module is self-contained by design), so
        // we test via `twig_ir_compiler::compile_program` directly.
        use twig_parser::parse;
        let src = "
            (module mymod (export sq))
            (define (sq x) (* x x))
            (sq 5)
        ";
        let program = parse(src).unwrap();
        let m = twig_ir_compiler::compile_program(&program, "mymod").unwrap();
        assert!(m.exports.iter().any(|e| e.public_name() == "sq"),
            "expected IIRExport for 'sq', got: {:?}", m.exports);
    }

    #[test]
    fn two_file_import_library_function_callable() {
        // root.tw imports lib.tw and calls a function from it.
        let dir = make_tempdir("two_file");

        write_temp(&dir, "lib.tw", "
            (module lib (export double))
            (define (double x) (* x 2))
        ");
        let root = write_temp(&dir, "main.tw", "
            (module main (import lib))
            (double 21)
        ");

        let m = compile_module_tree(&root, &[]).unwrap();
        // Both main and double should be in the linked module.
        assert!(m.get_function("main").is_some(), "main function missing");
        assert!(m.get_function("double").is_some(), "double function missing");
        // Root keeps its entry point.
        assert_eq!(m.entry_point.as_deref(), Some("main"));
    }

    #[test]
    fn three_file_chain_transitive_import() {
        // root → lib-a → lib-b (transitive)
        let dir = make_tempdir("three_file_chain");

        write_temp(&dir, "lib_b.tw", "
            (module lib_b (export triple))
            (define (triple x) (* x 3))
        ");
        write_temp(&dir, "lib_a.tw", "
            (module lib_a (import lib_b) (export six))
            (define (six x) (triple (* x 2)))
        ");
        let root = write_temp(&dir, "main.tw", "
            (module main (import lib_a))
            (six 1)
        ");

        let m = compile_module_tree(&root, &[]).unwrap();
        assert!(m.get_function("triple").is_some());
        assert!(m.get_function("six").is_some());
        assert!(m.get_function("main").is_some());
    }

    #[test]
    fn shared_dependency_compiled_once() {
        // root imports lib_a and lib_b; both import shared.
        // shared.tw must be compiled exactly once (no duplicate-function error).
        let dir = make_tempdir("shared_dep");

        write_temp(&dir, "shared.tw", "
            (module shared (export helper))
            (define (helper x) (+ x 1))
        ");
        write_temp(&dir, "lib_a.tw", "
            (module lib_a (import shared) (export use_a))
            (define (use_a x) (helper x))
        ");
        write_temp(&dir, "lib_b.tw", "
            (module lib_b (import shared) (export use_b))
            (define (use_b x) (helper (* x 2)))
        ");
        let root = write_temp(&dir, "main.tw", "
            (module main (import lib_a) (import lib_b))
            (+ (use_a 1) (use_b 2))
        ");

        // Should succeed — shared compiled once, no duplicate-function error.
        let m = compile_module_tree(&root, &[]).unwrap();
        assert!(m.get_function("helper").is_some());
    }

    #[test]
    fn unresolved_import_returns_error() {
        let dir = make_tempdir("unresolved");
        let root = write_temp(&dir, "main.tw", "
            (module main (import doesnt_exist))
            42
        ");
        let err = compile_module_tree(&root, &[]).unwrap_err();
        assert!(matches!(err, ModuleDriverError::UnresolvedImport { .. }),
            "expected UnresolvedImport, got: {err}");
    }

    #[test]
    fn circular_import_returns_error() {
        // a.tw imports b.tw; b.tw imports a.tw
        let dir = make_tempdir("circular");
        write_temp(&dir, "b.tw", "
            (module b (import a) (export g))
            (define (g x) x)
        ");
        // Write a.tw last so it's the root (it imports b which will try to import a).
        let root = write_temp(&dir, "a.tw", "
            (module a (import b) (export f))
            (define (f x) (g x))
        ");
        let err = compile_module_tree(&root, &[]).unwrap_err();
        assert!(matches!(err, ModuleDriverError::CircularImport { .. }),
            "expected CircularImport, got: {err}");
    }

    #[test]
    fn export_only_lists_declared_names() {
        // Functions not listed in (export …) should NOT appear in IIRExport.
        // Test at the compiler level (pre-link) because the linker clears
        // exports on the merged output.
        use twig_parser::parse;
        let src = "
            (module mod (export public_fn))
            (define (public_fn x) (internal x))
            (define (internal x) (* x 2))
            (public_fn 3)
        ";
        let program = parse(src).unwrap();
        let m = twig_ir_compiler::compile_program(&program, "mod").unwrap();
        // public_fn is exported
        assert!(m.exports.iter().any(|e| e.public_name() == "public_fn"),
            "public_fn should be exported");
        // internal is NOT exported
        assert!(!m.exports.iter().any(|e| e.public_name() == "internal"),
            "internal should not be exported");
    }

    #[test]
    fn empty_module_no_panic() {
        // An empty Twig file should compile without panicking.
        let dir = make_tempdir("empty_module");
        let root = write_temp(&dir, "empty.tw", "");
        // Empty source compiles to a module with just a main function that returns nil.
        let m = compile_module_tree(&root, &[]);
        // We just check it doesn't panic; the result may be Ok or Err depending
        // on how the parser handles empty input.
        let _ = m;
    }

    #[test]
    fn library_module_has_no_entry_point() {
        // When a library is compiled and linked, it should have no entry_point.
        // The linked result preserves the root's entry_point = Some("main").
        let dir = make_tempdir("lib_no_entry");
        write_temp(&dir, "lib.tw", "
            (module lib (export add))
            (define (add x y) (+ x y))
        ");
        let root = write_temp(&dir, "main.tw", "
            (module main (import lib))
            (add 1 2)
        ");
        let m = compile_module_tree(&root, &[]).unwrap();
        assert_eq!(m.entry_point.as_deref(), Some("main"),
            "linked module should have entry_point = main");
    }
}

// ---------------------------------------------------------------------------
// TW05-D integration tests — compiler data model in typed Twig (LANG57)
// ---------------------------------------------------------------------------
//
// These tests exercise the real `.tw` source files under
// `code/twig/compiler/`.  Each test:
//   1. Copies the required `.tw` files from the source tree to a temp dir
//      under `<tempdir>/compiler/`.
//   2. Writes a small test-entry `.tw` file (or reuses `main.tw`) to
//      `<tempdir>/compiler/main.tw`.
//   3. Calls `twig_vm::run_module_tree` with search root `<tempdir>`.
//
// The source files live at a path derived from `CARGO_MANIFEST_DIR` so the
// tests are hermetic regardless of the working directory at test time.

#[cfg(test)]
mod tw05d_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    // ── Helpers ─────────────────────────────────────────────────────────────

    /// Return the path to the `code/twig/compiler/` source directory.
    /// Derived from CARGO_MANIFEST_DIR so it's crate-relative, not CWD-relative.
    fn twig_compiler_src() -> PathBuf {
        // CARGO_MANIFEST_DIR is <repo>/code/packages/rust/twig-module-driver
        // so ../../../twig/compiler reaches code/twig/compiler/
        let manifest = env!("CARGO_MANIFEST_DIR");
        Path::new(manifest)
            .join("../../../twig/compiler")
            .canonicalize()
            .expect("code/twig/compiler/ must exist")
    }

    /// Create a fresh temp directory for one test (tag prevents collisions).
    fn tempdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("twig_tw05d_test_{tag}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Copy `<twig_src>/<name>.tw` → `<tempdir>/compiler/<name>.tw`.
    fn copy_tw(twig_src: &Path, dest_dir: &Path, name: &str) {
        let src = twig_src.join(format!("{name}.tw"));
        let dest = dest_dir.join("compiler").join(format!("{name}.tw"));
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(&src, &dest)
            .unwrap_or_else(|e| panic!("copy {}: {e}", src.display()));
    }

    /// Write an ad-hoc test entry file to `<dest_dir>/compiler/main.tw`.
    fn write_test_main(dest_dir: &Path, imports: &[&str], body: &str) -> PathBuf {
        let import_clause: String = imports
            .iter()
            .map(|i| format!("          (import {i})"))
            .collect::<Vec<_>>()
            .join("\n");
        let src = format!(
            "(module compiler/main\n  (typed lenient)\n  (export main)\n{import_clause})\n\n(define (main) {body})\n"
        );
        let path = dest_dir.join("compiler").join("main.tw");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &src).unwrap();
        path
    }

    // ── Test 1: make-span valid invariant ────────────────────────────────────

    #[test]
    fn span_make_span_valid_invariant() {
        // (make-span 0 3 7) should return a non-nil Span; we extract span-start
        // which should equal 3 to confirm a real record was constructed.
        let src = twig_compiler_src();
        let dir = tempdir("span_valid");
        copy_tw(&src, &dir, "span");

        let root = write_test_main(
            &dir,
            &["compiler/span"],
            "(span-start (make-span 0 3 7))",
        );

        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("span make-span valid should compile and run");
        assert_eq!(v.as_int(), Some(3), "span-start of make-span(0,3,7) should be 3");
    }

    // ── Test 2: make-span bad invariant → nil ────────────────────────────────

    #[test]
    fn span_make_span_bad_invariant_returns_nil() {
        // (make-span 0 7 3) — start > end — should return nil.
        let src = twig_compiler_src();
        let dir = tempdir("span_bad");
        copy_tw(&src, &dir, "span");

        let root = write_test_main(
            &dir,
            &["compiler/span"],
            "(if (make-span 0 7 3) 1 0)",
        );

        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("span make-span bad invariant should compile and run");
        assert_eq!(v.as_int(), Some(0), "make-span(0,7,3) should be falsy (nil); expected 0");
    }

    // ── Test 3: TkInteger? predicate ─────────────────────────────────────────

    #[test]
    fn token_tkinteger_predicate() {
        // (TkInteger? (TkInteger)) should return a truthy value.
        // Tests that the union variant predicate is generated correctly.
        let src = twig_compiler_src();
        let dir = tempdir("token_pred");
        copy_tw(&src, &dir, "span");
        copy_tw(&src, &dir, "token");

        let root = write_test_main(
            &dir,
            &["compiler/span", "compiler/token"],
            "(if (TkInteger? (TkInteger)) 1 0)",
        );

        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("token TkInteger? predicate should compile and run");
        assert_eq!(v.as_int(), Some(1), "TkInteger? (TkInteger) should be truthy");
    }

    // ── Test 4: AST IntLit accessor extracts value ──────────────────────────

    #[test]
    fn ast_intlit_accessor_extracts_value() {
        // (IntLit 99 nil) constructs a union value; (intlit-value ...) extracts
        // field 0 (value).  Uses the generated accessor for a cross-module union.
        //
        // Note: cross-module (match ...) on variant patterns requires variant_tags
        // to be propagated across modules (a LANG58 improvement).  We use the
        // generated accessor (`intlit-value`) instead, which is a plain cross-module
        // function call and works with LANG57's extern_fns pre-registration.
        let src = twig_compiler_src();
        let dir = tempdir("ast_accessor");
        copy_tw(&src, &dir, "span");
        copy_tw(&src, &dir, "ast");

        let root = write_test_main(
            &dir,
            &["compiler/span", "compiler/ast"],
            "(intlit-value (IntLit 99 nil))",
        );

        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("ast IntLit accessor should compile and run");
        assert_eq!(v.as_int(), Some(99), "intlit-value of (IntLit 99 nil) should be 99");
    }

    // ── Test 5: IirBuilder alloc-slot increments reg-count ──────────────────

    #[test]
    fn iir_builder_alloc_slot_increments_reg_count() {
        // new-builder creates a builder with reg-count 0;
        // alloc-slot returns an updated builder with reg-count 1.
        let src = twig_compiler_src();
        let dir = tempdir("iirbuilder_slot");
        copy_tw(&src, &dir, "span");
        copy_tw(&src, &dir, "iir-types");
        copy_tw(&src, &dir, "iir-builder");

        let root = write_test_main(
            &dir,
            &["compiler/span", "compiler/iir-types", "compiler/iir-builder"],
            // `iirbuilder-reg-count` — generated prefix is `iirbuilder` (lowercase of IirBuilder)
            "(let* ((b0 (new-builder 'fn1)) \
                    (p  (alloc-slot b0)) \
                    (b1 (car p))) \
               (iirbuilder-reg-count b1))",
        );

        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("iir-builder alloc-slot should compile and run");
        assert_eq!(v.as_int(), Some(1), "alloc-slot should increment reg-count to 1");
    }

    // ── Test 6: Full module tree smoke test ──────────────────────────────────

    #[test]
    fn full_module_tree_smoke_test() {
        // Compile all 7 .tw files (the actual source files from code/twig/compiler/)
        // and run (main).  Expected result: 1 (one register slot allocated).
        let src = twig_compiler_src();
        let dir = tempdir("full_tree");

        for name in &["span", "token", "diagnostic", "ast", "iir-types", "iir-builder", "main"] {
            copy_tw(&src, &dir, name);
        }

        let root = dir.join("compiler").join("main.tw");
        let v = twig_vm::run_module_tree(&root, &[dir.as_path()])
            .expect("full compiler data-model tree should compile and run");
        assert_eq!(v.as_int(), Some(1),
            "(main) should return 1 — one register slot was allocated by alloc-slot");
    }
}
