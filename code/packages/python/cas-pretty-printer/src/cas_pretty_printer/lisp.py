"""Always-prefix Lisp dialect — every node is rendered as ``(Head args...)``.

Useful for debugging the IR shape itself: skips all sugar, all
operator infix, all precedence — what you see is the tree.

    Add(2, Mul(3, x))  →  ``(Add 2 (Mul 3 x))``
"""

from __future__ import annotations

from symbolic_ir import IRApply, IRNode

from cas_pretty_printer.dialect import BaseDialect


class LispDialect(BaseDialect):
    """No sugar, no infix — pure prefix form."""

    name = "lisp"

    # Disable every operator spelling so the walker falls through to
    # function-call form on every IRApply.
    binary_ops: dict[str, str] = {}
    unary_ops: dict[str, str] = {}
    function_names: dict[str, str] = {}

    def call_brackets(self) -> tuple[str, str]:
        # We override the walker behavior entirely via try_sugar to
        # avoid emitting commas — Lisp uses spaces.
        return ("(", ")")

    def function_name(self, head_name: str) -> str:
        return head_name

    def try_sugar(self, node: IRApply) -> IRNode | None:
        # Sugar is the wrong tool here, but we use it as a hook so that
        # we own the formatting of every IRApply. We re-emit through a
        # custom synthetic head — but that would recurse forever. Instead,
        # leave try_sugar alone and rely on a custom head formatter
        # registered at module-import time. That keeps the Lisp output
        # uniform regardless of head.
        return None


def _lisp_format(node: IRApply, _dialect: object, fmt: object) -> str:
    """Used by the custom head formatter registered for Lisp output.

    Note that this function is intentionally reachable only when a user
    explicitly opts in via :func:`format_lisp` — registering it as a
    global head formatter would interfere with the other dialects.
    """
    raise NotImplementedError  # the actual entry point is `format_lisp`


def format_lisp(node: IRNode) -> str:
    """One-shot entry point for the Lisp dialect.

    Bypasses the walker's normal dispatch entirely — every node is
    rendered as either a leaf or ``(Head args...)``. Doesn't depend on
    any registered head formatter, so it's safe to call regardless of
    what other dialects have configured.
    """
    from symbolic_ir import IRFloat, IRInteger, IRRational, IRString, IRSymbol

    if isinstance(node, IRInteger):
        return str(node.value)
    if isinstance(node, IRRational):
        return f"{node.numer}/{node.denom}"
    if isinstance(node, IRFloat):
        return repr(node.value)
    if isinstance(node, IRString):
        return f'"{node.value}"'
    if isinstance(node, IRSymbol):
        return node.name
    if isinstance(node, IRApply):
        head_text = format_lisp(node.head)
        if not node.args:
            return f"({head_text})"
        args_text = " ".join(format_lisp(a) for a in node.args)
        return f"({head_text} {args_text})"
    raise TypeError(f"cannot Lisp-format node of type {type(node).__name__}")
