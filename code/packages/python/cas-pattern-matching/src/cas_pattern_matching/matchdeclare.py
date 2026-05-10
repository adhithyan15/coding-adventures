"""MACSYMA ``matchdeclare`` — per-VM pattern-variable declaration store.

:class:`MatchDeclareContext` records which symbols are pattern variables
and what type predicate constrains them.  It also compiles raw IR patterns
into the ``Pattern(name, Blank(...))`` form that the
:mod:`cas_pattern_matching.matcher` understands.

Every :class:`~symbolic_vm.vm.VM` carries one ``MatchDeclareContext`` as
``vm.match_declarations``.  All mutations happen through the VM's
``matchdeclare_handler``; library code (compile, query) is pure.

Supported predicates
--------------------
The table below maps the lowercase string tag (the name of the MACSYMA
predicate symbol, lowercased) to the head name passed to
:func:`~cas_pattern_matching.nodes.Blank`.  ``None`` means unconstrained
(any expression matches).

+------------------+-------------------+--------------------------------+
| MACSYMA tag      | Blank head        | Matches                        |
+==================+===================+================================+
| ``true``         | (none)            | anything                       |
+------------------+-------------------+--------------------------------+
| ``all``          | (none)            | anything                       |
+------------------+-------------------+--------------------------------+
| ``any``          | (none)            | anything (internal default)    |
+------------------+-------------------+--------------------------------+
| ``integerp``     | ``"Integer"``     | IRInteger literals             |
+------------------+-------------------+--------------------------------+
| ``symbolp``      | ``"Symbol"``      | IRSymbol (bare names)          |
+------------------+-------------------+--------------------------------+
| ``floatp``       | ``"Float"``       | IRFloat literals               |
+------------------+-------------------+--------------------------------+
| ``rationalp``    | ``"Rational"``    | IRRational literals            |
+------------------+-------------------+--------------------------------+
| ``numberp``      | (none)            | union — unconstrained fallback |
+------------------+-------------------+--------------------------------+
| ``listp``        | ``"List"``        | IRApply with head List         |
+------------------+-------------------+--------------------------------+
| ``stringp``      | ``"String"``      | IRString literals              |
+------------------+-------------------+--------------------------------+
| (unknown tag)    | (none)            | safe fallback — matches any    |
+------------------+-------------------+--------------------------------+

Example usage::

    from cas_pattern_matching.matchdeclare import MatchDeclareContext
    from symbolic_ir import IRSymbol, IRApply, SIN, POW, ADD, COS, IRInteger

    ctx = MatchDeclareContext()
    ctx.declare("x", "any")

    # Compile: sin(x)^2 + cos(x)^2  →  pattern with x as wildcard
    x = IRSymbol("x")
    raw = IRApply(ADD, (
        IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
        IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
    ))
    compiled = ctx.compile_pattern(raw)
    # x nodes become Pattern("x", Blank())
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRNode, IRSymbol

from cas_pattern_matching.nodes import Blank, Pattern

# ---------------------------------------------------------------------------
# Predicate tag → Blank head constraint
# ---------------------------------------------------------------------------

# Map from normalised (lowercase) predicate tag to the Blank head-constraint
# string.  ``None`` means an unconstrained Blank(), which matches anything.
_PRED_TO_BLANK_HEAD: dict[str, str | None] = {
    "true": None,
    "all": None,
    "any": None,
    "integerp": "Integer",
    "symbolp": "Symbol",
    "floatp": "Float",
    "rationalp": "Rational",
    "numberp": None,   # numeric union — fall back to unconstrained
    "listp": "List",
    "stringp": "String",
}


# ---------------------------------------------------------------------------
# Public class
# ---------------------------------------------------------------------------


class MatchDeclareContext:
    """Per-VM store of ``matchdeclare`` declarations.

    One instance lives on each :class:`~symbolic_vm.vm.VM` as
    ``vm.match_declarations``.  All mutations are in-place; the store
    is intentionally mutable so the user can redeclare variables during
    a session.

    Thread-safety: none — the VM is single-threaded.
    """

    def __init__(self) -> None:
        # Maps symbol name → normalised predicate tag string.
        self._decls: dict[str, str] = {}

    # ------------------------------------------------------------------
    # Mutation API — called by the VM's matchdeclare_handler
    # ------------------------------------------------------------------

    def declare(self, sym_name: str, pred_tag: str) -> None:
        """Record that ``sym_name`` is a pattern variable.

        Parameters
        ----------
        sym_name:
            The name of the IR symbol to mark as a pattern variable.
        pred_tag:
            A predicate tag string such as ``"true"``, ``"integerp"``,
            ``"symbolp"``.  Stored lowercased; unknown tags are kept
            and treated as unconstrained when compiling.
        """
        self._decls[sym_name] = pred_tag.lower()

    def forget(self, sym_name: str) -> None:
        """Remove the declaration for ``sym_name``, if any."""
        self._decls.pop(sym_name, None)

    def forget_all(self) -> None:
        """Remove all pattern-variable declarations."""
        self._decls.clear()

    # ------------------------------------------------------------------
    # Query API
    # ------------------------------------------------------------------

    def is_declared(self, sym_name: str) -> bool:
        """True if ``sym_name`` has been declared as a pattern variable."""
        return sym_name in self._decls

    def get_predicate(self, sym_name: str) -> str | None:
        """Return the normalised predicate tag for ``sym_name``, or None."""
        return self._decls.get(sym_name)

    # ------------------------------------------------------------------
    # Pattern compilation — called by defrule_handler / tellsimp_handler
    # ------------------------------------------------------------------

    def compile_pattern(self, pattern: IRNode) -> IRNode:
        """Compile ``pattern`` into matcher-ready IR.

        Every ``IRSymbol`` whose name appears in this context is replaced
        with ``Pattern(name, Blank(constraint))``.  All other nodes pass
        through structurally unchanged (head and args both walked).

        Parameters
        ----------
        pattern:
            Raw IR expression to compile, typically a rule LHS written
            in terms of declared variable names.

        Returns
        -------
        IRNode
            The compiled IR with pattern-variable placeholders inserted.
            Atoms and non-declared symbols are returned as-is.
        """
        return self._walk(pattern)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _walk(self, node: IRNode) -> IRNode:
        """Recursively compile one node."""
        # IRSymbol: replace if declared.
        if isinstance(node, IRSymbol):
            pred_tag = self._decls.get(node.name)
            if pred_tag is not None:
                blank_head = _PRED_TO_BLANK_HEAD.get(pred_tag)
                # blank_head = None  → Blank() (unconstrained wildcard)
                # blank_head = str   → Blank("Integer") etc.
                return Pattern(node.name, Blank(blank_head))
            return node

        # IRApply: recurse into head and each arg.
        if isinstance(node, IRApply):
            new_head = self._walk(node.head)
            new_args = tuple(self._walk(a) for a in node.args)
            if new_head is node.head and new_args == node.args:
                return node
            return IRApply(new_head, new_args)

        # All other atoms (IRInteger, IRRational, IRFloat, IRString) pass
        # through unchanged — they are literal constants in the pattern.
        return node
