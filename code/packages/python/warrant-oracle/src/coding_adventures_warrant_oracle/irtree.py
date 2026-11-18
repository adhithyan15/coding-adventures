"""The shape of an IR tree: which parts of a node are sub-expressions (VER04r Appendix A, A.1).

Almost every IR node is ``[tag, operand, operand, ..]`` and its sub-expressions are its list
operands.  Four shapes are not, and a walker that treats them as if they were would **silently
drop sub-expressions** (a subject call inside a tuple literal would not be counted):

``["subject_op", name, [args..]]`` and ``["model_call", name, [args..]]``
    the arguments are one list in position 2;
``["tuple", [e1, .., en]]`` (``ir.tuple``)
    the operands are one list in position 1, so ``node[1]`` is a *list of nodes*, not a node;
``["match", scrutinee, [[pattern, arm], ..]]`` (``ir.tuple``, ``match`` on a ``Result``)
    the sub-expressions are the scrutinee and the arm **bodies**.  A *pattern* is not an
    expression, although it looks like one: ``["ok", ["bind", "x"]]`` has the shape of the
    ``ok`` constructor and ``["tuple", [..]]`` the shape of a tuple literal.  A walker must never
    visit a pattern as an expression.

:func:`children` is the one place that knows this, so every walker in the package (the evaluator's
analyses, ``requires`` inference, the lowerer's ``walk``) uses it.
"""

from __future__ import annotations

from .errors import OracleError

#: Nodes with no sub-expression.  ``max_value``/``min_value`` carry only a type name.
_LEAVES = frozenset(["lit", "var", "err", "max_value", "min_value"])


def is_node(value: object) -> bool:
    """Is ``value`` an IR node, a list whose first element is a tag string?"""
    return isinstance(value, list) and bool(value) and isinstance(value[0], str)


def match_arms(node: list) -> list:
    """The ``[pattern, body]`` pairs of a ``match`` node, shape-checked (``ir`` error if malformed)."""
    arms = node[2] if len(node) == 3 else None
    if (
        not isinstance(arms, list)
        or not arms
        or not all(isinstance(a, list) and len(a) == 2 for a in arms)
    ):
        raise OracleError("ir", "a malformed match node")
    return arms


def children(node: list) -> list:
    """The sub-expressions of ``node`` in evaluation order (patterns are never among them)."""
    tag = node[0]
    if tag in _LEAVES:
        return []
    if tag in ("subject_op", "model_call"):
        return list(node[2])
    if tag == "tuple":
        return list(node[1])
    if tag == "match":
        return [node[1], *(body for _pattern, body in match_arms(node))]
    return [c for c in node[1:] if isinstance(c, list)]


def free_vars(ir: object) -> set[str]:
    """The variables an IR tree reads that no ``match`` pattern inside it binds.

    There is no shadowing (VER04r A.1), so a name bound by a pattern is never also a binder or a
    parameter, and "read minus bound" is exactly the free set.
    """
    reads: set[str] = set()
    bound: set[str] = set()
    stack = [ir]
    while stack:
        node = stack.pop()
        if not is_node(node):
            continue
        if node[0] == "var":
            reads.add(node[1])
            continue
        if node[0] == "match":
            for pattern, _body in match_arms(node):
                bound.update(pattern_binds(pattern))
        stack.extend(children(node))
    return reads - bound


def pattern_binds(pattern: object) -> list[str]:
    """The names a pattern binds, in source order (``bind`` nodes, looking inside ``ok``/``tuple``)."""
    out: list[str] = []
    stack = [pattern]
    while stack:
        p = stack.pop()
        if not is_node(p):
            continue
        if p[0] == "bind" and len(p) == 2 and isinstance(p[1], str):
            out.append(p[1])
        elif p[0] == "ok" and len(p) == 2:
            stack.append(p[1])
        elif p[0] == "tuple" and len(p) == 2 and isinstance(p[1], list):
            stack.extend(reversed(p[1]))
    return out
