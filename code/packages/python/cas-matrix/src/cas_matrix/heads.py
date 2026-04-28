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

# Re-exported for convenience.
LIST = IRSymbol("List")
