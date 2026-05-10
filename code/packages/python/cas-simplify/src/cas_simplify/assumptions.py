"""Per-VM assumption store for sign-aware simplification.

Every :class:`~symbolic_vm.vm.VM` carries one ``AssumptionContext`` that
records facts the user has declared with ``assume(...)`` and removes them
with ``forget(...)``.

Architecture note
-----------------
This module is a *pure library* — it imports only from ``symbolic_ir`` and
has no dependency on ``symbolic_vm``.  The VM injects the context into
every handler via ``vm.assumptions``; the library functions
(:func:`~cas_simplify.radcan.radcan`,
:func:`~cas_simplify.logcontract.logexpand`, etc.) accept it as an optional
keyword argument.

Facts tracked per symbol
------------------------
A "fact" is one of the string constants below.  Multiple facts may coexist
for the same symbol (e.g. ``positive`` and ``integer`` for a positive
integer parameter).

+------------+----------------------------------+-----------------------------------+
| Constant   | Meaning                          | Set by                            |
+============+==================================+===================================+
| positive   | x > 0                            | assume(x > 0) / assume(x, pos)   |
+------------+----------------------------------+-----------------------------------+
| negative   | x < 0                            | assume(x < 0) / assume(x, neg)   |
+------------+----------------------------------+-----------------------------------+
| zero       | x = 0                            | assume(x = 0)                     |
+------------+----------------------------------+-----------------------------------+
| nonzero    | x ≠ 0                            | assume(x ≠ 0)                     |
+------------+----------------------------------+-----------------------------------+
| nonneg     | x ≥ 0                            | assume(x ≥ 0)                     |
+------------+----------------------------------+-----------------------------------+
| nonpos     | x ≤ 0                            | assume(x ≤ 0)                     |
+------------+----------------------------------+-----------------------------------+
| integer    | x ∈ ℤ                            | assume(x, integer)                |
+------------+----------------------------------+-----------------------------------+

Query interface
---------------
All query methods return ``True`` / ``False`` / ``None`` where ``None``
means *unknown* — not enough information to determine the answer.

Example::

    ctx = AssumptionContext()
    x = "x"
    ctx.assume_relation(Greater(x_sym, IRInteger(0)))
    ctx.is_positive("x")   # True
    ctx.sign_of("x")       # 1
    ctx.is_negative("x")   # False
    ctx.is_integer("x")    # False (not recorded)
"""

from __future__ import annotations

from symbolic_ir import (
    EQUAL,
    GREATER,
    GREATER_EQUAL,
    LESS,
    LESS_EQUAL,
    NOT_EQUAL,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

# ---------------------------------------------------------------------------
# Internal fact string constants
# ---------------------------------------------------------------------------

_POS = "positive"
_NEG = "negative"
_ZERO = "zero"
_NNZ = "nonzero"
_NNG = "nonneg"
_NNP = "nonpos"
_INT = "integer"

# Synonyms accepted by assume_property.
_PROPERTY_MAP: dict[str, str] = {
    "positive": _POS,
    "pos": _POS,
    "negative": _NEG,
    "neg": _NEG,
    "zero": _ZERO,
    "nonzero": _NNZ,
    "nonneg": _NNG,
    "nonnegative": _NNG,
    "nonpos": _NNP,
    "nonpositive": _NNP,
    "integer": _INT,
    "integerp": _INT,
}

# Zero sentinel used for relational comparisons.
_ZERO_IR = IRInteger(0)


class AssumptionContext:
    """Mutable store of declared symbol properties.

    One instance lives on each :class:`~symbolic_vm.vm.VM` as
    ``vm.assumptions``.  All mutations (assume/forget) are in-place.

    Thread-safety: none — the VM itself is single-threaded.
    """

    def __init__(self) -> None:
        # Maps symbol name → set of fact strings.
        self._facts: dict[str, set[str]] = {}

    # ------------------------------------------------------------------
    # Mutation API — called by VM handlers
    # ------------------------------------------------------------------

    def assume_relation(self, expr: IRNode) -> None:
        """Parse a relational IR node and record the implied fact.

        Handles::

            Greater(x, 0)      → x is positive
            Less(x, 0)         → x is negative
            GreaterEqual(x, 0) → x is nonneg
            LessEqual(x, 0)    → x is nonpos
            Equal(x, 0)        → x is zero
            NotEqual(x, 0)     → x is nonzero

        Non-relational nodes and comparisons not against 0 are silently
        ignored (the VM handler returns ``done`` regardless).
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)
        if sym_name is None or rhs != _ZERO_IR:
            return
        head = expr.head
        if head == GREATER:
            self._add(sym_name, _POS)
        elif head == LESS:
            self._add(sym_name, _NEG)
        elif head == GREATER_EQUAL:
            self._add(sym_name, _NNG)
        elif head == LESS_EQUAL:
            self._add(sym_name, _NNP)
        elif head == EQUAL:
            self._add(sym_name, _ZERO)
        elif head == NOT_EQUAL:
            self._add(sym_name, _NNZ)

    def assume_property(self, sym: IRNode, prop: IRNode) -> None:
        """Record a property declaration: ``assume(x, positive)``.

        Accepts any synonym listed in ``_PROPERTY_MAP`` (case-insensitive).
        Silently ignores unknown property names.
        """
        sym_name = _sym_name(sym)
        prop_name = _sym_name(prop)
        if sym_name is None or prop_name is None:
            return
        canonical = _PROPERTY_MAP.get(prop_name.lower())
        if canonical is not None:
            self._add(sym_name, canonical)

    def forget_relation(self, expr: IRNode) -> None:
        """Remove the fact implied by a relational expression.

        Uses the same parsing logic as :meth:`assume_relation` — only
        comparisons against 0 are handled.
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)
        if sym_name is None or rhs != _ZERO_IR:
            return
        head = expr.head
        if head == GREATER:
            self._remove(sym_name, _POS)
        elif head == LESS:
            self._remove(sym_name, _NEG)
        elif head == GREATER_EQUAL:
            self._remove(sym_name, _NNG)
        elif head == LESS_EQUAL:
            self._remove(sym_name, _NNP)
        elif head == EQUAL:
            self._remove(sym_name, _ZERO)
        elif head == NOT_EQUAL:
            self._remove(sym_name, _NNZ)

    def forget_all(self) -> None:
        """Remove every recorded assumption."""
        self._facts.clear()

    # ------------------------------------------------------------------
    # Query API — called by radcan, logexpand, is_handler, sign_handler
    # ------------------------------------------------------------------

    def is_positive(self, sym_name: str) -> bool | None:
        """True if known positive, False if known non-positive, None otherwise.

        ``positive`` directly recorded → True.
        ``negative`` or ``zero`` recorded → False (definitively not positive).
        Anything else → None (unknown).
        """
        facts = self._facts.get(sym_name, frozenset())
        if _POS in facts:
            return True
        if _NEG in facts or _ZERO in facts:
            return False
        return None

    def is_negative(self, sym_name: str) -> bool | None:
        """True if known negative, False if known non-negative, None otherwise.

        Returns False for any fact that implies x ≥ 0: ``positive``,
        ``zero``, or ``nonneg``.
        """
        facts = self._facts.get(sym_name, frozenset())
        if _NEG in facts:
            return True
        if _POS in facts or _ZERO in facts or _NNG in facts:
            return False
        return None

    def is_nonneg(self, sym_name: str) -> bool | None:
        """True if known non-negative (positive or zero), None otherwise."""
        facts = self._facts.get(sym_name, frozenset())
        if _NNG in facts or _POS in facts or _ZERO in facts:
            return True
        if _NEG in facts:
            return False
        return None

    def is_integer(self, sym_name: str) -> bool:
        """True if the symbol is known to be an integer."""
        return _INT in self._facts.get(sym_name, frozenset())

    def sign_of(self, sym_name: str) -> int | None:
        """Return +1 / -1 / 0 based on recorded facts, or None if unknown."""
        facts = self._facts.get(sym_name, frozenset())
        if _POS in facts:
            return 1
        if _NEG in facts:
            return -1
        if _ZERO in facts:
            return 0
        return None

    def is_true_relation(self, expr: IRNode) -> bool | None:
        """Evaluate a relational IR node to True / False / None.

        Uses the currently recorded facts.  Only evaluates comparisons of
        a plain symbol against 0.  Returns None for anything more complex.

        Examples::

            # After assume(x > 0):
            is_true_relation(Greater(x, 0))  # True
            is_true_relation(Less(x, 0))     # False
            is_true_relation(Equal(x, 0))    # False
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return None
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)
        if sym_name is None or rhs != _ZERO_IR:
            return None

        facts = self._facts.get(sym_name, frozenset())
        head = expr.head

        if head == GREATER:
            # x > 0 → True iff positive; False iff negative or zero
            return self.is_positive(sym_name)

        if head == LESS:
            # x < 0 → True iff negative; False iff positive or zero
            return self.is_negative(sym_name)

        if head == GREATER_EQUAL:
            # x ≥ 0 → True if positive or zero; False if negative
            if _POS in facts or _ZERO in facts or _NNG in facts:
                return True
            if _NEG in facts:
                return False
            return None

        if head == LESS_EQUAL:
            # x ≤ 0 → True if negative or zero; False if positive
            if _NEG in facts or _ZERO in facts or _NNP in facts:
                return True
            if _POS in facts:
                return False
            return None

        if head == EQUAL:
            # x = 0 → True iff zero; False iff positive, negative, or nonzero
            if _ZERO in facts:
                return True
            if _POS in facts or _NEG in facts or _NNZ in facts:
                return False
            return None

        if head == NOT_EQUAL:
            # x ≠ 0 → True iff nonzero or positive or negative
            if _NNZ in facts or _POS in facts or _NEG in facts:
                return True
            if _ZERO in facts:
                return False
            return None

        return None

    def has_any_facts(self, sym_name: str) -> bool:
        """True if any facts are recorded for this symbol."""
        return bool(self._facts.get(sym_name))

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _add(self, sym_name: str, fact: str) -> None:
        if sym_name not in self._facts:
            self._facts[sym_name] = set()
        self._facts[sym_name].add(fact)

    def _remove(self, sym_name: str, fact: str) -> None:
        if sym_name in self._facts:
            self._facts[sym_name].discard(fact)
            # Clean up empty sets for tidiness.
            if not self._facts[sym_name]:
                del self._facts[sym_name]


# ---------------------------------------------------------------------------
# Module-level helper
# ---------------------------------------------------------------------------


def _sym_name(node: IRNode) -> str | None:
    """Return the name of ``node`` if it is an IRSymbol, else None."""
    if isinstance(node, IRSymbol):
        return node.name
    return None
