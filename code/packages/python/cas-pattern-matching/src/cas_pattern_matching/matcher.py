"""Structural matcher for IR patterns.

Algorithm
---------
:func:`match` recursively descends both the pattern and the target,
comparing them node-by-node. The five cases:

1. ``Blank()`` — matches anything; no binding.
2. ``Blank(head)`` — matches any node whose effective head equals
   ``head`` (compounds compare ``IRApply.head.name``; leaves use their
   type tag, e.g. ``"Integer"`` for ``IRInteger``).
3. ``Pattern(name, inner)`` — runs the inner match. On success, binds
   ``name → target``; if ``name`` was already bound to a different
   value, the whole match fails.
4. Compound ``IRApply`` patterns — head must equal head, args zip
   pairwise (no sequence wildcards in this Phase B release).
5. Otherwise — structural equality (``pattern == target``).
"""

from __future__ import annotations

from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_pattern_matching.bindings import Bindings
from cas_pattern_matching.nodes import (
    blank_head,
    is_blank,
    is_pattern,
    pattern_inner,
    pattern_name,
)


def match(
    pattern: IRNode,
    target: IRNode,
    bindings: Bindings | None = None,
) -> Bindings | None:
    """Try to match ``pattern`` against ``target``.

    Returns the resulting :class:`Bindings` on success or ``None`` on
    failure. Pass an existing ``bindings`` to extend it; the matcher
    never mutates it.
    """
    if bindings is None:
        bindings = Bindings()

    # 1. ``Blank``: matches anything (subject to optional head check).
    if is_blank(pattern):
        assert isinstance(pattern, IRApply)  # for type-narrowing
        head_constraint = blank_head(pattern)
        if head_constraint is None:
            return bindings
        if _effective_head_name(target) == head_constraint:
            return bindings
        return None

    # 2. ``Pattern``: bind a name around an inner pattern.
    if is_pattern(pattern):
        assert isinstance(pattern, IRApply)
        name = pattern_name(pattern)
        sub = match(pattern_inner(pattern), target, bindings)
        if sub is None:
            return None
        if name in sub:
            return sub if sub[name] == target else None
        return sub.bind(name, target)

    # 3. Compound vs compound: head + zipped args.
    if isinstance(pattern, IRApply):
        if not isinstance(target, IRApply):
            return None
        head_match = match(pattern.head, target.head, bindings)
        if head_match is None:
            return None
        if len(pattern.args) != len(target.args):
            return None
        # No sequence wildcards in this release: a literal pairwise zip
        # is sufficient. When BlankSequence lands this loop becomes a
        # backtracking search.
        cur: Bindings | None = head_match
        for p_arg, t_arg in zip(pattern.args, target.args, strict=True):
            if cur is None:
                return None
            cur = match(p_arg, t_arg, cur)
        return cur

    # 4. Leaves: structural equality.
    if pattern == target:
        return bindings
    return None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _effective_head_name(node: IRNode) -> str:
    """Return the head-tag used by :func:`Blank` head constraints.

    Compounds use their head symbol's name; literals use a Python-style
    type tag so ``Blank("Integer")`` matches every integer.
    """
    if isinstance(node, IRApply):
        return node.head.name if isinstance(node.head, IRSymbol) else "Apply"
    if isinstance(node, IRInteger):
        return "Integer"
    if isinstance(node, IRRational):
        return "Rational"
    if isinstance(node, IRFloat):
        return "Float"
    if isinstance(node, IRString):
        return "String"
    if isinstance(node, IRSymbol):
        return "Symbol"
    return type(node).__name__
