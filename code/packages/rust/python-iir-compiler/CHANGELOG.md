# Changelog

## Unreleased — LANG84 phase 7b, slice 2: the release sweep

### Added
- `tests/release_sweep.rs`: compiles every positive `py-int-print` case
  under every release whose `iir_status` claims it (3.0a1, 3.12.3, 3.13.16,
  3.11.17, 3.16.0a1, inventory order), digests each module's
  `interpreter_ir::serialise` bytes (SHA-256; the `release` field is not on
  the wire), classes each release by its whole outcome vector, and compares
  the result with the committed `code/grammars/python/release_classes.csv`
  (`subset,case,release,class`, 90 rows). A cell is `rejected` only where a
  `.reject@<release>` override says so and the compile error has its
  category (`int_underscores` on 3.0a1); any other refusal fails. Two
  classes: 3.0a1 alone (`e6a72b170747738e`), and 3.12.3, 3.13.16, 3.11.17,
  3.16.0a1 (`5afb074f1e9583ba`, representative 3.12.3). Every release that
  accepts a case lowers it to the same IIR. `PYTHON_RELEASE_SWEEP_WRITE=1`
  rewrites the CSV; that run fails on purpose and refuses
  `CI`/`GITHUB_ACTIONS`. Tests:
  `the_sweep_covers_every_claiming_release_in_inventory_order`,
  `release_classes_csv_matches_the_sweep`,
  `every_accepting_release_lowers_a_case_to_the_same_iir`,
  `the_boundary_releases_are_the_class_representatives`.
- Dev-dependency `coding_adventures_sha256` for the class digests.

### Changed
- The inventory and corpus readers moved from `tests/release_subsets.rs` to
  `tests/common/mod.rs`, shared with the sweep. They now fail on an override
  other than `.reject@<release>` (an `.expect@<x>` would otherwise be
  ignored).
- `the_default_release_claims_py_int_print_on_the_backends` no longer
  requires the default to be the only `backend_status` claim. It requires
  the default to claim B, and the releases claiming B to be exactly the
  releases claiming I. `releases.csv` now claims B for all five releases,
  because `lang-aot`'s matrix runs the corpus on each class representative.

## Unreleased — LANG84 phase 7b, slice 1: release recorded and selectable through lang-aot

### Added
- The compiled module records the release that produced it:
  `IIRModule::release` is the resolved ID from `compile_source_at` and
  `compile_cst` alike (`3.13.16` for `None`). lang-aot's `--lang
  python@<release>` relies on it. Test:
  `the_module_records_the_release_that_produced_it`.

### Changed
- `FrontendError::ReleaseNotAvailable.stage` is self-describing. A row with
  no grammar pair now reads `no grammar pair: grammar_stage none` instead of
  the bare `none`; python-parser's regeneration-bug phrases (`token table
  missing (regenerate python-lexer)`) are kept as they are. Test:
  `release_not_available_reasons_are_self_describing`.
- `claiming_releases_form_one_iir_identity_class` checks each module's
  `release`, then sets that (intentionally differing) field aside before
  comparing the releases' IIR.

## 0.3.0 — 2026-10-20

### Added — LANG84 phase 7a: `py-int-print` on the release path
- `compile_source_at(source, release, module_name, &FrontendBudgets)`
  resolves an exact `code/grammars/python/releases.csv` ID (`None` =
  `DEFAULT_RELEASE`, `3.13.16`). It looks up the release's `ReleaseProfile`
  (`profiles.rs`, profile `py3`), parses with that release's pair through
  `python-parser::parse_python_at`, and walks the CST in place (`int_lower.rs`).
  The walk uses borrowed nodes and no owned AST; the flat `arith`/`term`
  chains and runs of unary signs are folded in loops. `compile_cst` takes a
  caller-supplied tree after measuring it (items, depth, text).
- LANG83 ops only, plus `const`/`ret`:
  - `+ - *` and unary `-` → `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`,
    `i64_neg_ovf`. A result outside `i64` traps `Trap.Overflow` where
    CPython would make a bigger `int` (stated divergence; bignum promotion
    is later).
  - `//` and `%` → `i64_div_floor` and `i64_mod_floor`. A zero divisor
    traps `Trap.DivZero`.
  - `print(x)` → `print_i64`, `print()` → `putchar 10`.
  - `main` returns `i64` 0, not `ret_void`, the void-entry lesson from the
    CLR and BEAM.
- A divisor known to be `-1` is folded: `a // -1` → `i64_neg_ovf a`,
  `a % -1` → `const 0`. So `i64::MIN // -1` and `i64::MIN % -1` never reach
  a backend's divide.
- Refusals, as typed `Unsupported` errors naming the rule and position:
  - `/` (true division, `Slash::True` in the profile);
  - names, another callee, more than one argument;
  - a call used as a value, or a bare expression statement;
  - a literal outside `i64`.
- `FrontendError`: the LANG84 C6 categories.
- `run_source_at` / `run_module`: run on vm-core, keeping the output before
  a trap and its kind.
- Tests: `release_subsets` (stage I, IIR identity, CSV/profile agreement,
  host CPython oracle, folding Windows' `\r\n` stdout to `\n`),
  `release_lowering`, `cst_contract` (C8),
  `hostile_release` (C5, child process), `iir_path` (C2/C4.5).

### Unchanged
- LANG79's float pilot: `compile_source`, `compile_ast`, `run_source` and
  `pyvm` keep their API, behaviour and the legacy `"3.12"` minor-line
  grammar. LANG84 wants `compile_source` to delegate to
  `compile_source_at`. That waits until the float subset moves to release
  pairs (7b/7c, after LANG83 float formatting).

## 0.2.0 — 2026-10-19

### Module-level float bindings

- Lower a single-target `NAME = <float expression>` statement and rebinding.
  The value is evaluated into temporaries first and then bound with one IIR
  `mov` into the mangled register `py:NAME`, so `x = x + 1.0` reads the old
  value and user names cannot collide with `pyN` temporaries or builtins.
- Accept NAME atoms in standalone expressions, binding values and every
  zero-to-three `print` argument position.
- A read with no earlier binding (statically known in this branch-free
  subset) lowers to a raising `py_name_error` builtin call at that point, so
  `run_source` reports `NameError: name 'x' is not defined` with earlier
  print output retained and later statements not executed, matching CPython.
- Reject with specific compile errors: binding `print` or `__debug__`,
  chained, tuple/list/parenthesized/starred, annotated, augmented, attribute,
  subscript and keyword targets, tuple values, binding a `print()` result,
  integer values, `print` used as a value, and unbound reads of Python 3.12
  builtin or dunder names (which CPython resolves rather than raising).
- Direct-AST callers must supply the lexer's canonical EQUALS token and
  canonical NAME tokens with an ASCII, non-keyword identifier spelling;
  forged kinds, overrides, spellings and node substitutions fail.

### Bounded Python print calls and token validation (previously unreleased)

- Validate direct-AST float literal enum kind and the complete versioned ASCII
  spelling before underscore removal/f64 conversion. Reject malformed digit
  groups, integer-only spellings, signs, special names and trailing bytes
  without changing valid exponent overflow or runtime output behavior.

- Validate canonical unary/binary arithmetic operator tokens and grouping
  delimiters in caller-supplied ASTs. Reject forged kinds, effective types,
  spellings and token/node substitutions while preserving source behavior.

- Lower an exact zero-argument `print()` grammar call to a Rust `vm-core`
  builtin that writes one newline and retains it if a later VM instruction
  fails. Other call forms remain bounded by the existing pilot.
- Lower exactly two positional float expressions from the Python grammar tree
  to IIR and print them through a Rust VM builtin with one separating space.
  Preserve source-order evaluation, output limits, and earlier completed
  output if a later argument or statement fails.
- Lower exactly three positional float expressions with two separating spaces
  through a single bounded Rust VM builtin call. Validate both comma token
  kinds for direct AST callers and preserve earlier output on later failure.

## 0.1.0 — 2026-10-07

- Add Python 3.12 float expressions and one-argument `print` lowering to IIR.
- Execute the emitted module on Rust `vm-core` with Python-specific division
  and float display hooks.
- Bound text in caller-supplied ASTs and preserve earlier stdout when a later
  VM instruction fails.
