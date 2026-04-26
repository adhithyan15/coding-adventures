"""IR head sentinels introduced by cas-simplify.

The actual handler installation is left to consumers (the
:mod:`macsyma_runtime.MacsymaBackend`, the upcoming `mathematica-runtime`,
etc.) — this package only provides the operations themselves.
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

SIMPLIFY = IRSymbol("Simplify")
CANONICAL = IRSymbol("Canonical")

# Heads we treat as flat / commutative for canonicalization.
_COMMUTATIVE_FLAT = frozenset({"Add", "Mul"})


def is_commutative_flat(head_name: str) -> bool:
    """Return True if the head is associative AND commutative.

    Used by :func:`cas_simplify.canonical` to decide which heads can
    have their args sorted and flattened.
    """
    return head_name in _COMMUTATIVE_FLAT
