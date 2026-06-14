# TypeScript CAS solve parity

## Goal

Port the Rust `cas-solve` package to pure TypeScript so browser-side CAS paths
can solve first- and second-degree equations without native code or WASM.

## Scope

This slice mirrors the current Rust crate surface:

- `Frac`, an exact reduced rational type.
- `solveLinear(a, b)` for `a*x + b = 0`.
- `solveQuadratic(a, b, c)` for `a*x^2 + b*x + c = 0`.
- `SolveResult` values for finite solution lists or all-solutions cases.
- `SOLVE`, `NSOLVE`, `ROOTS`, and `%i` constants.

The broader Python package includes future-facing orchestration and higher
degree/system solving; this TypeScript package intentionally tracks the smaller
Rust crate that is already merged.
