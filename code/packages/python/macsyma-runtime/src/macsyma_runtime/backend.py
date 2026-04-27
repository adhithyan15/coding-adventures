"""The MACSYMA-flavored backend.

:class:`MacsymaBackend` is a thin subclass of :class:`SymbolicBackend`.
It adds:

- A :class:`History` reference, consulted by :meth:`lookup` so user
  references to ``%``, ``%i3``, ``%o3`` resolve transparently.
- Handlers for the runtime-owned heads (`Display`, `Suppress`, `Kill`,
  `Ev`).
- Handlers for all CAS substrate heads: `Simplify`, `Expand`, `Subst`,
  `Factor`, `Solve`, list operations, matrix operations, `Limit`, `Taylor`,
  and numeric helpers like `Abs`, `Floor`, `Ceiling`, etc.
- Pre-bound constants: ``%pi`` and ``%e`` resolve to their float values.
- Option-flag attributes (`numer`, `simp`, ...) plus a context-manager
  helper :meth:`with_numer` for short-lived flag overrides used by `Ev`.

Everything else — arithmetic, calculus, identity rewrites, list
passthrough — comes from :class:`SymbolicBackend` unchanged.
"""

from __future__ import annotations

import math
from collections.abc import Iterator, Mapping
from contextlib import contextmanager

from symbolic_ir import IRFloat, IRNode, IRSymbol
from symbolic_vm import SymbolicBackend
from symbolic_vm.backend import Handler

from macsyma_runtime.cas_handlers import build_cas_handler_table
from macsyma_runtime.handlers import (
    display_handler,
    make_ev_handler,
    make_kill_handler,
    suppress_handler,
)
from macsyma_runtime.heads import DISPLAY, EV, KILL, SUPPRESS
from macsyma_runtime.history import History


class MacsymaBackend(SymbolicBackend):
    """SymbolicBackend with the MACSYMA-specific runtime layer."""

    #: Numeric-mode flag: when set, `Ev(expr, numer)` forces the
    #: evaluator to collapse symbolic constants and arithmetic to
    #: floats. Phase A treats `numer` as a hint that does not yet
    #: change the symbolic backend's behavior — reserved.
    numer: bool

    #: Whether automatic simplification is enabled. Maxima's `simp`
    #: flag. Phase A defaults true; not consulted yet.
    simp: bool

    #: The session's I/O history, owned by the REPL but referenced by
    #: the backend so VM lookups can resolve `%`, `%iN`, `%oN`.
    history: History

    def __init__(self, *, history: History | None = None) -> None:
        super().__init__()
        self.history = history if history is not None else History()
        self.numer = False
        self.simp = True

        # Patch the inherited handler table with the runtime's heads.
        # ``SymbolicBackend.__init__`` filled ``self._handlers`` already.
        runtime_handlers: dict[str, Handler] = {
            DISPLAY.name: display_handler,
            SUPPRESS.name: suppress_handler,
            KILL.name: make_kill_handler(self),
            EV.name: make_ev_handler(),
        }
        # Merge in all CAS substrate handlers (simplify, factor, solve,
        # list ops, matrix, limit, taylor, numeric helpers, …).
        cas_handlers = build_cas_handler_table()
        self._handlers = {**self._handlers, **cas_handlers, **runtime_handlers}

        # Pre-bind the standard MACSYMA numeric constants so ``%pi`` and
        # ``%e`` resolve to their float values rather than remaining as
        # free symbols. Runtime handlers round-trip through the VM, so
        # these bindings are picked up automatically when an expression
        # containing ``%pi`` or ``%e`` is evaluated.
        self._env["%pi"] = IRFloat(math.pi)
        self._env["%e"] = IRFloat(math.e)

        # ``Kill`` and ``Ev`` need their arguments raw — not pre-evaluated:
        # ``kill(x)`` should clear the symbol ``x``, not evaluate ``x``
        # to its current binding and then ignore the result. ``ev`` reads
        # flag *names*, which would resolve to themselves under the
        # symbolic backend but holding them costs nothing and keeps the
        # contract obvious.
        self._held_heads = super().hold_heads() | frozenset({KILL.name, EV.name})

    def hold_heads(self) -> frozenset[str]:
        return self._held_heads

    # ---- environment helpers ------------------------------------------

    def unbind(self, name: str) -> None:
        """Remove ``name`` from the binding environment.

        No-op if the name was never bound. Used by the ``Kill`` handler.
        """
        self._env.pop(name, None)

    def reset_environment(self) -> None:
        """Clear every user-introduced binding.

        Re-installs the two pre-bound entries (``True``/``False``) that
        :class:`SymbolicBackend` expects to find. Also clears the
        history — Maxima's ``kill(all)`` does both.
        """
        # Cheapest correct approach: re-run the parent ``__init__``
        # state setup. We don't want to re-install handlers (that would
        # drop our runtime overrides), so we touch only ``_env``.
        from symbolic_vm.handlers import FALSE, TRUE

        self._env.clear()
        self._env["True"] = TRUE
        self._env["False"] = FALSE
        self.history.reset()

    # ---- name lookup with history fallback ----------------------------

    def lookup(self, name: str) -> IRNode | None:
        """Look up a binding, falling back to the history table.

        First checks ``self._env`` (the normal binding store). If that
        misses, asks the :class:`History` whether the name is one of
        ``%``, ``%iN``, ``%oN``. Only then does the VM see ``None``.
        """
        env_value = super().lookup(name)
        if env_value is not None:
            return env_value
        return self.history.resolve_history_symbol(name)

    # ---- option-flag context manager ----------------------------------

    @contextmanager
    def with_numer(self) -> Iterator[None]:
        """Temporarily enable the ``numer`` flag.

        Used by ``Ev(expr, numer)``. The flag is restored on exit even
        if the inner evaluation raises.
        """
        previous = self.numer
        self.numer = True
        try:
            yield
        finally:
            self.numer = previous

    def handlers(self) -> Mapping[str, Handler]:
        return self._handlers

    # The on_unresolved policy is inherited from SymbolicBackend:
    # unbound names stay as free symbols. The history fallback above
    # only kicks in for `%`/`%iN`/`%oN`; ordinary user variables that
    # weren't assigned still reach this method and are returned as-is.
    def on_unresolved(self, symbol: IRSymbol) -> IRNode:
        return symbol
