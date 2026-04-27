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

from symbolic_ir import IRApply, IRNode, IRSymbol
from symbolic_vm.backend import Handler

if TYPE_CHECKING:
    from symbolic_vm import VM

    from macsyma_runtime.backend import MacsymaBackend


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
        if "numer" in flags or "float" in flags:
            backend = vm.backend
            if hasattr(backend, "with_numer"):
                with backend.with_numer():
                    result: IRNode = vm.eval(inner)
            else:
                result = vm.eval(inner)
            return result

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
