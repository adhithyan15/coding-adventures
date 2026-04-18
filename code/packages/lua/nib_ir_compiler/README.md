# `coding-adventures-nib-ir-compiler`

`coding-adventures-nib-ir-compiler` lowers the typed AST returned by
`coding_adventures.nib_type_checker` into the generic register-based IR from
`coding_adventures.compiler_ir`.

This convergence-wave implementation supports the same subset as the Lua Nib
type checker and Nib WASM compiler:

- literals
- `let`
- assignment
- `return`
- `for`
- function calls
- additive expressions
