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

    Phase A only honours the ``numer`` flag (force-collapse to floats).
    Future phases will add ``simp``, ``expand``, ``factor``, ``ratsimp``,
    etc. For now, an unknown flag is silently ignored.
    """

    def ev_handler(vm: VM, expr: IRApply) -> IRNode:
        # We need the un-evaluated expression form here. The simplest
        # contract for Phase A: every flag is an IRSymbol that arrives
        # as itself (because they are all unbound). Loop through flags,
        # gather them into a set, then re-evaluate the first arg.
        if not expr.args:
            return expr
        first = expr.args[0]
        flags: set[str] = set()
        for arg in expr.args[1:]:
            if isinstance(arg, IRSymbol):
                flags.add(arg.name)
        if "numer" in flags:
            # Re-evaluate `first` with the numer flag set on the backend.
            backend = vm.backend
            if hasattr(backend, "with_numer"):
                with backend.with_numer():
                    return vm.eval(first)
        return first

    return ev_handler
