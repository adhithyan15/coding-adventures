# `coding-adventures-nib-type-checker`

`coding-adventures-nib-type-checker` is the Lua semantic-analysis stage for
Nib. It takes the generic grammar-driven AST from `coding_adventures.nib_parser`
and returns a typed wrapper that later stages can use for IR lowering.

This convergence-wave implementation covers the Nib subset already used by the
repo's WASM smoke tests:

- functions and parameters
- `let`
- assignment
- `return`
- `for`
- function calls
- integer and hex literals
- additive expressions
