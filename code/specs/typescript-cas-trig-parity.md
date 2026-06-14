# TypeScript CAS trig parity

## Goal

Port the Rust `cas-trig` package to pure TypeScript so browser-side CAS paths
can simplify and transform trigonometric symbolic IR without native code or
WASM.

## Scope

This slice mirrors the current Rust crate surface:

- `PI`, `E`, and trigonometric head constants from `symbolic-ir`.
- Exact `sin`, `cos`, and `tan` values at recognized rational multiples of
  `Pi`.
- Numeric evaluation for finite numeric inputs.
- `sinEval`, `cosEval`, `tanEval`, `atanEval`, `asinEval`, and `acosEval`.
- `trigSimplify` as a bottom-up expression tree walker.
- `expandTrig` for angle-addition and double-angle identities.
- `powerReduce` for `Sin(x)^2` and `Cos(x)^2`.

TypeScript `symbolic-ir` rejects non-finite float nodes, so pole and
out-of-domain detection returns unevaluated trig nodes directly instead of
temporarily constructing `Infinity` or `NaN` nodes.
