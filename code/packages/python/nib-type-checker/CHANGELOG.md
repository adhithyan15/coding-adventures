# Changelog — coding-adventures-nib-type-checker

All notable changes to this package are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project
uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-12

### Added

- `NibType` enum: four types (`U4`, `U8`, `BCD`, `BOOL`) with `size_bytes`
  property and helpers (`parse_type_name`, `types_are_compatible`,
  `is_bcd_op_allowed`, `is_numeric`).
- `Symbol` dataclass: name, type, is_const, is_static, is_fn, fn_params,
  fn_return_type.
- `ScopeChain` class: push/pop/define/lookup/define_global implementing
  lexical (static) scoping with a stack of dictionaries.
- `NibTypeChecker` class implementing `TypeChecker[ASTNode, ASTNode]` from
  `type-checker-protocol`. Performs two-pass type checking:
  - Pass 1: collect const/static/fn signatures into global scope; build
    call graph for recursion detection.
  - Pass 2: walk each function body, checking all statements and expressions.
- Checks enforced:
  1. All names declared before use (variables, functions).
  2. Expression types correct bottom-up (arithmetic, logical, comparison).
  3. Assignment LHS type == RHS type (no implicit widening).
  4. Function call argument types match parameter types; argument count
     matches.
  5. BCD operator restriction: only `+%` and `-` are legal for `bcd`
     operands.
  6. For-loop bounds must be integer literals or `const`-declared names.
  7. No recursion (direct or mutual), detected via DFS cycle check on the
     static call graph.
  8. `if` and `for` conditions must be `bool`.
  9. Return statements must match the declared return type.
- Module-level `check(ast)` convenience function.
- 80+ test cases organised into 12 categories covering valid programs,
  undeclared names, type mismatches, BCD restrictions, for-loop bounds,
  recursion, if-conditions, function call errors, return type errors,
  scope tests, NibType unit tests, and ScopeChain unit tests.

### Design notes

This package enforces *language-level* invariants only. Hardware constraints
(call depth ≤ 2, total RAM ≤ 160 bytes, register count) belong in the
`intel-4004-ir-validator`, which runs after IR generation. This keeps
the type checker target-independent — the same checker works for any
compilation target.

PR 5 of the Nib compiler pipeline (lexer → parser → **type-checker** → IR
compiler → backend validator → code generator).
