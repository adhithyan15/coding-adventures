"""VM handlers for trig transformation heads.

These handlers are installed on :class:`~symbolic_vm.backends.SymbolicBackend`
via :func:`build_trig_handler_table`.

Handler signature (must match ``symbolic_vm.backend.Handler``)::

    def handler(vm: VM, expr: IRApply) -> IRNode
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Callable

from symbolic_ir import IRApply, IRNode

from cas_trig.expand import trig_expand
from cas_trig.reduce import trig_reduce
from cas_trig.simplify import trig_simplify

if TYPE_CHECKING:
    from symbolic_vm.vm import VM  # pragma: no cover

Handler = Callable[["VM", IRApply], IRNode]


def trig_simplify_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``TrigSimplify(expr)`` — apply Pythagorean and sign-rule rewrites."""
    if len(expr.args) != 1:
        return expr
    return trig_simplify(expr.args[0])


def trig_expand_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``TrigExpand(expr)`` — expand compound trig arguments."""
    if len(expr.args) != 1:
        return expr
    return trig_expand(expr.args[0])


def trig_reduce_handler(_vm: "VM", expr: IRApply) -> IRNode:
    """``TrigReduce(expr)`` — reduce trig powers to multiple-angle form."""
    if len(expr.args) != 1:
        return expr
    return trig_reduce(expr.args[0])


def build_trig_handler_table() -> dict[str, Handler]:
    """Return a handler dict suitable for merging into ``SymbolicBackend``."""
    return {
        "TrigSimplify": trig_simplify_handler,
        "TrigExpand": trig_expand_handler,
        "TrigReduce": trig_reduce_handler,
    }
