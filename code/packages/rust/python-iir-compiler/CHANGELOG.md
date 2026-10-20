# Changelog

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
  host CPython oracle), `release_lowering`, `cst_contract` (C8),
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
