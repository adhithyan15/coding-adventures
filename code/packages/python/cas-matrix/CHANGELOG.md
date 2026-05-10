# Changelog

## 0.3.0 — 2026-05-04

**Phase 19 — Linear algebra completion: eigenvalues, eigenvectors, char poly,
LU decomposition, null/col/row spaces, and matrix norms.**

New module ``eigenvalues.py``:

- ``char_poly_coeffs(M)`` — returns coefficients of ``det(λI − A)`` as a
  ``list[Fraction]`` in ascending-power order.  Uses polynomial-valued
  cofactor expansion with exact ``Fraction`` arithmetic throughout.
- ``charpoly(M, lam)`` — same, but returns the polynomial as an IR expression
  tree in the symbol ``lam``.
- ``eigenvalues(M)`` — returns ``List(List(λ₁, m₁), List(λ₂, m₂), …)`` where
  each inner ``List`` is the eigenvalue and its algebraic multiplicity.
  Dispatches to ``cas-solve`` for linear (1×1) through quartic (4×4);
  raises ``MatrixError`` for n > 4.  Multiplicity is determined via the
  derivative test on the characteristic polynomial.  Complex conjugate pairs
  are correctly distinguished using Python complex arithmetic.
- ``eigenvectors(M)`` — returns ``List(List(λ, m, List(v₁, …)), …)``.
  Exact null-space basis vectors are provided for rational eigenvalues;
  complex or irrational eigenvalues yield an empty ``List()`` for their
  vector group.

New module ``subspaces.py``:

- ``nullspace(M)`` — ``List(v₁, v₂, …)`` of n×1 column-vector bases for
  the null space of M.  Returns ``List()`` for full column-rank matrices.
- ``columnspace(M)`` — ``List(c₁, c₂, …)`` of m×1 column-vector bases,
  taken from the pivot columns of the original matrix (not the RREF).
- ``rowspace(M)`` — ``List(r₁, r₂, …)`` of 1×n row-vector bases, taken
  from the non-zero rows of the RREF.

New module ``lu.py``:

- ``lu_decompose(M)`` — Doolittle LU with partial pivoting.  Returns
  ``List(L, U, P)`` where ``P·M = L·U``.  L is unit lower-triangular, U
  is upper-triangular, P is a permutation matrix.  Exact ``Fraction``
  arithmetic throughout.  Raises ``MatrixError`` for non-square, singular,
  or symbolic input.

New module ``norms.py``:

- ``norm(M)`` — Euclidean (L²) norm of a column or row vector.  Raises
  ``MatrixError`` for a non-vector matrix (use the ``"frobenius"`` flag).
- ``norm(M, "frobenius")`` — Frobenius norm of any matrix.  Returns an
  exact ``IRInteger`` / ``IRRational`` when the sum of squares is a perfect
  rational square; otherwise returns ``IRApply(SQRT, (sum_of_squares,))``.

New IR head sentinels in ``heads.py``:

- ``EIGENVALUES``, ``EIGENVECTORS``, ``CHARPOLY``
- ``LU``, ``NULLSPACE``, ``COLUMNSPACE``, ``ROWSPACE``
- ``NORM``

All new functions and sentinels are re-exported from the package root.

Dependency added: ``coding-adventures-cas-solve>=0.6.0`` (used by
``eigenvalues()`` to solve the characteristic polynomial).

---

## 0.2.0 — 2026-04-28

**Group E — Row reduction and rank for numeric matrices.**

New module ``rowreduce.py``:

- ``row_reduce(M)`` — Gauss-Jordan elimination over the rationals producing
  the reduced row echelon form (RREF).  Every pivot is normalised to 1;
  all other entries in pivot columns are zeroed.  Returns a new ``Matrix``
  IR node.  Only ``IRInteger`` / ``IRRational`` entries are supported;
  symbolic entries raise ``MatrixError``.
- ``rank(M)`` — rank of a numeric matrix via forward (REF) elimination.
  Returns ``IRInteger(r)``.

New IR head sentinels in ``heads.py``:

- ``RANK = IRSymbol("Rank")``
- ``ROW_REDUCE = IRSymbol("RowReduce")``

Both functions and sentinels are re-exported from the package root
(``from cas_matrix import rank, row_reduce, RANK, ROW_REDUCE``).

---

## 0.1.0 — 2026-04-25

Initial release.

- ``Matrix`` head sentinel; ``matrix(rows)`` constructor.
- ``dimensions(M)`` returns ``[rows, cols]``.
- ``transpose(M)``, ``identity_matrix(n)``, ``zero_matrix(rows, cols)``.
- ``add_matrices(A, B)``, ``sub_matrices(A, B)``,
  ``scalar_multiply(s, M)``.
- ``dot(A, B)`` — matrix product with shape check.
- ``trace(M)``.
- ``determinant(M)`` via cofactor / Laplace expansion (works for any
  entries; produces un-simplified symbolic IR).
- ``inverse(M)`` via the adjugate / determinant.
- Type-checked, ruff- and mypy-clean. Zero capabilities required.
