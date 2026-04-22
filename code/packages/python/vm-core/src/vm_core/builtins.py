"""Builtin registry for vm-core.

Languages register host-provided callables here so that ``call_builtin``
instructions can invoke Python functions without any VM modification.

Usage::

    from vm_core.builtins import BuiltinRegistry
    registry = BuiltinRegistry()
    registry.register("print", lambda args: print(*args))
    registry.register("input", lambda args: input(args[0] if args else ""))

    # In the dispatch handler for "call_builtin":
    result = registry.call("print", [42])
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any


class BuiltinRegistry:
    """Maps builtin names to host-provided Python callables.

    Each callable receives a single argument: a list of resolved values.
    It should return a value (or None for void builtins).
    """

    def __init__(self) -> None:
        self._table: dict[str, Callable[[list[Any]], Any]] = {}
        self._install_standard_builtins()

    def _install_standard_builtins(self) -> None:
        """Pre-register a minimal set of standard builtins."""
        self._table["noop"] = lambda _args: None
        self._table["assert_eq"] = lambda args: (
            None if args[0] == args[1]
            else (_ for _ in ()).throw(  # type: ignore[attr-defined]
                AssertionError(f"assert_eq failed: {args[0]!r} != {args[1]!r}")
            )
        )

    def register(self, name: str, fn: Callable[[list[Any]], Any]) -> None:
        """Register a builtin callable under ``name``."""
        self._table[name] = fn

    def call(self, name: str, args: list[Any]) -> Any:
        """Invoke the builtin named ``name`` with ``args``.

        Raises KeyError if the name is not registered.
        """
        fn = self._table.get(name)
        if fn is None:
            raise KeyError(f"undefined builtin: {name!r}")
        return fn(args)

    def is_registered(self, name: str) -> bool:
        """Return True if a builtin with this name is registered."""
        return name in self._table

    def registered_names(self) -> list[str]:
        """Return all registered builtin names in insertion order."""
        return list(self._table)
