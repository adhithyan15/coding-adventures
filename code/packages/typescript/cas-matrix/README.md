# cas-matrix (TypeScript)

Pure TypeScript matrix operations over symbolic IR expressions. This package
mirrors the current Rust `cas-matrix` crate and uses the same representation:

```text
Matrix(List(cell, cell, ...), List(cell, cell, ...), ...)
```

Every cell is an arbitrary `IRNode`. Arithmetic returns symbolic expressions
such as `Add`, `Sub`, `Mul`, `Div`, and `Neg`; downstream simplification can
fold numeric entries.

## Operations

| Function | Description |
|---|---|
| `matrix(rows)` | Build a rectangular `Matrix` IR node |
| `rowsOf(m)` | Extract cloned matrix rows |
| `dimensions(m)` | Return `List(rows, cols)` |
| `numRows(m)` / `numCols(m)` | Shape helpers |
| `getEntry(m, row, col)` | 1-based element access |
| `identityMatrix(n)` / `zeroMatrix(rows, cols)` | Constructors |
| `transpose(m)` | Matrix transpose |
| `addMatrices(a, b)` / `subMatrices(a, b)` | Elementwise arithmetic |
| `scalarMultiply(s, m)` | Scalar-times-matrix |
| `dot(a, b)` | Matrix product |
| `trace(m)` | Main diagonal sum |
| `determinant(m)` | Symbolic cofactor determinant |
| `inverse(m)` | Symbolic adjugate inverse |
| `rowReduce(m)` | Exact rational reduced row echelon form for integer/rational matrices |
| `rank(m)` | Exact rational matrix rank for integer/rational matrices |
