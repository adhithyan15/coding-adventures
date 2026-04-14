"""Reusable framework for AST-driven language type checkers.

This module builds on the lightweight ``TypeChecker`` protocol and provides
shared machinery for concrete language checkers:

- lifecycle management for ``check(ast)``
- diagnostic accumulation
- rule/kind-based hook dispatch over full AST nodes

Concrete language packages subclass ``GenericTypeChecker`` and either:

1. override ``run(ast)`` directly, or
2. register hooks per phase/kind and dispatch into them.

The framework is intentionally minimal. It does not assume a particular AST
shape beyond the subclass being able to answer "what kind of node is this?"
and "where should diagnostics point?".
"""

from __future__ import annotations

import re
from collections import defaultdict
from collections.abc import Callable
from typing import Any, Generic, TypeVar

from type_checker_protocol.protocol import TypeCheckResult, TypeChecker, TypeErrorDiagnostic

ASTT = TypeVar("ASTT")
HookFn = Callable[..., Any]

_NOT_HANDLED = object()


class GenericTypeChecker(Generic[ASTT], TypeChecker[ASTT, ASTT]):
    """Reusable base class for node-driven language type checkers.

    Subclasses implement ``run(ast)`` and can use ``dispatch()`` to route
    full AST nodes to language-specific handlers. A "handler" can be either:

    - a registered hook via ``register_hook(phase, kind, fn)``, or
    - a method named ``_{phase}_{kind}``

    where ``kind`` is the normalized node kind string returned by
    ``node_kind(node)``.
    """

    def __init__(self) -> None:
        self._errors: list[TypeErrorDiagnostic] = []
        self._hooks: dict[tuple[str, str], list[HookFn]] = defaultdict(list)

    def check(self, ast: ASTT) -> TypeCheckResult[ASTT]:
        """Run the checker and return the typed AST plus diagnostics."""
        self._errors = []
        self.run(ast)
        return TypeCheckResult(typed_ast=ast, errors=list(self._errors))

    def run(self, ast: ASTT) -> None:
        """Execute type checking for *ast*.

        Concrete language packages must override this.
        """
        raise NotImplementedError

    def register_hook(self, phase: str, kind: str, hook: HookFn) -> None:
        """Register a handler for a specific phase/node-kind pair."""
        key = (phase, self._normalize_kind(kind))
        self._hooks[key].append(hook)

    def dispatch(
        self,
        phase: str,
        node: ASTT,
        *args: Any,
        default: Any = _NOT_HANDLED,
        **kwargs: Any,
    ) -> Any:
        """Dispatch *node* to the first matching hook/handler for *phase*.

        ``dispatch`` passes the full AST node through unchanged, which keeps
        the abstraction flexible enough for richer language-specific logic.
        """
        kind = self.node_kind(node)
        normalized = self._normalize_kind(kind)

        for key in ((phase, normalized), (phase, "*")):
            for hook in self._hooks.get(key, []):
                result = hook(node, *args, **kwargs)
                if result is not _NOT_HANDLED:
                    return result

        if normalized:
            for name in (f"_{phase}_{normalized}", f"{phase}_{normalized}"):
                handler = getattr(self, name, None)
                if handler is not None:
                    return handler(node, *args, **kwargs)

        for name in (f"_{phase}", phase):
            handler = getattr(self, name, None)
            if handler is not None:
                return handler(node, *args, **kwargs)

        return default

    def node_kind(self, node: ASTT) -> str | None:
        """Return the language-specific kind string for *node*.

        Subclasses typically map this to an AST rule name or enum label.
        """
        raise NotImplementedError

    def locate(self, subject: object) -> tuple[int, int]:
        """Return ``(line, column)`` for a diagnostic target."""
        return (1, 1)

    def _error(self, message: str, subject: object) -> None:
        """Append one diagnostic for *subject*."""
        line, column = self.locate(subject)
        self._errors.append(
            TypeErrorDiagnostic(message=message, line=line, column=column)
        )

    @staticmethod
    def _normalize_kind(kind: str | None) -> str:
        """Normalize a node kind so it can participate in dispatch."""
        if not kind:
            return ""
        return re.sub(r"\W+", "_", kind).strip("_")


__all__ = ["GenericTypeChecker"]
