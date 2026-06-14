# TypeScript CAS matrix parity

## Goal

Port the Rust `cas-matrix` package to pure TypeScript so browser-side CAS paths
can build and manipulate symbolic matrices without native code or WASM.

## Scope

This slice mirrors the current Rust crate surface:

- `Matrix` IR construction using `List` row nodes.
- Shape inspection and 1-based element access.
- Identity and zero matrix constructors.
- Transpose, elementwise add/subtract, scalar multiplication, dot product, and
  trace.
- Symbolic determinant and inverse via cofactor expansion.

The implementation intentionally returns unsimplified symbolic arithmetic nodes,
matching Rust behavior. Row reduction, LU, eigenvalue helpers, norms, and
subspace operations remain future parity slices because they are present in the
larger Python package but not yet in the Rust crate.
