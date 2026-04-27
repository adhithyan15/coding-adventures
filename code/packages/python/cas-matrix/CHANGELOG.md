# Changelog

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
