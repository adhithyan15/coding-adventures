# Changelog — iir-linker

## Unreleased — #18432 — private means private; renaming never panics

### Fixed

- **A call could reach another module's private function.**  Nothing ever
  raised `LinkError::UndeclaredCall`, so a function reference to a name the
  module neither defined nor imported stayed bare and bound to whichever
  function owned that name after merging.  With two libraries each holding
  a private `helper`, an app's un-imported `call helper` ran the first
  library's.  The new `resolve::check_function_references` (step 2c of
  `link`) checks every operand the merge pass renames — `call` callee,
  `alloc_closure` body, `task_spawn` / `group_spawn` target — against the
  module's own functions and its imports' local names, and reports
  `UndeclaredCall { in_module, in_function, callee }` once per
  `(module, function, name)`.  `call_builtin` and data operands are not
  function references and are never checked.
- **The rename counter could panic on crafted export aliases.**  When a
  private function's prefixed name `<module>::<fn>` was already taken, the
  `$N` search `assert!`ed a bound of *total functions + 1*.  Export aliases
  claim names without adding functions, so exporting one function as `c`,
  `b::c`, `b::c$0` … `b::c$3` next to a module `b` with a private `c`
  panicked `link()` — in release builds too.  The new `first_free_suffix`
  keeps counting until a name is free (always within `taken + 1` tries, by
  pigeonhole) and remembers where each base left off, so many same-named
  functions do not rescan from `$0` (no quadratic blow-up).

### Changed

- `merge::function_name_slot` is now `pub(crate)` and joined by
  `function_name_operand`, so the new check and the rename pass share one
  definition of "function reference".
- Docs: `link`, `LinkError::UndeclaredCall`, the `merge_modules`
  precondition table, the crate docs and the README describe the check and
  the counter.

### Tests

- Integration: a call and a closure naming another module's private
  function, `task_spawn` / `group_spawn` targets, one report per function,
  every legitimate reference kind (own, import, aliased import) still
  linking, and crafted aliases (`b::c$0` … `b::c$9`) linking instead of
  panicking.  All but the last-but-one fail on the previous linker.
- Unit: `check_function_references` (accepted, undeclared, imports are per
  module, builtins and data operands ignored), `first_free_suffix`
  resumption, many same-named functions, `function_name_operand`.

## Unreleased — #18429 — one owner per name

### Fixed

- **Two modules exporting the same name linked silently.**  The export map
  is keyed by `(module_name, public_name)`, so `left::foo` and `right::foo`
  never collided there — but exported functions keep their *bare* name in
  the merged module, so the result held two functions called `foo` and
  every reference (including `right`'s calls to its own `foo`, and Twig's
  bare-name cross-module calls) ran `left`'s.  `build_export_map` now
  records the first module to claim each public name and reports the new
  `LinkError::ConflictingExport { function_name, first_module,
  second_module }` when a different module claims it again.  Aliases count
  by their public name.  One module holding two same-named functions
  (Twig's user `main` beside its synthesized `main`) is unaffected.
- **Two input modules with the same name shared their imports.**  Import
  rewrites are keyed by module name, so each same-named module picked up
  the other's imports, and an import could override the other's own
  function.  `link` now rejects this with the new
  `LinkError::DuplicateModuleName` (one error per repeated name), checked
  first by the new `resolve::check_module_names`.
- **An unresolved entry point bound to another module's private function.**
  If the entry module's `entry_point` named a function it neither defined
  nor imported, the name stayed bare and bound to whatever owned it after
  merging.  The new `resolve::check_entry_point` requires the effective
  entry point (first module with one) to be a function that module defines
  or the local name of one of its imports; otherwise
  `LinkError::UnresolvedEntryPoint`.
- `merge_modules` documents its preconditions as a table and re-asserts the
  one-owner rule with a `debug_assert!` (it silently runs the wrong
  function rather than crashing when violated).
- **Docs:** `function_name_slot` now states the LANG28 contract that the
  `fn` operand of `task_spawn` (`srcs[0]`) and `group_spawn` (`srcs[1]`) is
  a function *name*, never a closure register — the rewrite matches by
  operand text, so a register there would be renamed too.

### Changed

- `verify_imports` / `verify_imports_against` run the same export-map
  check, so they now also report `ConflictingExport` when two provider
  modules export the same public name.  Nothing outside the linker calls
  them today.

### Tests

- Nine new integration tests in `tests/test_linker.rs`: cross-module
  duplicate export (plain and via an alias), distinct aliases of same-named
  functions, duplicate names inside one module, duplicate module names
  (and reported once per name), unresolved entry point, entry point via an
  import's local name, and only the effective entry point is checked.  The
  five rejection tests fail on the previous linker.
- Unit tests for the three new `Display` arms, `ConflictingExport` vs
  `DuplicateExport` in `build_export_map`, `check_module_names`,
  `check_entry_point`, and the debug-build `merge_modules` guard.

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
