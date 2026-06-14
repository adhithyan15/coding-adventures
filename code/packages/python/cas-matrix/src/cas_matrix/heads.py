"""IR head sentinels for matrix operations."""

from __future__ import annotations

from symbolic_ir import IRSymbol

MATRIX = IRSymbol("Matrix")
DIMENSIONS = IRSymbol("Dimensions")
DOT = IRSymbol("Dot")
TRANSPOSE = IRSymbol("Transpose")
DETERMINANT = IRSymbol("Determinant")
INVERSE = IRSymbol("Inverse")
IDENTITY_MATRIX = IRSymbol("IdentityMatrix")
ZERO_MATRIX = IRSymbol("ZeroMatrix")
TRACE = IRSymbol("Trace")
RANK = IRSymbol("Rank")
ROW_REDUCE = IRSymbol("RowReduce")

# Phase 19 — eigenvalues / eigenvectors / LU / subspaces / norm / charpoly
EIGENVALUES = IRSymbol("Eigenvalues")
EIGENVECTORS = IRSymbol("Eigenvectors")
CHARPOLY = IRSymbol("CharPoly")
LU = IRSymbol("LU")
NULLSPACE = IRSymbol("NullSpace")
COLUMNSPACE = IRSymbol("ColumnSpace")
ROWSPACE = IRSymbol("RowSpace")
NORM = IRSymbol("Norm")

# Re-exported for convenience.
LIST = IRSymbol("List")
