# TypeScript CAS complex parity

## Goal

Port the Python/Rust `cas-complex` package to pure TypeScript so browser-side
CAS paths can normalize and inspect complex symbolic expressions without native
code or WASM.

## Scope

This slice mirrors the Rust crate surface:

- `IMAGINARY_UNIT`, `RE`, `IM`, `CONJUGATE`, `ABS`, and `ARG` constants.
- `complexNormalize` and `splitComplex` for rectangular normalization.
- `realPart`, `imagPart`, and `conjugate`.
- `modulus` and `argument` for numeric rectangular inputs, with unevaluated
  `Abs(expr)` and `Arg(expr)` fallback nodes for symbolic inputs.
- `complexPow` for integer powers of numeric complex inputs.

The implementation is pure TypeScript and depends only on `symbolic-ir`.
Symbols other than `I` are treated as opaque real atoms, matching the current
Rust implementation.
