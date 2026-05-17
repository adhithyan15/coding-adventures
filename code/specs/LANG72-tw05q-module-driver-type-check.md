# LANG72 — TW05-Q: Module-Driver Type-Check Phase

## Motivation

After LANG71 (TW05-P Part 2), all 11 compiler modules run under
`(typed strict)` and the type checker can propagate exports across
module boundaries via `check_program_with_globals`.  However,
type-checking currently only happens in hand-written tests — the
`compile_module_tree` function in `twig-module-driver` has no
type-checking step.  This means a module with `(typed strict)` that
references an undefined name compiles fine (the IR compiler ignores
type annotations) but should fail.

TW05-Q wires the type checker into `compile_module_tree` so that
type errors in strict-mode modules cause compilation to fail.

## Solution

Add **Phase 3.5: Type Check** to `compile_module_tree`, inserted
between the existing Phase 3 (extern name collection) and Phase 4
(IR compilation).

### Topological ordering

Phase 3.5 must process modules in dependency order (dependencies
before importers).  Kahn's algorithm on the adjacency graph (built in
Phase 1) produces this order:

1. Compute `in_degree[path] = adjacency[path].len()` (number of
   direct imports for each module).
2. Seed the queue with all modules whose `in_degree = 0` (no imports;
   pure libraries).
3. When a module is processed, decrement the `in_degree` of every
   module that imports it; enqueue any that reach `in_degree = 0`.

The result is a `topo_order: Vec<PathBuf>` with dependencies before
importers.

### Per-module type check

For each module in `topo_order`:

1. Collect `extra_globals` by calling
   `twig_type_checker::extract_module_exports` on every
   already-checked direct dependency.
2. Call `twig_type_checker::check_program_with_globals(program,
   None, &extra_globals)`.
3. If the module is `(typed strict)` and `result.ok == false`,
   return `Err(ModuleDriverError::TypeErrors { path, errors })`.
4. Otherwise store `(program.clone(), result.typed_ast.env)` in a
   per-path cache for use by later modules.

Modules in `(typed lenient)` or `(typed off)` mode with type errors
do **not** fail compilation — lenient mode always returns `ok: true`
from `check_program_with_globals`, so the check is a no-op for them.

### New error variant

```rust
/// The source file has `(typed strict)` and the type checker found errors.
TypeErrors {
    /// Path of the module with type errors.
    path: PathBuf,
    /// The type errors found.
    errors: Vec<type_checker_protocol::TypeErrorDiagnostic>,
},
```

## New dependency

`twig-module-driver/Cargo.toml` gains:

```toml
twig-type-checker = { path = "../twig-type-checker" }
```

## Version

`twig-module-driver`: 0.13.0 → 0.14.0

## Tests (`tw05q_tests`, 4 new)

| Test | Verifies |
|------|---------|
| `compiler_tree_type_checks_clean` | Compile all 11 `.tw` modules; type check passes (no TypeErrors error) |
| `strict_module_bad_varref_fails_type_check` | A `(typed strict)` module with `(unknown-fn 42)` → `TypeErrors` error |
| `lenient_module_bad_varref_compiles` | A `(typed lenient)` module with `(unknown-fn 42)` → compiles successfully |
| `type_errors_carry_path` | `TypeErrors` error contains the correct module file path |

## Files changed

| File | Change |
|------|--------|
| `code/specs/LANG72-tw05q-module-driver-type-check.md` | **new** (this file) |
| `code/packages/rust/twig-module-driver/src/lib.rs` | Add `TypeErrors` variant, Phase 3.5 topo-sort + type-check |
| `code/packages/rust/twig-module-driver/Cargo.toml` | Add `twig-type-checker` dep; 0.13.0 → 0.14.0 |
| `code/packages/rust/twig-module-driver/CHANGELOG.md` | Prepend `[0.14.0]` entry |

## Commit sequence

1. `docs(specs)` — `LANG72-tw05q-module-driver-type-check.md`
2. `feat(twig-module-driver)` — Phase 3.5 type-check, `TypeErrors` variant, tests, bump 0.14.0

## Verification

```bash
cargo test -p twig-module-driver -- tw05q    # 4 new tests pass
cargo test -p twig-module-driver             # all existing tests still pass
cargo build --workspace                     # clean build
```
