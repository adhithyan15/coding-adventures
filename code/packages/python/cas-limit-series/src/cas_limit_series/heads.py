"""IR head sentinels for limit / series operations."""

from __future__ import annotations

from symbolic_ir import IRSymbol

LIMIT = IRSymbol("Limit")
TAYLOR = IRSymbol("Taylor")
SERIES = IRSymbol("Series")
BIG_O = IRSymbol("Big_O")
