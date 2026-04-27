"""BackendProtocol — the interface that JIT backends must implement.

Any object that satisfies this structural protocol can be passed to
``JITCore`` as the ``backend`` argument.

Implementing a backend
----------------------
A minimal backend for a simulator:

    class MySimulatorBackend:
        name = "my-sim"

        def compile(self, cir: list[CIRInstr]) -> bytes | None:
            # Translate CIR to your binary format.
            # Return None if the function cannot be compiled.
            return encode(cir)

        def run(self, binary: bytes, args: list) -> Any:
            # Execute the compiled binary with the given arguments.
            return MySimulator(binary).run(args)

The ``BackendProtocol`` is ``runtime_checkable`` so you can use
``isinstance(obj, BackendProtocol)`` in tests and assertions.
"""

from __future__ import annotations

from typing import Any, Protocol, runtime_checkable

from jit_core.cir import CIRInstr


@runtime_checkable
class BackendProtocol(Protocol):
    """Structural protocol for jit-core backends.

    A backend translates a ``list[CIRInstr]`` into a native binary and
    provides a mechanism to execute that binary.

    Attributes
    ----------
    name:
        A short human-readable identifier, e.g. ``"intel4004"``.
        Stored in ``JITCacheEntry.backend_name`` for diagnostics.
    """

    name: str

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        """Translate ``cir`` to a native binary.

        Parameters
        ----------
        cir:
            Post-optimization ``CIRInstr`` list from the specialization pass.

        Returns
        -------
        bytes
            Opaque binary ready for ``run()``.
        None
            If this function cannot be compiled by this backend (e.g., uses
            instructions the backend doesn't support).
        """
        ...

    def run(self, binary: bytes, args: list[Any]) -> Any:
        """Execute a previously compiled binary.

        Parameters
        ----------
        binary:
            The bytes returned by ``compile()``.
        args:
            Positional arguments in the same order as the ``IIRFunction``
            parameter list.

        Returns
        -------
        Any
            The function's return value, or ``None`` for void functions.
        """
        ...
