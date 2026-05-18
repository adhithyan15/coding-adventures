# Changelog — twigc

## [0.2.0] — 2026-05-18

### Added (LANG74 — TW05-S twigc --self-check)

Adds the TW05 fixed-point self-check mode, completing the
"Definition of done" requirement from the TW05 spec:

> `twigc --self-check` reaches a stage1/stage2 fixed point

#### CLI (`twigc` binary)

- `twigc --self-check <DIR>` — run `(fixed-point-check "<DIR>")` via
  the self-hosted compiler pipeline.  Exits 0 on pass, 5 on failure.
  `<DIR>` must be the directory containing the eleven Twig compiler
  source files (`span.tw`, `lexer.tw`, `main.tw`, etc.).
- New exit code 5: self-check failed (fixed-point not reached).

#### Library (`twigc` crate)

- `twigc_self_check(compiler_dir, extra_search_paths) -> Result<bool, TwigcError>`
  Writes an ephemeral wrapper `.tw` that imports `compiler/main` and
  calls `(fixed-point-check dir)` with the compiler directory baked
  in as a string constant.  Returns `Ok(true)` when the fixed-point
  check passes.

#### Tests (`twigc_tests`, +1 test = 7 total)

| Test | Verifies |
|------|---------|
| `self_check_compiler_tree_fixed_point` | `twigc_self_check` on the real 11-module compiler → `Ok(true)` |

---

## [0.1.0] — 2026-05-17

### Added (LANG73 — TW05-R twigc CLI driver)

Initial release.  Wraps the Twig multi-file compilation pipeline in a
user-facing CLI binary and a testable library.

#### CLI (`twigc` binary)

- `twigc <file.tw>` — compile and run; prints the integer result of `main()`.
- `twigc --check <file.tw>` — type-check only; exit 0 on success, 1 on
  `TypeErrors` from a `(typed strict)` module.
- `twigc --emit=iir <file.tw>` — compile to IIR and print a human-readable
  function listing to stdout.
- `twigc --search-path=<DIR>` — add DIR to the module search path (repeatable).
- `-h`/`--help` and `-V`/`--version` flags.

#### Library (`twigc` crate)

- `twigc_check(path, search_paths) -> Result<(), TwigcError>`
- `twigc_emit_iir(path, search_paths) -> Result<String, TwigcError>`
- `twigc_run(path, search_paths) -> Result<i64, TwigcError>`
- `TwigcError` enum: `Driver(ModuleDriverError)` and `Vm { message }`.

#### Tests (`twigc_tests`, 6 tests)

| Test | Verifies |
|------|---------|
| `check_clean_strict_program_ok` | Clean strict module → `Ok(())` |
| `check_strict_program_with_type_error_fails` | Bad varref in strict → `Err(TypeErrors)` |
| `check_lenient_bad_varref_passes` | Bad varref in lenient → not TypeErrors |
| `emit_iir_produces_fn_listing` | IIR listing contains `fn main:` |
| `run_arithmetic_returns_value` | `(+ 21 21)` → 42 |
| `run_compiler_tree_main_returns_2` | Full 11-module compiler tree, `(main)` → 2 (span.tw fn count) |
