# Changelog — `oct-type-checker`

## 0.1.0 — 2026-05-20 (OCT02 phase 2)

Initial Rust port of the Oct type checker.  Walks the AST produced by
`coding-adventures-oct-parser` and enforces Oct's language-level type
invariants (u8 / bool / void; bool→u8 coerces implicitly; u8→bool is
rejected; `if`/`while` need bool; integer literals must fit u8).

### What's covered (V1)

- Two-pass driver: pass 1 collects `static` and `fn` signatures so
  forward calls work; pass 2 walks each function body.
- Statements: `let`, `assign`, `return`, `if`/`else`, `while`, `loop`,
  `break`, `expr_stmt`.
- Expressions: literals, identifiers, binary chains (`||`, `&&`, `==`,
  `!=`, relational, additive, bitwise), unary `!` / `~`, parens, user
  function calls, intrinsic calls.
- `main` exists with no params and void return type.
- Duplicate `static` / `fn` declarations rejected.

### V1 simplifications vs. the Python checker

- 8008 intrinsic arg validation is best-effort only (we still walk the
  arg expressions so undefined-variable errors surface).  The
  `oct-iir-compiler` rejects every intrinsic call with
  `Unsupported8008Intrinsic`, so deeper type-checking buys nothing.
- No in-place AST annotation (Python sets `node._oct_type`); the Rust
  port returns inferred types via the call stack only — the
  `oct-iir-compiler` re-infers types where it needs them.

### Tests

19 unit tests covering the happy path and every error class:
duplicate decls, missing main, wrong arity, undefined name/function,
u8↔bool asymmetry, if/while condition typing, return mismatches.
