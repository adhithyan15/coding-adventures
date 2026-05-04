"""IR head sentinels for limit / series operations."""

from __future__ import annotations

from symbolic_ir import IRSymbol

LIMIT = IRSymbol("Limit")
TAYLOR = IRSymbol("Taylor")
SERIES = IRSymbol("Series")
BIG_O = IRSymbol("Big_O")

# ---------------------------------------------------------------------------
# Phase 20 — infinity sentinels
# ---------------------------------------------------------------------------
#
# These reuse existing ``IRSymbol`` names recognised by MACSYMA (``inf``,
# ``minf``) and the numeric evaluator in ``limit_advanced``.  They are
# exported here so downstream code can pattern-match against a single
# canonical object instead of constructing ``IRSymbol("inf")`` everywhere.

#: Positive infinity — ``IRSymbol("inf")``.
INF = IRSymbol("inf")
#: Negative infinity — ``IRSymbol("minf")``.
MINF = IRSymbol("minf")
