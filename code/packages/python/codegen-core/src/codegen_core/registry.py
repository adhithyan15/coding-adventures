"""BackendRegistry — look up backends by name.

A ``BackendRegistry`` is a simple name-to-backend mapping.  Its purpose
is to decouple the *selection* of a backend from the *construction* of a
``CodegenPipeline``:

::

    registry = BackendRegistry()
    registry.register(Intel4004Backend())
    registry.register(WasmBackend())

    backend = registry.get("intel4004")   # → Intel4004Backend instance
    pipeline = CodegenPipeline(backend=backend, optimizer=cir_optimizer)

Why a registry?
---------------
Without a registry, every call site that selects a backend must import
and instantiate it directly, coupling the call site to a specific
backend package.  The registry moves backend selection to configuration
time and makes it easy to enumerate available backends in diagnostics or
help output.

The registry is intentionally minimal — it is a thin ``dict`` wrapper.
Multi-backend dispatch (e.g., try backend A, fall back to backend B) is
out of scope; build that on top if needed.
"""

from __future__ import annotations

from typing import Any

from codegen_core.backend import Backend


class BackendRegistry:
    """Name-to-backend mapping.

    Backends are keyed by ``Backend.name``.  Registering a second backend
    with the same name silently replaces the first.

    Examples
    --------
    >>> class MockBackend:
    ...     name = "mock"
    ...     def compile(self, ir): return b"\\x00"
    ...     def run(self, binary, args): return 0
    >>> registry = BackendRegistry()
    >>> registry.register(MockBackend())
    >>> registry.get("mock").name
    'mock'
    >>> registry.names()
    ['mock']
    """

    def __init__(self) -> None:
        self._backends: dict[str, Backend[Any]] = {}

    def register(self, backend: Backend[Any]) -> None:
        """Add ``backend`` to the registry under its ``name``.

        If a backend with the same name already exists it is replaced.

        Parameters
        ----------
        backend:
            Any object satisfying ``Backend[IR]`` for some ``IR``.
        """
        self._backends[backend.name] = backend

    def get(self, name: str) -> Backend[Any] | None:
        """Return the backend registered under ``name``, or ``None``.

        Parameters
        ----------
        name:
            The value of ``Backend.name`` used when registering.

        Returns
        -------
        Backend[Any] | None
            The registered backend, or ``None`` if not found.
        """
        return self._backends.get(name)

    def get_or_raise(self, name: str) -> Backend[Any]:
        """Return the backend registered under ``name``, or raise ``KeyError``.

        Parameters
        ----------
        name:
            The value of ``Backend.name`` used when registering.

        Returns
        -------
        Backend[Any]
            The registered backend.

        Raises
        ------
        KeyError
            If no backend with ``name`` has been registered.
        """
        try:
            return self._backends[name]
        except KeyError:
            available = ", ".join(sorted(self._backends)) or "<none>"
            raise KeyError(
                f"No backend named {name!r} in registry. "
                f"Available: {available}"
            ) from None

    def names(self) -> list[str]:
        """Return a sorted list of all registered backend names."""
        return sorted(self._backends)

    def __len__(self) -> int:
        return len(self._backends)

    def __contains__(self, name: object) -> bool:
        return name in self._backends

    def __repr__(self) -> str:
        names = ", ".join(sorted(self._backends)) or "<empty>"
        return f"BackendRegistry({names})"
