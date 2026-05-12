# Changelog — iir-linker

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
