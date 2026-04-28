# cas-matrix (Rust)

Matrix operations over symbolic IR — Rust port of the Python `cas-matrix` package.

Matrices are represented as `Matrix(List(cell...), List(cell...), ...)` in the IR.
All arithmetic produces unevaluated IR; pass through `cas_simplify::simplify` to reduce.

## Operations

| Function | Description |
|----------|-------------|
| `matrix(rows)` | Construct a Matrix IR node |
| `is_matrix(node)` | Check if a node is a Matrix |
| `dimensions(m)` | Returns `List(nrows, ncols)` |
| `num_rows(m)` / `num_cols(m)` | Shape accessors |
| `get_entry(m, row, col)` | 1-based element access |
| `identity_matrix(n)` | n×n identity (integer 0/1 entries) |
| `zero_matrix(rows, cols)` | All-zero matrix |
| `transpose(m)` | Transpose |
| `add_matrices(a, b)` | Elementwise `Add(aᵢⱼ, bᵢⱼ)` |
| `sub_matrices(a, b)` | Elementwise `Sub(aᵢⱼ, bᵢⱼ)` |
| `scalar_multiply(s, m)` | Elementwise `Mul(s, mᵢⱼ)` |
| `trace(m)` | Sum of diagonal (symbolic `Add(...)`) |
| `dot(a, b)` | Matrix product |
| `determinant(m)` | Cofactor expansion (O(n!)) |
| `inverse(m)` | Adjugate/determinant (symbolic) |

## Usage

```rust
use cas_matrix::{matrix, transpose, determinant, inverse};
use symbolic_ir::{int, sym};

let m = matrix(vec![
    vec![int(1), int(2)],
    vec![int(3), int(4)],
]).unwrap();

let t = transpose(&m).unwrap();
// t == Matrix([[1, 3], [2, 4]])

let d = determinant(&m).unwrap();
// d = Sub(Mul(1, 4), Mul(2, 3))  — pass through simplify to get Integer(-2)
```

## Error handling

All fallible operations return `MatrixResult<IRNode>` (= `Result<IRNode, MatrixError>`).

## Stack position

```
symbolic-ir  ←  cas-matrix
```
