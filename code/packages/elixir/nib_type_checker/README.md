# `coding_adventures_nib_type_checker`

The Elixir `nib_type_checker` package is the semantic-analysis stage for Nib.
It consumes the generic grammar-driven AST from `coding_adventures_nib_parser`
and returns a typed wrapper that later stages can use for IR lowering.

This convergence-wave implementation focuses on the Nib subset already used by
the repo's WASM smoke tests:

- functions and parameters
- `let`
- assignment
- `return`
- `for`
- function calls
- integer and hex literals
- additive expressions
