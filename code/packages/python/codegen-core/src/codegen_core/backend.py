"""Backend protocols — generic over the IR type.

A *backend* translates a typed IR into a native binary and provides a
mechanism to execute that binary.  The key design decision is that
``codegen-core`` defines the backend interface *generically* — the same
structural protocol works for any IR type:

- ``Backend[list[CIRInstr]]`` — the JIT/AOT path (intel4004, wasm, jvm, …)
- ``Backend[IrProgram]`` — the compiled-language path (nib-wasm, bf-wasm, …)
- ``Backend[Any]`` — escape hatch for backends with unusual IR shapes

Why a Protocol?
---------------
Python's structural subtyping (``Protocol``) means a backend author writes
a plain class with the right ``name``, ``compile``, and ``run`` methods —
no inheritance required.  This keeps backends independent of
``codegen-core``'s version; only the method signatures must match.

Both ``BackendProtocol`` (the old JIT-specific name, kept for backwards
compatibility) and ``Backend`` (the new generic alias) are exported from
this module.  New code should use ``Backend[IR]``; existing callers of
``BackendProtocol`` continue to work.

CIR-backend type alias
-----------------------
``CIRBackend`` is a convenience alias for ``Backend[list[CIRInstr]]``.
``isinstance(obj, CIRBackend)`` works because ``CIRBackend`` is
``runtime_checkable``.
"""

from __future__ import annotations

from typing import Any, Protocol, TypeVar, runtime_checkable

# TypeVar for the IR type flowing through the pipeline.
IR = TypeVar("IR")


@runtime_checkable
class Backend(Protocol[IR]):
    """Universal backend protocol — generic over IR type.

    Backends translate a typed IR value into a native binary and provide
    a mechanism to execute that binary.

    Type parameters
    ---------------
    IR
        The IR type this backend accepts.  For the JIT/AOT path this is
        ``list[CIRInstr]``.  For the compiled-language path it would be
        ``IrProgram``.

    Attributes
    ----------
    name:
        Short human-readable identifier, e.g. ``"intel4004"`` or
        ``"wasm"``.  Stored in ``CodegenResult.backend_name`` for
        diagnostics.
    """

    name: str

    def compile(self, ir: IR) -> bytes | None:
        """Translate ``ir`` to a native binary.

        Parameters
        ----------
        ir:
            The typed IR produced by the upstream specialisation or
            compilation pass.

        Returns
        -------
        bytes
            Opaque binary ready for ``run()``.
        None
            If this backend cannot compile the given IR (e.g., it uses
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
            Positional arguments in calling-convention order.

        Returns
        -------
        Any
            The function's return value, or ``None`` for void functions.
        """
        ...


# ---------------------------------------------------------------------------
# Backwards-compatible alias — jit-core's BackendProtocol re-exports this
# ---------------------------------------------------------------------------

#: Alias for ``Backend[list[CIRInstr]]`` — the JIT/AOT concrete instantiation.
#: Kept alongside ``Backend`` so existing code that imports ``BackendProtocol``
#: from ``jit_core.backend`` continues to work without modification.
BackendProtocol = Backend


# ---------------------------------------------------------------------------
# Convenience type alias
# ---------------------------------------------------------------------------

#: Concrete backend type for the JIT / AOT path.
#: ``isinstance(obj, CIRBackend)`` is True for any object with the right
#: structural shape because ``Backend`` is ``runtime_checkable``.
CIRBackend = Backend
