# `coding_adventures_nib_ir_compiler`

The Elixir `nib_ir_compiler` package lowers the typed AST returned by
`coding_adventures_nib_type_checker` into the generic register-based IR from
`coding_adventures_compiler_ir`.

This convergence-wave implementation targets the same subset as the Elixir
type checker and Nib WASM compiler:

- literals
- `let`
- assignment
- `return`
- `for`
- function calls
- additive expressions
