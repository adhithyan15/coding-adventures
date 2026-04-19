# ir-to-wasm-validator

This package validates the subset of Haskell `compiler-ir` currently accepted
by the local Haskell Wasm lowerer. It catches malformed operand shapes before
the backend emits bytes.
