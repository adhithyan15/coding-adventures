"""Handlers for the MACSYMA-runtime-owned heads.

Each handler conforms to the symbolic-vm :data:`Handler` signature:

    def handler(vm: VM, expr: IRApply) -> IRNode

For Phase A the runtime owns five heads:

- ``Display`` — terminator wrapper for ``;``. Identity-on-inner.
- ``Suppress`` — terminator wrapper for ``$``. Identity-on-inner.
- ``Kill``    — clear bindings.
- ``Ev``      — re-evaluate with flags.
- ``Block``   — reserved for Phase G.

The runtime keeps these heads in :mod:`macsyma_runtime.heads` so they
are easy to import as singletons.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import (
    LIST,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)
from symbolic_vm.backend import Handler

if TYPE_CHECKING:
    from symbolic_vm import VM

    from macsyma_runtime.backend import MacsymaBackend


# ---------------------------------------------------------------------------
# Numer fold — recursive exact-to-float conversion
# ---------------------------------------------------------------------------


def _numer_fold(node: IRNode) -> IRNode:
    """Recursively convert exact numerics to :class:`~symbolic_ir.IRFloat`.

    In MACSYMA, ``ev(expr, numer)`` forces every exact rational or integer
    sub-expression to a floating-point value.  Constants such as ``%pi`` and
    ``%e`` are pre-bound as ``IRFloat`` by the backend, so they are already
    handled.  The only cases that need explicit conversion are:

    - ``IRInteger`` — exact integer (e.g. ``3``)   → ``IRFloat(3.0)``
    - ``IRRational`` — exact fraction (e.g. ``1/2``) → ``IRFloat(0.5)``
    - ``IRApply`` — recurse into args; special-case ``Pow`` to preserve
      integer exponents so that ``x^2`` is not changed to ``x^2.0``
      (the underlying numeric routines expect integer exponents in Pow).
    - Everything else (``IRSymbol``, ``IRFloat``) — returned unchanged.

    This function is *pure* — it never mutates nodes.  If nothing changes
    the original node is returned by identity so callers can do a fast
    ``is``-check.
    """
    # --- leaf: exact integer → float ----------------------------------------
    if isinstance(node, IRInteger):
        return IRFloat(float(node.value))

    # --- leaf: exact rational → float ----------------------------------------
    if isinstance(node, IRRational):
        return IRFloat(node.numer / node.denom)

    # --- leaf: already float or symbol → no-op --------------------------------
    if isinstance(node, (IRFloat, IRSymbol)):
        return node

    # --- compound: recurse with Pow-exponent guard ----------------------------
    if isinstance(node, IRApply):
        head = node.head
        # For Pow(base, exp) keep the exponent exact so that ``x^2`` stays
        # ``x^2`` (not ``x^2.0``).  Only fold the base.
        if (
            isinstance(head, IRSymbol)
            and head.name == "Pow"
            and len(node.args) == 2
        ):
            new_base = _numer_fold(node.args[0])
            exp = node.args[1]          # keep exponent as-is
            if new_base is node.args[0]:
                return node             # nothing changed — return original
            return IRApply(head, (new_base, exp))

        # General case — fold every argument.
        new_args = tuple(_numer_fold(a) for a in node.args)
        if new_args == node.args:
            return node
        return IRApply(head, new_args)

    # --- fallback (e.g. future IR node types) ---------------------------------
    return node


def display_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Display(inner)`` returns ``inner`` unchanged.

    The VM has already evaluated ``inner`` (held heads are an opt-in
    list and Display is not held). The REPL inspected the head before
    evaluation to decide whether to print. By the time we get here the
    wrapper has done its job and we just unwrap.
    """
    if len(expr.args) != 1:
        raise ValueError(f"Display takes 1 arg, got {len(expr.args)}")
    return expr.args[0]


def suppress_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Suppress(inner)`` returns ``inner`` unchanged. Twin of Display."""
    if len(expr.args) != 1:
        raise ValueError(f"Suppress takes 1 arg, got {len(expr.args)}")
    return expr.args[0]


def make_kill_handler(backend: MacsymaBackend) -> Handler:
    """Build a ``Kill`` handler bound to a particular backend.

    ``Kill`` mutates the backend's environment, so it can't be a plain
    free function — it needs the backend reference.
    """

    def kill_handler(_vm: VM, expr: IRApply) -> IRNode:
        # The args were evaluated before reaching us. But for `kill(x)`
        # we want to clear the binding for the *symbol name x*, not
        # whatever x evaluates to. The Symbolic backend leaves unbound
        # names unchanged, so a fresh symbol still arrives as
        # IRSymbol("x"). For names that *are* bound, the user's intent
        # of `kill(x)` is to clear x, not to inspect its value — so
        # we accept either form: if we see an IRSymbol we use its name;
        # if we see anything else we silently do nothing for that arg.
        for arg in expr.args:
            if isinstance(arg, IRSymbol):
                if arg.name == "all":
                    backend.reset_environment()
                else:
                    backend.unbind(arg.name)
        return _DONE

    return kill_handler


# A tiny sentinel that downstream code can ignore. Kill is "for its
# side effect" — there is no meaningful return value. We use the
# IRSymbol("done") shape Maxima itself uses.
_DONE = IRSymbol("done")


def declare_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Declare(sym, property, ...)`` records MACSYMA symbol properties.

    Properties are stored in the VM's existing assumption context so they feed
    the same simplification and ``is(...)`` machinery as ``assume(x, prop)``.
    Arguments are consumed as symbol/property pairs:
    ``declare(n, integer, x, positive)``.
    """
    if len(expr.args) % 2 != 0:
        return expr
    for i in range(0, len(expr.args), 2):
        sym, prop = expr.args[i], expr.args[i + 1]
        vm.assumptions.assume_property(sym, prop)
    return _DONE


def properties_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Properties(sym)`` returns a list of properties declared for ``sym``."""
    if len(expr.args) != 1:
        return expr
    target = expr.args[0]
    if not isinstance(target, IRSymbol):
        return IRApply(LIST, ())
    return IRApply(
        LIST,
        tuple(IRSymbol(fact) for fact in vm.assumptions.facts_for(target.name)),
    )


def propvars_handler(vm: VM, expr: IRApply) -> IRNode:
    """``PropVars()`` returns symbols that currently have declared properties."""
    if expr.args:
        return expr
    return IRApply(
        LIST,
        tuple(IRSymbol(name) for name in vm.assumptions.symbols_with_facts()),
    )


def make_ev_handler() -> Handler:
    """Build the ``Ev(expr, *flags)`` handler.

    Supported flags
    ---------------
    ``numer`` / ``float``
        Force numeric (floating-point) evaluation.  Folds all exact
        rationals and constants to ``IRFloat``.
    ``expand``
        Apply ``Expand`` to the result before returning.
    ``factor``
        Apply ``Factor`` to the result before returning.
    ``ratsimp``
        Apply ``RatSimplify`` (cancel GCD of numerator/denominator) to
        the result before returning.  Implemented via A3 substrate.
    ``trigsimp``
        Apply ``TrigSimplify`` (Pythagorean identities etc.) to the
        result before returning.  Implemented via B1 substrate.

    Unknown flags are silently ignored so that future flags don't break
    existing sessions.
    """

    def ev_handler(vm: VM, expr: IRApply) -> IRNode:
        # Every flag is an IRSymbol that arrives as itself (unbound).
        # Collect them, then evaluate the first arg with the appropriate
        # post-processing applied.
        if not expr.args:
            return expr
        inner = expr.args[0]
        flags: set[str] = set()
        for arg in expr.args[1:]:
            if isinstance(arg, IRSymbol):
                flags.add(arg.name)

        # ---- numer / float ------------------------------------------------
        # Evaluate the expression, then fold every exact rational/integer
        # leaf to IRFloat.  ``with_numer`` is used when available so that
        # any *downstream* ev() calls also stay in float mode; the fold
        # at the end guarantees the returned value is fully numeric.
        if "numer" in flags or "float" in flags:
            backend = vm.backend
            if hasattr(backend, "with_numer"):
                with backend.with_numer():
                    result: IRNode = vm.eval(inner)
            else:
                result = vm.eval(inner)
            return _numer_fold(result)

        # ---- plain evaluation first, then post-process --------------------
        result = vm.eval(inner)

        if "expand" in flags:
            result = vm.eval(IRApply(IRSymbol("Expand"), (result,)))

        if "factor" in flags:
            result = vm.eval(IRApply(IRSymbol("Factor"), (result,)))

        if "ratsimp" in flags:
            result = vm.eval(IRApply(IRSymbol("RatSimplify"), (result,)))

        if "trigsimp" in flags:
            result = vm.eval(IRApply(IRSymbol("TrigSimplify"), (result,)))

        return result

    return ev_handler
