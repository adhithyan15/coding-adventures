"""Pattern-related IR sentinel heads and constructors.

We model patterns *inside* the existing :class:`symbolic_ir.IRApply`
structure rather than introducing new dataclass types. This keeps the
matcher uniform — every pattern is just an IR tree — and avoids
copying machinery (hashing, equality) that the IR already provides.

The trade-off: we can't put a Python predicate inside an IRApply
(those have to be hashable). ``Condition`` is therefore deferred to a
later release, which will introduce a registered-predicate table.
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRNode, IRSymbol

# Sentinel heads. Module-level singletons so identity comparison works.
BLANK = IRSymbol("Blank")
PATTERN = IRSymbol("Pattern")
RULE = IRSymbol("Rule")
RULE_DELAYED = IRSymbol("RuleDelayed")
REPLACE = IRSymbol("Replace")
REPLACE_ALL = IRSymbol("ReplaceAll")
REPLACE_REPEATED = IRSymbol("ReplaceRepeated")


# ---------------------------------------------------------------------------
# Helper constructors
# ---------------------------------------------------------------------------


def Blank(head: str | None = None) -> IRApply:
    """Construct an anonymous wildcard pattern.

    ``Blank()`` matches any single expression. ``Blank("Integer")``
    matches only literals of the named type or compounds whose head is
    that name. The head check is loose: leaves are matched by their
    type-tag (``"Integer"``, ``"Symbol"``, ``"Rational"``, ``"Float"``,
    ``"String"``); compounds match by their head's symbol name.
    """
    if head is None:
        return IRApply(BLANK, ())
    return IRApply(BLANK, (IRSymbol(head),))


def Pattern(name: str, inner: IRNode) -> IRApply:
    """Construct a named-pattern wrapper.

    Inside the matcher, when the inner pattern matches a target, the
    binding ``name → target`` is recorded. If ``name`` is already
    bound to a different target, the match fails.
    """
    return IRApply(PATTERN, (IRSymbol(name), inner))


def Rule(lhs: IRNode, rhs: IRNode) -> IRApply:
    """Construct an immediate-substitution rewrite rule.

    The rewriter applies the rule by:

    1. Matching ``lhs`` against the current expression.
    2. On success, substituting the captured bindings into ``rhs``
       and returning the result.
    """
    return IRApply(RULE, (lhs, rhs))


def RuleDelayed(lhs: IRNode, rhs: IRNode) -> IRApply:
    """Construct a delayed-substitution rule.

    Identical to :func:`Rule` for the matcher; reserved separately so
    future passes that *evaluate* RHSes (vs. just substituting into
    them) can distinguish the two.
    """
    return IRApply(RULE_DELAYED, (lhs, rhs))


# ---------------------------------------------------------------------------
# Inspection helpers
# ---------------------------------------------------------------------------


def is_blank(node: IRNode) -> bool:
    return isinstance(node, IRApply) and node.head == BLANK


def is_pattern(node: IRNode) -> bool:
    return isinstance(node, IRApply) and node.head == PATTERN


def is_rule(node: IRNode) -> bool:
    return (
        isinstance(node, IRApply)
        and node.head in (RULE, RULE_DELAYED)
        and len(node.args) == 2
    )


def pattern_name(node: IRApply) -> str:
    """Extract the bound name from a ``Pattern(name, inner)`` apply."""
    head = node.args[0]
    if not isinstance(head, IRSymbol):
        raise ValueError(f"Pattern name must be IRSymbol, got {head!r}")
    return head.name


def pattern_inner(node: IRApply) -> IRNode:
    """Extract the inner sub-pattern from a ``Pattern(name, inner)`` apply."""
    return node.args[1]


def blank_head(node: IRApply) -> str | None:
    """Extract the optional head constraint from a ``Blank`` apply.

    Returns the head name (e.g., ``"Integer"``) or None if the blank
    is unconstrained.
    """
    if not node.args:
        return None
    head = node.args[0]
    if not isinstance(head, IRSymbol):
        raise ValueError(f"Blank head must be IRSymbol, got {head!r}")
    return head.name
