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
        # Phase G1 — store compound relations as ``(lhs, op, rhs)`` triples.
        # ``op`` is one of the six string constants ``">"``, ``"<"``, ``">="``,
        # ``"<="``, ``"="``, ``"!="``.  IR nodes use structural equality
        # (frozen dataclasses), so set membership deduplicates automatically.
        # See :meth:`assume_relation` for the canonicalisation rules that
        # decide what shape gets stored.
        self._general_relations: set[tuple[IRNode, str, IRNode]] = set()

    # ------------------------------------------------------------------
    # Mutation API — called by VM handlers
    # ------------------------------------------------------------------

    def assume_relation(self, expr: IRNode) -> None:
        """Parse a relational IR node and record the implied fact.

        Two paths:

        1.  **Plain-symbol path** (original Phase 21 behaviour) — recognises
            comparisons of a bare ``IRSymbol`` against literal zero and folds
            them into the per-symbol fact table::

                Greater(x, 0)      → x is positive
                Less(x, 0)         → x is negative
                GreaterEqual(x, 0) → x is nonneg
                LessEqual(x, 0)    → x is nonpos
                Equal(x, 0)        → x is zero
                NotEqual(x, 0)     → x is nonzero

        2.  **Compound-relation path** (Track G1) — any relational shape
            that the plain-symbol path doesn't accept (e.g. ``a^2 > b^2``,
            ``f(x) = g(x)``) is stored verbatim in ``_general_relations`` as
            a canonicalised ``(lhs, op, rhs)`` triple.  No semantic inference
            is attempted; only exact structural matches via
            :meth:`is_true_relation` succeed.

        Non-relational nodes are silently ignored (the VM handler returns
        ``done`` regardless).
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return
        head = expr.head
        op = _RELATION_HEAD_TO_OP.get(head)
        if op is None:
            return
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)
        # Plain-symbol-vs-zero path: fold into the per-symbol fact table.
        if sym_name is not None and rhs == _ZERO_IR:
            if op == ">":
                self._add(sym_name, _POS)
            elif op == "<":
                self._add(sym_name, _NEG)
            elif op == ">=":
                self._add(sym_name, _NNG)
            elif op == "<=":
                self._add(sym_name, _NNP)
            elif op == "=":
                self._add(sym_name, _ZERO)
            elif op == "!=":
                self._add(sym_name, _NNZ)
            return
        # Compound-relation path: store the canonicalised triple verbatim.
        self._general_relations.add(_canon_relation(lhs, op, rhs))

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

        Mirrors :meth:`assume_relation` — drops plain-symbol-vs-zero facts
        from the per-symbol table, and removes the canonicalised triple
        from ``_general_relations`` for compound shapes.  Silently no-ops
        for non-relational input or for relations that were never
        recorded.
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return
        head = expr.head
        op = _RELATION_HEAD_TO_OP.get(head)
        if op is None:
            return
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)
        if sym_name is not None and rhs == _ZERO_IR:
            if op == ">":
                self._remove(sym_name, _POS)
            elif op == "<":
                self._remove(sym_name, _NEG)
            elif op == ">=":
                self._remove(sym_name, _NNG)
            elif op == "<=":
                self._remove(sym_name, _NNP)
            elif op == "=":
                self._remove(sym_name, _ZERO)
            elif op == "!=":
                self._remove(sym_name, _NNZ)
            return
        self._general_relations.discard(_canon_relation(lhs, op, rhs))

    def forget_all(self) -> None:
        """Remove every recorded assumption — both plain-symbol facts and
        compound relations."""
        self._facts.clear()
        self._general_relations.clear()

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

        Three paths, tried in order:

        1.  **Plain-symbol-vs-zero** (original Phase 21 behaviour): folds
            against the per-symbol fact table and may return ``True`` or
            ``False`` depending on what the user has asserted (or its
            logical contradiction).
        2.  **Compound-relation lookup** (Track G1): when the plain-symbol
            path doesn't apply, checks ``_general_relations`` for a stored
            triple matching the query.  Honours commutativity of ``=`` and
            ``!=`` and the dual rewrite ``a < b ↔ b > a``,
            ``a ≤ b ↔ b ≥ a``.  Returns ``True`` on hit.
        3.  **Unknown** — returns ``None``.  No negative-knowledge
            inference: an assertion of ``a^2 > b^2`` says nothing about
            ``a^2 < b^2`` until the user explicitly asserts it.

        Examples::

            # After assume(x > 0):
            is_true_relation(Greater(x, 0))  # True
            is_true_relation(Less(x, 0))     # False
            is_true_relation(Equal(x, 0))    # False

            # After assume(a^2 > b^2):
            is_true_relation(Greater(a^2, b^2))  # True
            is_true_relation(Less(b^2, a^2))     # True  (commute)
            is_true_relation(Less(a^2, b^2))     # None  (no negative inference)
        """
        if not isinstance(expr, IRApply) or len(expr.args) != 2:
            return None
        head = expr.head
        op = _RELATION_HEAD_TO_OP.get(head)
        if op is None:
            return None
        lhs, rhs = expr.args
        sym_name = _sym_name(lhs)

        # Plain-symbol-vs-zero path — original Phase 21 behaviour.
        if sym_name is not None and rhs == _ZERO_IR:
            plain = self._is_true_plain(sym_name, head)
            if plain is not None:
                return plain
            # Fall through to compound-relation lookup in case the user
            # asserted the comparison verbatim against a non-symbol shape
            # that happens to canonicalise to the same triple.

        # Compound-relation path — structural lookup with commutativity.
        return self._lookup_general(lhs, op, rhs)

    def _is_true_plain(
        self, sym_name: str, head: IRNode
    ) -> bool | None:
        """Resolve a plain-symbol-vs-zero query against the fact table.

        Extracted so :meth:`is_true_relation` can fall through to the
        compound-relation path when the per-symbol table has nothing to
        say (every branch returning ``None``).  Kept private; the public
        entry point is :meth:`is_true_relation`.
        """
        facts = self._facts.get(sym_name, frozenset())

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

    def facts_for(self, sym_name: str) -> tuple[str, ...]:
        """Return the recorded facts for ``sym_name`` in deterministic order."""
        return tuple(sorted(self._facts.get(sym_name, ())))

    def symbols_with_facts(self) -> tuple[str, ...]:
        """Return every symbol that currently has at least one recorded fact."""
        return tuple(sorted(name for name, facts in self._facts.items() if facts))

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

    def _lookup_general(
        self, lhs: IRNode, op: str, rhs: IRNode
    ) -> bool | None:
        """Structural lookup against ``_general_relations`` with
        commutativity-aware rewriting.

        Returns ``True`` when the query (or an equivalent rewrite) was
        previously asserted, else ``None``.  The rewrites mirror the
        canonical-form rules in :func:`_canon_relation`:

        +-------------------+-----------------------------+
        | Query             | Matches stored fact         |
        +===================+=============================+
        | ``a > b``         | ``(a, >, b)`` or ``(b, <, a)`` |
        | ``a < b``         | ``(a, <, b)`` or ``(b, >, a)`` |
        | ``a >= b``        | ``(a, >=, b)`` or ``(b, <=, a)`` |
        | ``a <= b``        | ``(a, <=, b)`` or ``(b, >=, a)`` |
        | ``a = b``         | ``(a, =, b)`` or ``(b, =, a)`` |
        | ``a != b``        | ``(a, !=, b)`` or ``(b, !=, a)`` |
        +-------------------+-----------------------------+

        Because :meth:`assume_relation` always canonicalises before
        insertion, a single set lookup per equivalence class is enough —
        the table above tells us which canonical form to probe.
        """
        canon = _canon_relation(lhs, op, rhs)
        if canon in self._general_relations:
            return True
        return None


# ---------------------------------------------------------------------------
# Module-level helpers
# ---------------------------------------------------------------------------


# Maps the relation IR head symbol to the short op string used in the
# canonical triple.  Centralised so :meth:`assume_relation`,
# :meth:`forget_relation`, and :meth:`is_true_relation` agree on the
# vocabulary in one place.
_RELATION_HEAD_TO_OP: dict[IRNode, str] = {
    GREATER: ">",
    LESS: "<",
    GREATER_EQUAL: ">=",
    LESS_EQUAL: "<=",
    EQUAL: "=",
    NOT_EQUAL: "!=",
}


def _canon_relation(
    lhs: IRNode, op: str, rhs: IRNode
) -> tuple[IRNode, str, IRNode]:
    """Return a canonical ``(lhs, op, rhs)`` triple for the relation.

    Canonicalisation rules:

    - ``a < b`` is stored as ``(b, ">", a)`` — every strict inequality
      becomes a ``>``.
    - ``a <= b`` is stored as ``(b, ">=", a)`` — every non-strict
      inequality becomes a ``>=``.
    - ``a = b`` and ``a != b`` are commutative; we pick the lexicographic
      order of ``str(...)`` so duplicates from either argument order
      collapse to the same triple.
    - ``a > b`` and ``a >= b`` are stored verbatim.

    This is purely a deduplication strategy — it does not assert any
    semantic equivalence the caller didn't already imply.
    """
    if op == "<":
        return (rhs, ">", lhs)
    if op == "<=":
        return (rhs, ">=", lhs)
    if op in ("=", "!="):
        # Pick a deterministic order so a = b and b = a collapse.
        if _node_key(lhs) <= _node_key(rhs):
            return (lhs, op, rhs)
        return (rhs, op, lhs)
    return (lhs, op, rhs)


def _node_key(node: IRNode) -> str:
    """Deterministic ordering key for the commutativity tiebreak in
    :func:`_canon_relation`.  We use ``str(node)`` because every IR node
    has a structural ``__str__`` already (see ``symbolic_ir.nodes``).
    """
    return str(node)


def _sym_name(node: IRNode) -> str | None:
    """Return the name of ``node`` if it is an IRSymbol, else None."""
    if isinstance(node, IRSymbol):
        return node.name
    return None
