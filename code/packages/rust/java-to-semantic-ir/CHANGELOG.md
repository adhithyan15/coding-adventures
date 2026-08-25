# Changelog

All notable changes to the `java-to-semantic-ir` crate will be documented in this file.

## [0.1.0] - 2026-08-25

### Added

- New crate: the first SIR frontend for
  [SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
  nominal/static-dispatch OOP profile. Implements JV02 milestone M0:
  `compile(tree, module_name)` / `compile_source(source, module_name)`,
  `JavaLowerError { message, line, column }`, mirroring every other
  `-to-semantic-ir` frontend's public API shape exactly.
- Lowers one top-level `class` declaring a `public static void
  main(String[] args)` method whose body is a flat sequence of literal
  expression statements — integer, floating-point (including exponent and
  `f`/`F`/`d`/`D` suffix forms, and large-integer-falls-back-to-float),
  boolean, `null`, and string literals — into a synthesized SIR `main`
  `Function`.
- Every other construct (variable references, every operator including
  unary `-`/`+`/`!`, control flow, method calls, additional classes/
  methods/fields) returns a clean, explicit `JavaLowerError` rather than
  being silently mis-lowered.
- 19 tests in `tests/test_lower.rs` (every literal kind, statement
  ordering, empty body, module-name/metadata preservation, and every scope
  boundary's rejection) plus a doctest. Every positive test also asserts
  the lowered `Module` passes `semantic_ir::validate()`.
- **Caught during development, not shipped**: an initial implementation of
  the expression-precedence-chain descent (`descend_to_literal`) checked
  only the Node-filtered child list at each grammar level, missing that a
  real unary `-`/`+`/`!` shows up as an extra *token* sibling alongside the
  nested expression node — the initial version silently dropped a leading
  `-` and lowered `-7;` to `IntLit(7)`. Caught by this crate's own
  `unary_minus_is_unsupported_in_m0` test before this version shipped;
  fixed by checking the raw (unfiltered) children list instead, so any
  node with more than the one expected `Node` child is correctly rejected.

Registered in the workspace `Cargo.toml` `members` list (alongside
`java-lexer`/`java-parser`).
