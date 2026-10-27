# Changelog — iir-linker

## Unreleased — #18421 — renames reach every function reference

### Fixed

- **Closures from a second module ran the first module's lambda.**  Every
  Twig module numbers its lambdas from `__lambda_0`, so linking two modules
  that each use a `lambda` produced two `__lambda_0`s.  `merge_modules`
  correctly renamed the second to `"<module>::__lambda_0"`, but it only ever
  rewrote `call` callees — and only for *imported* names — so the second
  module's `alloc_closure(Str("__lambda_0"), …)` still named the first
  module's body.  The result was a silently wrong answer when the two
  lambdas had the same arity, and an "arity mismatch" run error otherwise.
  The same blind spot meant a module's own `call helper` was not redirected
  when its private `helper` was renamed.
- `merge_modules` now builds a per-module rename map (the module's own
  renamed functions plus its imports; imports win on a clash, as before) and
  applies it to every function-name operand listed in the new
  `function_name_slot` table: `call` / `alloc_closure` / `task_spawn`
  `srcs[0]` and `group_spawn` `srcs[1]`.  The operand's spelling
  (`Var` vs `Str`) is preserved; data operands (`const(Str)`, global names,
  `catch` kinds) are never touched.
- The merged `entry_point` is mapped through the entry module's rename map,
  so a renamed entry function is still the one that runs.
- When a module holds two functions with the same name (Twig emits the
  user's `(define (main) …)` beside its synthesized top-level `main`),
  references bind to the first, matching `IIRModule::get_function`.

### Tests

- Eight new `merge` unit tests: `alloc_closure` body follows a rename, a
  private intra-module call follows a rename, string constants and global
  names are left alone, imports still win over a same-named private
  function, the entry point follows a rename, duplicate names bind to the
  first, and the `function_name_slot` / operand-spelling contracts.  The
  first five fail on the previous linker.

## Unreleased — LANG84 phase 4b — `IIRModule::release`

- Every `IIRModule { .. }` struct literal (code, tests and doc examples)
  names the new `release: None` field from interpreter-ir (LANG84 phase 4b). No
  behaviour change: backends do not read the field.

## [0.1.0] — 2026-05-11

Initial release (LANG33).

### Added

- `LinkError` enum — four failure modes: `Unresolved`, `TypeMismatch`,
  `DuplicateExport`, `UndeclaredCall`.  All variants carry structured fields
  for actionable diagnostics; `Display` and `std::error::Error` implemented.

- `build_export_map` — builds `HashMap<(module_name, public_name), ResolvedExport>`
  from a slice of `IIRModule`s, detecting `DuplicateExport` collisions.

- `resolve_imports` — walks every import in every module, matches against the
  export map, and type-checks if the import carries `param_types` / concrete
  `return_type`.

- `verify_imports_against` — pre-flight import check without merging; used by
  REPL tooling.

- `merge_modules` — collapes a set of `IIRModule`s into one:
  - Private function name collisions resolved with `"<module>::"` prefix.
  - `call` instructions rewritten to use the merged (possibly renamed) callee.
  - `entry_point` preserved from the first module that has one.
  - Merged module has empty `exports`/`imports` (self-contained).

- `link(modules)` — free function combining export-map build + import
  resolution + merge.  Returns `Err(Vec<LinkError>)` if any errors found.

- `link_strict(modules)` — fail-fast variant returning `Err(LinkError)` on the
  first error.

- `verify_imports(module, providers)` — pre-flight without merging.

- `IIRLinker` struct — stateful facade matching the LANG20 `CodeGenerator`
  pattern; `Default` implemented.

- 30 unit tests (across `error`, `resolve`, `merge`, `linker` modules).
- 30 integration tests in `tests/test_linker.rs`.
- 3 doc-tests.

### Architecture notes

Two-pass design:

**Pass 1 (resolve.rs):** Build export map and check all imports.  Errors are
accumulated — a single call reports all missing exports, all type mismatches,
and all duplicate exports at once rather than stopping at the first.

**Pass 2 (merge.rs):** Rename colliding private functions, rewrite `call`
instructions, flatten functions into one `IIRModule`.  Only reached if pass 1
produces no errors, so merge code never sees unresolved imports.

The pointer-keyed `merged_name_map` (`*const IIRFunction → String`) is safe
within the lifetime of the merge call because the modules slice is borrowed
for the duration of `merge_modules`.
