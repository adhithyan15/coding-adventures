# Changelog — CodingAdventures::LatticeAstToCss (Perl)

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::LatticeAstToCss`.
- `compile($ast)` — compile a Lattice AST to CSS text.
- Two-pass architecture: symbol collection (Pass 1) then expansion
  and emission (Pass 2).
- Variable declaration and `$var` reference expansion via scope chain.
- Nested rule flattening with `&` parent-reference support.
- Mixin definition (`@mixin`) and `@include` expansion with positional
  parameters and default parameter values.
- `@if` / `@else if` / `@else` control flow with compile-time evaluation
  (numeric comparison, equality, boolean values).
- `@for $i from N through M` (inclusive) and `@for $i from N to M`
  (exclusive) loop unrolling (capped at 1000 iterations).
- `@each $var in list` loop iteration.
- `@while` loop (capped at 1000 iterations).
- `@function` definition and call-site evaluation via `@return`.
- CSS built-in function passthrough (rgb, calc, rgba, etc.).
- Lexical scope chain (`_child_env` / `_lookup_var`) for variable
  isolation across nested blocks and mixin/function call sites.
- Test suite with Test2::V0 covering all major features.
