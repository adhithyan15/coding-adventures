# LANG56 — Multi-File Module Driver

**Status**: In progress  
**Branch**: `feat/lang56-multi-file-module-driver`  
**Depends on**: LANG33 (iir-linker), LANG48 (module_info in AST), LANG52 (stdlib)

---

## Motivation

Every LANG milestone so far compiles a **single source string** to a single
`IIRModule`.  The self-hosted Twig compiler is too large for one file — its
lexer, parser, type-checker, and codegen will live in separate `*.tw` files
that import each other.

The module system is already partly wired:
- **Grammar**: `import_clause` and `export_clause` exist in `twig.grammar`
- **AST**: `ModuleInfo.imports: Vec<String>` / `exports: Vec<String>` in
  `twig-parser`
- **IIR**: `IIRExport` / `IIRImport` types in `interpreter-ir` (LANG33)
- **Linker**: `iir_linker::link(&[IIRModule]) → IIRModule` (LANG33)

**The gap (LANG56)**: the compiler ignores `module_info` entirely (emitting
`exports: vec![]`, `imports: vec![]`), and there is no driver that resolves
import names to file paths, compiles them recursively, and links the result.

LANG56 closes this gap with:
1. A new `twig-module-driver` crate: `compile_module_tree(root_path, search_roots)`
2. `twig-ir-compiler` updates: populate `IIRExport`/`IIRImport` from `module_info`
3. `twig-vm` extension: `run_file(path)` convenience entry point

---

## Module naming convention

Module names in `(import …)` use **slash-separated paths** relative to one of
the search roots.  The file extension `.tw` is implicit:

| Import name | Resolved file |
|------------|--------------|
| `stdlib/io` | `<root>/stdlib/io.tw` |
| `compiler/lexer` | `<root>/compiler/lexer.tw` |
| `utils` | `<root>/utils.tw` |

The module's own name is declared in `(module name …)`.  If absent, the
module name is derived from the resolved file path relative to the search root
(e.g. `compiler/lexer` for `<root>/compiler/lexer.tw`).

---

## What changes

| File/Package | Change |
|-------------|--------|
| **NEW** `twig-module-driver/Cargo.toml` | New crate — depends on `twig-parser`, `twig-ir-compiler`, `iir-linker` |
| **NEW** `twig-module-driver/src/lib.rs` | `compile_module_tree` + `ModuleDriverError` |
| `twig-ir-compiler/src/compiler.rs` | Populate `exports` / `imports` from `module_info` |
| `twig-ir-compiler/Cargo.toml` | Version `0.9.0 → 0.10.0` |
| `twig-vm/Cargo.toml` | Add `twig-module-driver` dep; version `0.13.0 → 0.14.0` |
| `twig-vm/src/lib.rs` | Add `run_file(path)` and `run_files(root, roots)` entry points |
| `code/packages/rust/Cargo.toml` | Add `twig-module-driver` to workspace members |
| Changelogs + README files | New/updated per package |

---

## Design

### `twig-module-driver` — the resolver + compiler + linker

```rust
/// Compile a multi-file Twig program rooted at `root_path`.
///
/// Reads `root_path`, compiles it, walks its `import` declarations
/// recursively (BFS to avoid double-compiling), links all `IIRModule`s
/// with `iir_linker::link`, and returns the single linked module.
///
/// `search_roots` is the ordered list of directories to search for
/// `<module-name>.tw` files.  The directory containing `root_path`
/// is always prepended implicitly.
pub fn compile_module_tree(
    root_path: &Path,
    search_roots: &[&Path],
) -> Result<IIRModule, ModuleDriverError>
```

**Algorithm** (BFS, visits each module name once):

```
queue  ← [root_path]
seen   ← {}
modules ← []

while queue not empty:
    path ← dequeue
    if path in seen: continue
    seen.insert(path)

    source ← read_file(path)
    program ← twig_parser::parse(source)?
    iir_mod ← twig_ir_compiler::compile_source_with_exports(program, module_name)?

    for import_name in iir_mod.imports:
        resolved ← resolve(import_name, search_roots)?
        if resolved not in seen:
            queue.push(resolved)

    modules.push(iir_mod)

linked ← iir_linker::link(&modules)?
return linked
```

The **root module** is the entry point: its `entry_point` function (`main`)
becomes the entry point of the linked module.  All other modules have
`entry_point = None` so the linker treats them as libraries.

### `twig-ir-compiler` changes — populate exports/imports

When `program.module_info` is `Some(info)`:

**Exports**: every name in `info.exports` that matches a compiled function is
added as an `IIRExport`:
```rust
for name in &info.exports {
    if self.fn_globals.contains(name) {
        module.exports.push(IIRExport::new(name));
    }
}
```

**Imports**: every name in `info.imports` becomes an `IIRImport` with a
placeholder return type (`"any"`) — the linker resolves the real types:
```rust
for import_name in &info.imports {
    // import_name is the module path (e.g. "compiler/lexer")
    // The linker will resolve which functions come from it.
    // We register a module-level dependency, not per-function imports
    // (the linker uses export tables from the imported modules).
    module.imports.push(IIRImport::new(import_name, "*", "any"));
}
```

Actually, for the first version we take a simpler approach: since the IIR
linker merges all functions from all modules, **we don't need per-function
`IIRImport` entries** — the linker resolves names globally.  The exports
are the important piece for encapsulation.

Revised approach:
- Populate `module.exports` from `info.exports`
- Leave `module.imports` empty for now (LANG56 v1 uses global name merging,
  not strict import checking)
- `module.entry_point = None` for non-root modules (root keeps `"main"`)

This is the simplest correct approach: the linker merges all functions and
the VM runs the root's `main`.

### `ModuleDriverError` variants

```rust
#[non_exhaustive]
pub enum ModuleDriverError {
    /// Could not read a source file
    Io { path: PathBuf, error: std::io::Error },
    /// Parse error in a module
    Parse { path: PathBuf, error: twig_parser::TwigParseError },
    /// Compile error in a module
    Compile { path: PathBuf, error: twig_ir_compiler::TwigCompileError },
    /// Import name could not be resolved to a file in any search root
    UnresolvedImport { import_name: String, searched: Vec<PathBuf> },
    /// Circular import detected (module directly or transitively imports itself)
    CircularImport { cycle: Vec<String> },
    /// Linker error during final link step
    Link(iir_linker::LinkerError),
}
```

### `twig-vm` entry points

Two new convenience functions in `twig-vm/src/lib.rs`:

```rust
/// Compile and run a single `.tw` file.
/// Imports are resolved relative to the file's directory.
pub fn run_file(path: &Path) -> Result<LispyValue, TwigVMError>

/// Compile and run a multi-file program with explicit search roots.
pub fn run_module_tree(
    root: &Path,
    search_roots: &[&Path],
) -> Result<LispyValue, TwigVMError>
```

`TwigVMError` already exists; add a `ModuleDriver(ModuleDriverError)` variant.

---

## Tests (≥ 10)

Tests live in `twig-module-driver/src/lib.rs` using `tempfile` or
in-memory string fixtures written to a `tempdir`.

1. `single_file_no_imports` — root file with no module declaration compiles and
   links correctly (backward compat)
2. `single_file_with_module_decl_no_imports` — `(module my-mod)` with exports,
   no imports — exports populated correctly
3. `two_file_import` — root imports one library module; library exports one
   function used in root
4. `three_file_chain` — root → lib-a → lib-b (transitive import)
5. `shared_dependency` — root imports lib-a and lib-b which both import lib-c;
   lib-c compiled only once (BFS dedup)
6. `unresolved_import_error` — import of nonexistent module → `UnresolvedImport`
7. `circular_import_error` — a.tw imports b.tw imports a.tw → `CircularImport`
8. `export_only_exports_declared_names` — functions not in `(export …)` are not
   in `IIRExport` list
9. `run_file_executes_correctly` — `run_file` on a temp file returns correct value
10. `multi_file_program_end_to_end` — full two-file program runs and returns
    the expected result via `run_module_tree`

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-module-driver` | NEW | `0.1.0` |
| `twig-ir-compiler` | `0.9.0` | `0.10.0` |
| `twig-vm` | `0.13.0` | `0.14.0` |

---

## Intentional deferrals

- **Per-function import checking** — LANG56 v1 uses global-merge linking;
  strict per-function import declarations deferred to LANG57.
- **Circular import with graceful message** — detected via visited-set, error
  emitted, but the cycle members are not fully reconstructed (just the repeated
  module name). Full cycle reconstruction deferred.
- **Search-path configuration from CLI** — LANG56 exposes the API; the CLI
  binary still only uses single-file mode. CLI multi-file mode deferred.
- **Typed mode across modules** — each module is compiled with its own
  `(typed …)` setting. Cross-module type checking deferred to LANG57.
