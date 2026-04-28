"""IR head sentinels for equation solving."""

from __future__ import annotations

from symbolic_ir import IRSymbol

SOLVE = IRSymbol("Solve")
NSOLVE = IRSymbol("NSolve")
ROOTS = IRSymbol("Roots")
