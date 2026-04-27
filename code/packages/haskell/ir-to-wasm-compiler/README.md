# ir-to-wasm-compiler

Haskell `ir-to-wasm-compiler` lowers the local `compiler-ir` function subset
into `wasm-types` modules. It currently supports labels, integer arithmetic,
calls, returns, and exported function signatures.
