# Changelog

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
