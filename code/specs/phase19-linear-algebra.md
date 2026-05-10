# Phase 19 — Linear Algebra Completion

**Package**: `cas-matrix` → 0.3.0  
**VM**: `symbolic-vm` → 0.39.0  
**Branch**: `claude/phase19-linear-algebra`

---

## Background

`cas-matrix` 0.2.0 shipped the foundational matrix operations: construction,
arithmetic, determinant, inverse, trace, rank, and row-reduction.  Phase 19
adds the five most practically important higher-level operations that
historical MACSYMA exposed through its `eigenvalues`, `eigenvectors`, `lu`,
`nullspace`, `columnspace`, `rowspace`, `charpoly`, and `norm` functions.

---

## 19a — Characteristic polynomial (`charpoly`)

```
charpoly(A, λ)  →  det(λI − A)  as a polynomial expression in λ
```

**Algorithm**: polynomial-valued cofactor expansion.

Each entry of the matrix `(λI − A)` is:
- Diagonal position `(i,i)`: `λ − aᵢᵢ`, represented as `[−aᵢᵢ, 1]`
  (ascending-power coefficient list).
- Off-diagonal `(i,j)`: `−aᵢⱼ`, represented as `[−aᵢⱼ]`.

Determinant is computed by the same cofactor recurrence on these
polynomial coefficient lists. Result: `[c₀, c₁, …, cₙ]` with
`charpoly(λ) = c₀ + c₁λ + … + cₙλⁿ`.

The IR form returned to the user is an Add/Mul/Pow tree built from the
coefficients and the user-supplied `λ` symbol.

Supported for any `n×n` matrix with rational (IRInteger/IRRational) entries.

**MACSYMA example**:
```
charpoly(matrix([1,2],[3,4]), lambda)
→ Equal(lambda^2 - 5*lambda - 2, 0)   [traditional form]
or simply the polynomial expression: lambda^2 - 5*lambda - 2
```

---

## 19b — Eigenvalues (`eigenvalues`)

```
eigenvalues(A)  →  List(List(λ₁,m₁), List(λ₂,m₂), …)
```

**Algorithm**:
1. Compute `char_poly_coeffs(A)` — Fraction coefficient list.
2. Find roots by delegating to `cas_solve`:
   - n=1: `solve_linear`
   - n=2: `solve_quadratic`
   - n=3: `solve_cubic`
   - n=4: `solve_quartic`
   - n>4: return unevaluated
3. Collect equal roots and count multiplicity.
4. Return `List(List(λ₁, m₁), …)`.

Eigenvalues may be rational, irrational (containing `Sqrt`), or complex
(containing `%i`), matching the output of the underlying solvers.

**MACSYMA example**:
```
eigenvalues(matrix([1,2],[2,1]))  →  List(List(-1, 1), List(3, 1))
eigenvalues(matrix([2,0],[0,2]))  →  List(List(2, 2))
```

---

## 19c — Eigenvectors (`eigenvectors`)

```
eigenvectors(A)  →  List(List(λ₁, m₁, List(v₁, v₂, …)), …)
```

For each eigenvalue `λᵢ`:
1. If `λᵢ` is rational: form `B = A − λᵢI` with Fraction entries.
2. RREF(`B`) → identify free variable columns (non-pivot columns).
3. For each free variable: set that free variable to 1, all others to 0,
   back-substitute to get the eigenvector.
4. Return eigenvectors as column-vector `Matrix` IR nodes.

If `λᵢ` is irrational/complex (IR expression, not a plain Fraction): the
eigenvector computation is omitted — return an empty `List()` for that
eigenvalue's vector list.

**MACSYMA example**:
```
eigenvectors(matrix([1,2],[2,1]))
→  List(List(-1, 1, List(Matrix([-1],[1]))),
        List( 3, 1, List(Matrix([ 1],[1]))))
```

---

## 19d — LU decomposition (`lu`)

```
lu(A)  →  List(L, U, P)
```

Doolittle algorithm with **partial pivoting**:
- `P` is a permutation matrix (rows are identity rows reordered).
- `L` is lower-triangular with 1s on the main diagonal.
- `U` is upper-triangular.
- `P·A = L·U`.

All arithmetic uses `Fraction` — exact, no floating point.

Supported for `n×n` matrices with rational entries. Singular matrices
(zero pivot even after partial pivoting) raise `MatrixError`.

**MACSYMA example**:
```
lu(matrix([2,1],[1,3]))  →  List(L, U, P)
```

---

## 19e — Subspace bases (`nullspace`, `columnspace`, `rowspace`)

All three derive from the RREF computed by `row_reduce`.

### `nullspace(A)` — null space of A
1. RREF(`A`) → identify free columns (non-pivot columns).
2. For each free column `j`: build basis vector `v` where:
   - `v[j] = 1`
   - `v[free_k] = 0` for every other free column `k ≠ j`
   - `v[pivot_row_for_col_c] = −RREF[pivot_row, j]` for every pivot column `c`
3. Return `List(v₁, v₂, …)` where each `vᵢ` is a column-vector Matrix.
4. If A has full column rank: return `List()` (trivial null space).

### `columnspace(A)` — column space of A
1. RREF(`A`) → identify pivot column indices.
2. Return corresponding columns of the **original** `A` (not RREF) as
   column-vector Matrices.
3. Return `List(c₁, c₂, …)`.

### `rowspace(A)` — row space of A
1. RREF(`A`) → extract non-zero rows.
2. Return those rows as row-vector Matrices.
3. Return `List(r₁, r₂, …)`.

**MACSYMA examples**:
```
nullspace(matrix([1,2,3],[4,5,6]))   →  List(Matrix([1],[-2],[1]))
columnspace(matrix([1,2],[2,4]))     →  List(Matrix([1],[2]))
rowspace(matrix([1,2,3],[4,5,6]))    →  List(Matrix([1,0,-1]), Matrix([0,1,2]))
```

---

## 19f — Matrix and vector norm (`norm`)

```
norm(v)           →  Euclidean norm √(Σ vᵢ²)  for a column-vector Matrix
norm(A, "frobenius")  →  Frobenius norm √(Σ aᵢⱼ²)  for a matrix
```

Result is an `IRInteger` or `IRRational` if the sum of squares is a perfect
square, otherwise `IRApply(SQRT, (sum_of_squares,))`.

`norm` without a second argument on a non-vector falls through to unevaluated.

**MACSYMA example**:
```
norm(matrix([3],[4]))     →  5          (Euclidean: sqrt(9+16) = 5)
norm(matrix([1,1],[1,1]), "frobenius")  →  2   (sqrt(4) = 2)
```

---

## Implementation plan

### New files in `cas-matrix`

| File | Content |
|------|---------|
| `eigenvalues.py` | `char_poly_coeffs`, `eigenvalues`, `eigenvectors`, `charpoly` |
| `subspaces.py` | `nullspace`, `columnspace`, `rowspace` |
| `lu.py` | `lu_decompose` (returns `(L, U, P)` as IR Matrix triples) |
| `norms.py` | `norm` |

### New IR heads

Added to `heads.py`:
```python
EIGENVALUES  = IRSymbol("Eigenvalues")
EIGENVECTORS = IRSymbol("Eigenvectors")
CHARPOLY     = IRSymbol("CharPoly")
LU           = IRSymbol("LU")
NULLSPACE    = IRSymbol("NullSpace")
COLUMNSPACE  = IRSymbol("ColumnSpace")
ROWSPACE     = IRSymbol("RowSpace")
NORM         = IRSymbol("Norm")
```

### Files changed

| File | Change |
|------|--------|
| `cas-matrix/src/cas_matrix/eigenvalues.py` | NEW |
| `cas-matrix/src/cas_matrix/subspaces.py` | NEW |
| `cas-matrix/src/cas_matrix/lu.py` | NEW |
| `cas-matrix/src/cas_matrix/norms.py` | NEW |
| `cas-matrix/src/cas_matrix/heads.py` | Add 8 new heads |
| `cas-matrix/src/cas_matrix/__init__.py` | Add all new exports |
| `cas-matrix/tests/test_phase19.py` | NEW ≥ 50 tests |
| `cas-matrix/CHANGELOG.md` | 0.3.0 entry |
| `cas-matrix/pyproject.toml` | Bump to 0.3.0; add `cas-solve>=0.6.0` dep |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | 8 new handlers + registration |
| `symbolic-vm/CHANGELOG.md` | 0.39.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.39.0; `cas-matrix>=0.3.0` |

---

## Polynomial arithmetic for `char_poly_coeffs`

A polynomial is represented as `list[Fraction]` where `p[k]` is the
coefficient of `λ^k`.  Arithmetic:

```python
def _poly_add(p, q):
    n = max(len(p), len(q))
    return [p[i] if i >= len(q) else
            q[i] if i >= len(p) else
            p[i] + q[i] for i in range(n)]

def _poly_sub(p, q):
    return _poly_add(p, [-c for c in q])

def _poly_mul(p, q):
    if not p or not q: return [Fraction(0)]
    result = [Fraction(0)] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            result[i + j] += a * b
    return result

def _poly_neg(p):
    return [-c for c in p]
```

Cofactor expansion on the polynomial-valued matrix `(λI − A)`:
```python
def _det_poly(rows):  # rows: list[list[list[Fraction]]]
    n = len(rows)
    if n == 0: return [Fraction(1)]
    if n == 1: return list(rows[0][0])
    if n == 2:
        a, b, c, d = rows[0][0], rows[0][1], rows[1][0], rows[1][1]
        return _poly_sub(_poly_mul(a, d), _poly_mul(b, c))
    result = [Fraction(0)]
    for j, entry in enumerate(rows[0]):
        minor = [[rows[r][c] for c in range(n) if c != j]
                 for r in range(1, n)]
        sub = _det_poly(minor)
        product = _poly_mul(entry, sub)
        result = _poly_add(result, product) if j % 2 == 0 \
                 else _poly_sub(result, product)
    return result
```

Entry construction for `(λI − A)`:
```python
# diagonal: [−aᵢᵢ, 1]  (= λ − aᵢᵢ)
# off-diag: [−aᵢⱼ]
```

---

## Null-space algorithm in detail

Given RREF `R` of an `m×n` matrix `A`:

1. Identify pivot columns: column `c` is a pivot column if `R[pivot_count, c] == 1`
   and all entries above/below that position are 0.
2. Free columns = all non-pivot columns.
3. For each free column `j`:
   - Initialise solution vector `v` of length `n` to all zeros.
   - Set `v[j] = 1`.
   - For each pivot column `c` with pivot in row `r`:
     `v[c] = −R[r, j]`
4. Return `v` as a column-vector Matrix.

---

## LU algorithm

Doolittle factorisation with partial pivoting on an `n×n` Fraction matrix:

```
P, L, U initialised: P = I_n, L = I_n, U = copy of A

for k = 0..n-1:
    # Partial pivoting: find row with max |U[i,k]| for i >= k
    pivot = argmax |U[i,k]| for i in [k, n)
    swap rows k and pivot in U and P; swap rows k and pivot in L's
    completed part (columns 0..k-1)

    # Doolittle step
    for i = k+1..n-1:
        factor = U[i,k] / U[k,k]
        L[i,k] = factor
        U[i,:] -= factor * U[k,:]
```

Returns `(L_IR, U_IR, P_IR)` as Matrix IR nodes.

---

## Test matrix

| Class | Tests | Validates |
|-------|-------|-----------|
| `TestPhase19_CharPoly` | 8 | 1×1, 2×2 (rational/integer), 3×3, 4×4, coefficient values |
| `TestPhase19_Eigenvalues` | 10 | 1×1, 2×2 integer, 2×2 rational, diagonal 3×3, repeated eigenvalue, complex roots |
| `TestPhase19_Eigenvectors` | 8 | 2×2 simple, 3×3 diagonal, repeated eigenvalue (2D eigenspace) |
| `TestPhase19_LU` | 8 | 2×2, 3×3, identity, requires pivoting, L lower/U upper checks |
| `TestPhase19_Subspaces` | 10 | nullspace (1 free var, 2 free vars, trivial), columnspace, rowspace |
| `TestPhase19_Norm` | 6 | vector 3-4-5, vector non-perfect-square, Frobenius perfect, non-perfect |
| `TestPhase19_Fallthrough` | 4 | symbolic entries, non-square for eigenvalues, n>4, singular LU |
| `TestPhase19_Regressions` | 5 | Rank, RowReduce, Determinant, Inverse, Trace still work |

Total: ≥ 59 tests.
