# `coding_adventures_nib_wasm_compiler`

The Elixir `nib_wasm_compiler` package is the end-to-end Nib-to-WASM
orchestration layer for the local Elixir compiler stack.

It packages:

- `nib_parser`
- `nib_type_checker`
- `nib_ir_compiler`
- `ir_to_wasm_validator`
- `ir_to_wasm_compiler`
- `wasm_validator`
- `wasm_module_encoder`

into one source-to-binary step.
