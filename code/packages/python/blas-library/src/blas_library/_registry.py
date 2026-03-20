"""Backend Registry — find and select BLAS backends.

=== What is the Registry? ===

The registry is a central catalog of available BLAS backends. It provides
three modes of selection:

    1. EXPLICIT:    registry.get("cuda")     -- give me CUDA specifically
    2. AUTO-DETECT: registry.get_best()      -- give me the best available
    3. CUSTOM:      registry.register(...)   -- add my own backend

=== Auto-Detection Priority ===

When you ask for "the best available backend," the registry tries each
backend in priority order and returns the first one that successfully
initializes:

    cuda > metal > vulkan > opencl > webgpu > opengl > cpu

CUDA is first because it's the most optimized for ML (and most GPUs are
NVIDIA in data centers). CPU is always last — it's the universal fallback
that works everywhere.

=== How It Works Internally ===

The registry stores *classes* (not instances). When you call ``get("cuda")``,
it instantiates ``CudaBlas()`` on the spot. This is because GPU backends
allocate device resources in ``__init__``, and we don't want to waste GPU
memory on backends that aren't being used.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._protocol import BlasBackend


class BackendRegistry:
    """Backend registry — find and select BLAS backends.

    ================================================================
    BACKEND REGISTRY -- FIND AND SELECT BLAS BACKENDS
    ================================================================

    The registry keeps track of which backends are available and
    helps the caller pick one. Three modes of selection:

    1. EXPLICIT:    registry.get("cuda")
    2. AUTO-DETECT: registry.get_best()
    3. CUSTOM:      registry.register("my_backend", MyBlas)

    Auto-detection priority (customizable):
        cuda > metal > vulkan > opencl > webgpu > opengl > cpu

    CUDA is first because it's the most optimized for ML.
    Metal is second because Apple silicon has unified memory.
    CPU is always last -- it's the universal fallback.
    ================================================================
    """

    # The default auto-detection order. CUDA first (ML standard),
    # CPU last (universal fallback).
    _default_priority = [
        "cuda",
        "metal",
        "vulkan",
        "opencl",
        "webgpu",
        "opengl",
        "cpu",
    ]

    def __init__(self) -> None:
        """Create an empty registry with default priority order."""
        self._backends: dict[str, type[BlasBackend]] = {}
        self._priority: list[str] = list(self._default_priority)

    def register(self, name: str, backend_class: type[BlasBackend]) -> None:
        """Register a backend class by name.

        The class is stored but NOT instantiated yet. Instantiation happens
        when ``get()`` or ``get_best()`` is called.

        Args:
            name: Backend identifier (e.g., "cuda", "cpu").
            backend_class: The backend class to register.
        """
        self._backends[name] = backend_class

    def get(self, name: str) -> BlasBackend:
        """Get a specific backend by name, instantiating it on demand.

        Args:
            name: Backend identifier.

        Returns:
            An instantiated backend.

        Raises:
            RuntimeError: If the backend name is not registered.
        """
        if name not in self._backends:
            available = ", ".join(sorted(self._backends.keys()))
            raise RuntimeError(
                f"Backend '{name}' not registered. Available: {available}"
            )
        return self._backends[name]()

    def get_best(self) -> BlasBackend:
        """Try each backend in priority order, return the first that works.

        Each backend is instantiated inside a try/except. If initialization
        fails (e.g., no GPU available), we skip to the next one. CPU always
        works, so this never fails (as long as CPU is registered).

        Returns:
            The highest-priority backend that successfully initializes.

        Raises:
            RuntimeError: If no backend could be initialized.
        """
        for name in self._priority:
            if name in self._backends:
                try:
                    return self._backends[name]()
                except Exception:  # noqa: BLE001
                    # This backend failed to initialize — try the next one.
                    # Common reasons: no GPU driver, wrong platform, etc.
                    continue

        raise RuntimeError(
            "No BLAS backend could be initialized. "
            f"Tried: {[n for n in self._priority if n in self._backends]}"
        )

    def list_available(self) -> list[str]:
        """List names of all registered backends.

        Returns:
            A list of registered backend names.
        """
        return list(self._backends.keys())

    def set_priority(self, priority: list[str]) -> None:
        """Change the auto-detection priority order.

        Args:
            priority: New priority list (first = highest priority).
        """
        self._priority = list(priority)


# =========================================================================
# Global registry instance — shared across the whole application
# =========================================================================

# This is the single global registry. It's populated by __init__.py when
# the package is imported. Users can also register custom backends here.
global_registry = BackendRegistry()
