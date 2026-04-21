# Changelog

All notable changes to `coding-adventures-oct-type-checker` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-20

### Added

- **`OctTypeChecker`** — Two-pass type checker for the Oct language, implementing
  `GenericTypeChecker[ASTNode]` from `type_checker_protocol`.

  - **Pass 1 (signature collection)**: Walks top-level declarations and records
    all `static` names/types and function signatures (name, param types, return
    type). This enables forward calls — functions can be called before they
    appear in the source file.

  - **Pass 2 (body type-checking)**: Walks each function body with the complete
    global scope available. Type-checks every statement and expression,
    annotates expression nodes with `._oct_type`.

- **`check_oct(ast: ASTNode) -> TypeCheckResult[ASTNode]`** — Module-level
  convenience function; the standard entry point for the pipeline.

- **Two Oct types enforced**: `"u8"` (unsigned 8-bit integer, 0–255) and
  `"bool"` (boolean, stored as 0/1). `bool` implicitly coerces to `u8` but
  not vice versa.

- **Language-level invariants enforced**:
  - All names declared before use (variables, functions)
  - Expression type inference for all operator categories:
    - Arithmetic (`+`, `-`) and bitwise (`&`, `|`, `^`): u8-compatible → u8
    - Comparisons (`==`, `!=`, `<`, `>`, `<=`, `>=`): u8-compatible → bool
    - Logical (`&&`, `||`): bool → bool
    - Unary `!` (logical NOT): bool → bool
    - Unary `~` (bitwise NOT): u8-compatible → u8
  - Assignment compatibility (bool→u8 allowed, u8→bool rejected)
  - Function call arity and argument type checking
  - Intrinsic argument types and return types enforced
  - Port arguments to `in()`/`out()` must be compile-time literals
  - `if`/`while` conditions must be `bool` (not just "truthy")
  - Return statements must match declared return type
  - Integer literals must be in range 0–255
  - `main` function must exist with no params and no return type
  - Duplicate static/function declarations rejected

- **All 10 hardware intrinsics typed**:
  - `in(PORT) → u8` (PORT must be literal)
  - `out(PORT, val) → void` (PORT must be literal; val u8-compatible)
  - `adc(a, b) → u8` (both args u8-compatible)
  - `sbb(a, b) → u8` (both args u8-compatible)
  - `rlc(a) → u8`, `rrc(a) → u8`, `ral(a) → u8`, `rar(a) → u8` (u8-compatible arg)
  - `carry() → bool` (no args)
  - `parity(a) → bool` (u8-compatible arg)

- **In-place AST annotation**: Sets `._oct_type` attribute on every expression
  node, preserving source location info and avoiding a parallel tree.

- **Scope isolation**: Variables declared inside `if`/`while`/`loop` bodies do
  not leak into the enclosing scope (each block checker receives a scope copy).

- **Error cascade prevention**: Propagated `None` types from prior errors are
  silently tolerated to avoid spurious cascaded diagnostics.

- **Comprehensive test suite** (`tests/test_oct_type_checker.py`):
  - 24 test classes covering all invariants
  - Result structure validation
  - Positive and negative tests for every language construct
  - Full program examples from the OCT00 spec
  - AST annotation verification
  - Error cascade behaviour

### Design Notes

- **Hardware constraints excluded**: Max 4 locals per function, max 7 call
  depth, and port range 0–7/0–23 are Intel 8008-specific backend constraints.
  They live in the IR validator, not here. The same `OctTypeChecker` could in
  principle target the Intel 8080 (a strict superset) without modification.

- **Two-pass necessity**: Pass 1 is required for forward references. Without
  it, `fn main() { helper(); }` followed by `fn helper() { }` would fail with
  "undefined function" unless functions were processed in source order.

- **Token type normalization**: `tokenize_oct` promotes keyword tokens to plain
  strings (e.g. `"fn"`, `"carry"`) while non-keyword tokens carry `TokenType`
  enum values. All helpers in this package use `isinstance(t, str) else t.name`
  normalization (the `_tok_type_name` pattern).
