# Changelog

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
