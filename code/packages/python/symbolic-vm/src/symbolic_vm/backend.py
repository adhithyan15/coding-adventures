"""The :class:`Backend` abstract base class.

A :class:`~symbolic_vm.vm.VM` delegates every evaluation policy decision
to a Backend. This is the seam where language-specific behavior lives.

The VM asks the backend four kinds of questions:

1. *How do I look up a name?* — :meth:`Backend.lookup` / :meth:`Backend.bind`.
2. *What if a name is unbound?* — :meth:`Backend.on_unresolved`.
3. *Are there any rewrite rules to try first?* — :meth:`Backend.rules`.
4. *How should I evaluate an* ``IRApply`` *with a given head?* —
   :meth:`Backend.handlers`.

Backends can also mark certain head names as "held", meaning the VM
will *not* evaluate the arguments of those applies before handing them
to the handler. ``Define`` is the archetypal held head: when compiling
``f(x) := x^2``, you want the body ``x^2`` stored as-is, not evaluated
right now.

The two reference backends in this package —
:class:`~symbolic_vm.backends.StrictBackend` and
:class:`~symbolic_vm.backends.SymbolicBackend` — share almost all of
their handler table; only the unresolved-name policy and the set of
rewrite rules differ meaningfully between them.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import Callable, Iterable, Mapping
from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRNode, IRSymbol

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


# A rewrite rule is a predicate + transform pair. The predicate looks
# at the already-argument-evaluated ``IRApply``; if it returns True,
# the transform produces the rewritten ``IRNode``, which the VM then
# recursively evaluates. Rules are tried in order before head handlers.
RulePredicate = Callable[[IRApply], bool]
RuleTransform = Callable[[IRApply], IRNode]
Rule = tuple[RulePredicate, RuleTransform]

# A handler is the per-head evaluator. It receives the VM itself — so
# it can call ``vm.eval(...)`` on subexpressions when it produces new
# IR — and the current ``IRApply`` (whose args have already been
# evaluated, unless the head was held).
Handler = Callable[["VM", IRApply], IRNode]


class Backend(ABC):
    """Policy object that the VM consults for every evaluation decision."""

    # ------------------------------------------------------------------
    # Name binding
    # ------------------------------------------------------------------

    @abstractmethod
    def lookup(self, name: str) -> IRNode | None:
        """Return the current binding for ``name`` or ``None`` if unset."""

    @abstractmethod
    def bind(self, name: str, value: IRNode) -> None:
        """Install or update a binding for ``name``."""

    def unbind(self, name: str) -> None:  # noqa: B027
        """Remove any binding for ``name``.

        Called by the ``Block`` handler when a local variable exits
        scope and the name was unbound before the block started.  The
        default is a no-op so minimal custom backends don't have to
        implement it; real backends backed by a dict should override.
        """

    # ------------------------------------------------------------------
    # Evaluation policy
    # ------------------------------------------------------------------

    @abstractmethod
    def on_unresolved(self, symbol: IRSymbol) -> IRNode:
        """Decide what to return when a symbol has no binding.

        Strict backends raise; symbolic backends return the symbol
        unchanged (so undefined names act as free variables).
        """

    def on_unknown_head(self, expr: IRApply) -> IRNode:
        """Decide what to return when no handler exists for the head.

        The default is to leave the expression unchanged. Strict
        backends override this to raise.
        """
        return expr

    def rules(self) -> Iterable[Rule]:
        """Yield rewrite rules to try before dispatching to a handler."""
        return ()

    def handlers(self) -> Mapping[str, Handler]:
        """Return the head-name → handler table."""
        return {}

    def hold_heads(self) -> frozenset[str]:
        """Head names whose arguments the VM should NOT evaluate first.

        ``Define`` stores a function body without evaluating it;
        ``If`` chooses one of its branches; ``Assign`` consumes the
        lhs symbolically. Handlers for these heads take responsibility
        for evaluating the parts they actually want evaluated.
        """
        return frozenset()
