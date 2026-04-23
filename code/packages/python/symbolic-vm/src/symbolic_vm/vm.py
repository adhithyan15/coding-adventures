"""The generic symbolic :class:`VM` — a tree walker over ``symbolic_ir``.

The walker itself is small: about forty meaningful lines of code. Every
interesting evaluation decision (how to resolve names, whether to leave
unknown heads alone, what rewrite rules to try) is delegated to the
:class:`~symbolic_vm.backend.Backend`.

Evaluation algorithm (applicative order with holds and rules)::

    eval(node):
        if node is an atom:
            IRSymbol        → backend.lookup / on_unresolved
            numeric literal → return as-is
        if node is IRApply(head, args):
            if head is a held head:
                new_args = args                     # unevaluated
            else:
                new_args = [eval(a) for a in args]  # applicative
            for rule in backend.rules():
                if rule matches: return eval(rule.transform(expr))
            if backend.handlers()[head_name] exists:
                return handler(vm, expr)
            if head is a user-defined function:
                return eval(substitute(body, params → new_args))
            return backend.on_unknown_head(expr)

A "user-defined function" is a symbol bound (via :func:`handlers.define`)
to an ``IRApply(Define, (name, params, body))`` record. The VM checks
for this just before the "unknown head" fallback, so user functions
slot neatly into the same dispatch flow as built-ins.
"""

from __future__ import annotations

from symbolic_ir import DEFINE, LIST, IRApply, IRNode, IRSymbol

from symbolic_vm.backend import Backend


class VM:
    """Evaluates symbolic IR trees under a policy supplied by a Backend."""

    def __init__(self, backend: Backend) -> None:
        self.backend = backend

    # ------------------------------------------------------------------
    # Public entry points
    # ------------------------------------------------------------------

    def eval(self, node: IRNode) -> IRNode:
        """Evaluate ``node`` and return the resulting IR node."""
        if isinstance(node, IRSymbol):
            return self._eval_symbol(node)
        if isinstance(node, IRApply):
            return self._eval_apply(node)
        # Numeric and string literals pass through unchanged.
        return node

    def eval_program(self, statements: list[IRNode]) -> IRNode | None:
        """Evaluate a sequence of statements; return the last value.

        Mirrors a MACSYMA REPL: each statement is evaluated in order
        against the same backend environment, and the value of the
        program is whatever the final statement evaluated to. An
        empty program returns ``None``.
        """
        result: IRNode | None = None
        for stmt in statements:
            result = self.eval(stmt)
        return result

    # ------------------------------------------------------------------
    # Internal evaluation
    # ------------------------------------------------------------------

    def _eval_symbol(self, sym: IRSymbol) -> IRNode:
        value = self.backend.lookup(sym.name)
        if value is None:
            return self.backend.on_unresolved(sym)
        # Guard against the trivial self-loop ``x : x`` that would
        # otherwise recurse forever. Any non-trivial binding gets
        # re-evaluated so transitive bindings work (``a : b; b : 5;``
        # makes ``a`` resolve to ``5``).
        if value == sym:
            return sym
        return self.eval(value)

    def _eval_apply(self, node: IRApply) -> IRNode:
        head_name = _head_name(node.head)

        # 1. Evaluate arguments unless the head holds them.
        if head_name in self.backend.hold_heads():
            new_args = node.args
        else:
            new_args = tuple(self.eval(a) for a in node.args)
        expr = IRApply(node.head, new_args)

        # 2. Try rewrite rules (cheap syntactic rewrites first).
        for predicate, transform in self.backend.rules():
            if predicate(expr):
                return self.eval(transform(expr))

        # 3. Dispatch to a head-specific handler.
        handler = self.backend.handlers().get(head_name)
        if handler is not None:
            return handler(self, expr)

        # 4. User-defined function? Look up the head symbol; if it's
        #    bound to a ``Define`` record, inline-substitute and eval.
        if isinstance(node.head, IRSymbol):
            bound = self.backend.lookup(head_name)
            if _is_define_record(bound):
                return self._apply_user_function(bound, new_args)

        # 5. No handler, no function — fall back per backend policy.
        return self.backend.on_unknown_head(expr)

    # ------------------------------------------------------------------
    # User-defined function application
    # ------------------------------------------------------------------

    def _apply_user_function(
        self, definition: IRApply, args: tuple[IRNode, ...]
    ) -> IRNode:
        """Substitute params → args in a ``Define`` body and evaluate it.

        The binding has the shape produced by the macsyma-compiler:
        ``Define(name, List(param1, param2, ...), body)``. Substitution
        is a plain structural walk — we do NOT handle shadowing,
        because MACSYMA functions are flat (no nested ``lambda``).
        Adding proper lexical scoping later only means tracking an
        environment during substitution.
        """
        _name, params, body = definition.args
        if not (isinstance(params, IRApply) and params.head == LIST):
            raise TypeError(
                f"user function params must be a List, got {params!r}"
            )
        param_names = tuple(
            p.name for p in params.args if isinstance(p, IRSymbol)
        )
        if len(param_names) != len(args):
            raise TypeError(
                f"arity mismatch: function expects {len(param_names)} "
                f"args, got {len(args)}"
            )
        substitution = dict(zip(param_names, args, strict=True))
        return self.eval(_substitute(body, substitution))


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _head_name(head: IRNode) -> str:
    """Return the head's symbol name, or ``""`` for non-symbol heads."""
    if isinstance(head, IRSymbol):
        return head.name
    return ""


def _is_define_record(node: IRNode | None) -> bool:
    """True if ``node`` is an ``IRApply(Define, ...)`` stored binding."""
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head == DEFINE
    )


def _substitute(node: IRNode, mapping: dict[str, IRNode]) -> IRNode:
    """Replace free occurrences of names in ``node`` with values in ``mapping``.

    Walks the tree structurally. Symbols whose names match get
    swapped for the corresponding IR node; everything else passes
    through unchanged. Applies recurse into both head and args so
    substituting ``f`` into ``f(x)`` works too.
    """
    if isinstance(node, IRSymbol):
        return mapping.get(node.name, node)
    if isinstance(node, IRApply):
        new_head = _substitute(node.head, mapping)
        new_args = tuple(_substitute(a, mapping) for a in node.args)
        return IRApply(new_head, new_args)
    return node
