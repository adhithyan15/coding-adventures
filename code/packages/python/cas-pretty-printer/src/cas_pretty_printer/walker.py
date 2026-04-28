"""The dialect-agnostic IR walker.

The walker descends an IR tree and emits source text. It tracks one
piece of state — `min_prec`, the minimum precedence the emitted text
must have to avoid being parenthesized by its parent. Every dialect
hook (operator spellings, brackets, sugar) is consulted via the
`Dialect` protocol; the walker itself does not know which language it
is printing.

Algorithm
---------

For an `IRApply(head, args)` whose head is a known operator:

1. Try `dialect.try_sugar(node)`; if it returns a rewritten tree,
   format that recursively.
2. Try a registered head formatter (see :func:`register_head_formatter`).
3. If `head` is a binary operator with arity ≥ 2, emit
   ``arg op arg op ...`` with each child formatted at the correct
   `min_prec`. Left-associative operators give the rightmost child
   `parent_prec + 1`; right-associative operators give the leftmost
   child `parent_prec + 1`. (Variadic flat operators are unaffected
   either way — the same precedence on every child works.)
4. If `head` is a unary operator with arity 1, emit ``op{arg}`` with
   `arg` formatted at `parent_prec`.
5. If `head` is `List`, emit the list brackets.
6. Otherwise, function-call form: ``name(arg, arg, ...)``.

After step 3 or 4, parentheses are wrapped iff the operator's
precedence is strictly less than `min_prec`.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING

from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRString,
    IRSymbol,
)

if TYPE_CHECKING:
    from cas_pretty_printer.dialect import Dialect

# Type alias for a custom head formatter.
# Receives (node, dialect, format_recursively_fn) and returns a string.
HeadFormatter = Callable[[IRApply, "Dialect", Callable[[IRNode], str]], str]

# Registry consulted by the walker. Downstream packages register their
# heads here so cas-pretty-printer doesn't need to know about them.
_HEAD_FORMATTERS: dict[str, HeadFormatter] = {}


def register_head_formatter(head_name: str, formatter: HeadFormatter) -> None:
    """Teach the walker about a new head.

    The formatter is called whenever the walker encounters an
    ``IRApply`` whose head is a symbol with the given name. It receives
    the node, the active dialect, and a `fmt(child)` helper that
    formats children with no precedence context (the formatter is
    responsible for any nesting concerns). It must return a string.

    Example::

        def format_matrix(node, dialect, fmt):
            rows = [", ".join(fmt(c) for c in row.args)
                    for row in node.args]
            return "matrix(" + ", ".join(f"[{r}]" for r in rows) + ")"

        register_head_formatter("Matrix", format_matrix)
    """
    _HEAD_FORMATTERS[head_name] = formatter


def unregister_head_formatter(head_name: str) -> None:
    """Reverse of :func:`register_head_formatter`. Mostly for tests."""
    _HEAD_FORMATTERS.pop(head_name, None)


_PREC_ATOM = 100  # mirrors dialect.PREC_ATOM; duplicated here to avoid import cycle
_PREC_NEG = 55   # mirrors dialect.PREC_NEG; duplicated here to avoid import cycle


def pretty(node: IRNode, dialect: Dialect, *, style: str = "linear") -> str:
    """Format `node` as source text in the given dialect.

    Args:
        node: The IR tree.
        dialect: A :class:`Dialect` instance (typically a subclass of
            :class:`BaseDialect`).
        style: Reserved for future ``"2d"`` ASCII output. Currently only
            ``"linear"`` is supported.

    Returns:
        A single-line (linear style) string.
    """
    if style != "linear":
        raise ValueError(f"unsupported style {style!r}")
    return _format(node, dialect, min_prec=0)


def _format(node: IRNode, dialect: Dialect, min_prec: int) -> str:
    if isinstance(node, IRInteger):
        return _format_integer(node, dialect, min_prec)
    if isinstance(node, IRRational):
        return _format_rational(node, dialect, min_prec)
    if isinstance(node, IRFloat):
        return _format_float(node, dialect, min_prec)
    if isinstance(node, IRString):
        return dialect.format_string(node.value)
    if isinstance(node, IRSymbol):
        return dialect.format_symbol(node.name)
    if isinstance(node, IRApply):
        return _format_apply(node, dialect, min_prec)
    raise TypeError(f"cannot pretty-print node of type {type(node).__name__}")


def _format_integer(node: IRInteger, dialect: Dialect, min_prec: int) -> str:
    text = dialect.format_integer(node.value)
    # Negative literals need parentheses only when the surrounding operator
    # binds *more tightly* than unary minus (PREC_NEG = 55).  This gives:
    #
    #   -2*y   →  no parens (Mul prec 50 < 55),  output  -2*y
    #   2^(-3) →  parens    (Pow prec 60 > 55),  output  2^(-3)
    #
    # The old threshold of `min_prec > 0` was too conservative and produced
    # the ugly `(-2)*y` form even though `-2*y` is unambiguous.
    if node.value < 0 and min_prec > _PREC_NEG:
        return f"({text})"
    return text


def _format_rational(node: IRRational, dialect: Dialect, min_prec: int) -> str:
    text = dialect.format_rational(node.numer, node.denom)
    if (node.numer < 0 or "/" in text) and min_prec > 0:
        return f"({text})"
    return text


def _format_float(node: IRFloat, dialect: Dialect, min_prec: int) -> str:
    text = dialect.format_float(node.value)
    if node.value < 0 and min_prec > 0:
        return f"({text})"
    return text


def _format_apply(node: IRApply, dialect: Dialect, min_prec: int) -> str:
    # 1. Sugar.
    sugared = dialect.try_sugar(node)
    if sugared is not None and sugared is not node:
        return _format(sugared, dialect, min_prec)

    head = node.head
    head_name = head.name if isinstance(head, IRSymbol) else None

    # 2. Custom head formatter.
    if head_name is not None and head_name in _HEAD_FORMATTERS:
        def fmt_helper(child: IRNode) -> str:
            return _format(child, dialect, 0)
        return _HEAD_FORMATTERS[head_name](node, dialect, fmt_helper)

    # 3. List literal.
    if head_name == "List":
        return _format_list(node, dialect)

    # 4. Unary op.
    if head_name is not None and len(node.args) == 1:
        op_text = dialect.unary_op(head_name)
        if op_text is not None:
            prec = dialect.precedence(head_name)
            inner = _format(node.args[0], dialect, prec)
            text = f"{op_text}{inner}"
            return _wrap_if_needed(text, prec, min_prec)

    # 5. Binary / n-ary op.
    if head_name is not None and len(node.args) >= 2:
        op_text = dialect.binary_op(head_name)
        if op_text is not None:
            prec = dialect.precedence(head_name)
            right_assoc = dialect.is_right_associative(head_name)
            parts: list[str] = []
            n = len(node.args)
            for i, arg in enumerate(node.args):
                # For variadic flattened ops, parent_prec on every child
                # is fine. For binary 2-arg with associativity:
                #   left-assoc: rightmost child needs prec+1
                #   right-assoc: leftmost child needs prec+1
                if right_assoc and i < n - 1 or (not right_assoc) and i > 0:
                    child_prec = prec + 1
                else:
                    child_prec = prec
                parts.append(_format(arg, dialect, child_prec))
            text = op_text.join(parts)
            return _wrap_if_needed(text, prec, min_prec)

    # 6. Function call.
    return _format_call(node, dialect)


def _format_list(node: IRApply, dialect: Dialect) -> str:
    open_b, close_b = dialect.list_brackets()
    args = ", ".join(_format(a, dialect, 0) for a in node.args)
    return f"{open_b}{args}{close_b}"


def _format_call(node: IRApply, dialect: Dialect) -> str:
    head = node.head
    if isinstance(head, IRSymbol):
        name = dialect.function_name(head.name)
    else:
        # Higher-order: head is itself a compound expression.
        # Render it at atom precedence so it gets wrapped if needed.
        name = _format(head, dialect, _PREC_ATOM)
    open_b, close_b = dialect.call_brackets()
    args = ", ".join(_format(a, dialect, 0) for a in node.args)
    return f"{name}{open_b}{args}{close_b}"


def _wrap_if_needed(text: str, prec: int, min_prec: int) -> str:
    if prec < min_prec:
        return f"({text})"
    return text
