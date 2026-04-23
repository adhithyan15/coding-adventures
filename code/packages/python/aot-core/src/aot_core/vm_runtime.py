"""VmRuntime — pre-compiled vm-runtime library for linking into AOT binaries.

When a program uses dynamic features (runtime polymorphism, closures, string
operations, dynamic dispatch) that cannot be compiled to static native code,
``aot-core`` falls back to including those functions in the **IIR table
section** of the ``.aot`` binary.  At execution time, the vm-runtime library
interprets those functions on demand.

What is a vm-runtime?
---------------------
The vm-runtime is a compiled, linkable form of ``vm-core`` — the same
interpreter dispatch loop that jit-core uses, but pre-compiled to the target
architecture as a static library that can be embedded in the ``.aot`` binary.

This package does **not** produce the vm-runtime library itself (that is
the responsibility of ``vm-core``'s build system).  Instead, ``VmRuntime``
wraps a pre-compiled byte payload and provides:

1.  ``serialise_iir_table(fns)`` — serialise a list of ``IIRFunction`` objects
    to bytes for the IIR table section of the ``.aot`` binary.

2.  ``deserialise_iir_table(data)`` — reverse: parse IIR table bytes back into
    plain ``dict`` records (for inspection and testing).

Serialisation format
--------------------
The IIR table is JSON-encoded.  Each function is a JSON object:

::

    {
        "name": "str",
        "params": [["name", "type"], ...],
        "instructions": [
            {"op": "add", "dest": "r0", "srcs": [1, "x"],
             "type_hint": "any", "deopt_anchor": null},
            ...
        ],
        "type_status": 3
    }

JSON is chosen because it is self-describing and human-readable, which aids
debugging on embedded targets.  A more compact binary encoding can be added
later without changing the public API.

Prebuilt paths (future)
-----------------------
In a production build system, pre-compiled vm-runtime libraries would live at::

    vm-runtime/prebuilt/
        vm_runtime_intel4004.a
        vm_runtime_riscv32.a
        vm_runtime_wasm32.a

For now, ``VmRuntime`` is backend-neutral: the ``library_bytes`` can hold any
opaque byte payload.
"""

from __future__ import annotations

import json

from interpreter_ir import IIRFunction, IIRInstr


class VmRuntime:
    """Wrapper for a pre-compiled vm-runtime library.

    Parameters
    ----------
    library_bytes:
        Pre-compiled vm-runtime static library bytes.  May be empty (``b""``)
        for simulator targets that run the IIR table via Python ``vm-core``.
    """

    def __init__(self, library_bytes: bytes = b"") -> None:
        self.library_bytes = library_bytes

    @property
    def is_empty(self) -> bool:
        """True if no pre-compiled library was provided."""
        return len(self.library_bytes) == 0

    # ------------------------------------------------------------------
    # IIR table serialisation
    # ------------------------------------------------------------------

    def serialise_iir_table(self, fns: list[IIRFunction]) -> bytes:
        """Serialise a list of ``IIRFunction`` objects to IIR table bytes.

        The result is suitable for embedding in the ``.aot`` snapshot's
        IIR table section via ``snapshot.write(iir_table=...)``.

        Parameters
        ----------
        fns:
            Functions that could not be fully compiled and must be interpreted
            by the vm-runtime at run time.

        Returns
        -------
        bytes
            UTF-8-encoded JSON payload.
        """
        records = [_serialise_fn(fn) for fn in fns]
        return json.dumps(records, separators=(",", ":")).encode()

    def deserialise_iir_table(self, data: bytes) -> list[dict]:
        """Parse IIR table bytes back to plain dicts (for inspection/testing).

        Parameters
        ----------
        data:
            Bytes produced by ``serialise_iir_table()``.

        Returns
        -------
        list[dict]
            One dict per function, with the same structure as the JSON objects
            written by ``serialise_iir_table()``.
        """
        return json.loads(data.decode())


# ---------------------------------------------------------------------------
# Serialisation helpers
# ---------------------------------------------------------------------------

def _serialise_instr(instr: IIRInstr) -> dict:
    return {
        "op": instr.op,
        "dest": instr.dest,
        "srcs": list(instr.srcs),
        "type_hint": instr.type_hint,
        "deopt_anchor": instr.deopt_anchor,
    }


def _serialise_fn(fn: IIRFunction) -> dict:
    return {
        "name": fn.name,
        "params": list(fn.params),
        "instructions": [_serialise_instr(i) for i in fn.instructions],
        "type_status": fn.type_status.value,
    }
