# Changelog — coding-adventures-lattice-ast-to-css (Lua)

## [0.1.1] — 2026-08-09

### Fixed

- Closed the Unix and Windows standalone build chain through
  `lattice_parser` and all of its unpublished local prerequisites.
- Rebuilt exact checked-in rocks with dependency fetching disabled instead of
  trusting ambient LuaRocks installations.
- Added a deployed-module smoke after installation rather than relying only on
  source-path Busted coverage.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.lattice_ast_to_css`.
- `M.compile(ast)` — compile a Lattice AST to CSS text.
- Two-pass architecture: symbol collection then expansion/emission.
- Variable declaration and `$var` reference expansion.
- Nested rule flattening with `&` parent-reference support.
- Mixin definition (`@mixin`) and `@include` expansion with parameters
  and default parameter values.
- `@if` / `@else if` / `@else` control flow with compile-time evaluation.
- `@for $i from N through M` (inclusive) and `@for $i from N to M`
  (exclusive) loop unrolling (capped at 1000 iterations).
- `@each $var in list` loop iteration.
- `@while` loop (capped at 1000 iterations).
- `@function` definition and call-site evaluation.
- CSS built-in function passthrough (rgb, calc, etc. are never
  looked up as Lattice functions).
- Lexical scope chain (`child_env` / `lookup_var`) for variable
  isolation across nested blocks and mixin calls.
- Busted test suite covering all major features.
