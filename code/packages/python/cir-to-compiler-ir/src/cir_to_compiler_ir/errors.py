"""CIRLoweringError — raised when CIR-to-IrProgram lowering fails.

When the lowerer encounters an unsupported instruction (``call_runtime``,
``io_in``, ``io_out``) or a completely unknown op prefix, it raises this
error to signal that the ``list[CIRInstr]`` cannot be lowered to an
``IrProgram`` in V1.

Unsupported operations are those that require backend-specific knowledge
the generic lowering pass does not possess:

  ``call_runtime``
    Generic runtime dispatch (GC allocation, dynamic method resolution,
    foreign-function calls).  These need a slow-path interpreter call or
    a target-specific ABI.  Deferred to LANG24.

  ``io_in`` / ``io_out``
    Platform I/O (stdin/stdout).  WASM uses its own import mechanism;
    JVM uses ``System.out``; GE-225 has no I/O beyond paper tape.
    Deferred to LANG23.

Any instruction with an op prefix not recognised by the dispatch table
also raises ``CIRLoweringError``.  This is a safety net — it prevents
silent data corruption when a new JIT op is added to ``jit-core`` without
a corresponding lowering rule here.

Usage
-----
::

    from cir_to_compiler_ir import CIRLoweringError, lower_cir_to_ir_program

    try:
        prog = lower_cir_to_ir_program(instrs)
    except CIRLoweringError as exc:
        print(f"Cannot lower to IrProgram: {exc}")
"""

from __future__ import annotations


class CIRLoweringError(Exception):
    """Raised when lowering a ``list[CIRInstr]`` to an ``IrProgram`` fails.

    This exception carries a human-readable message describing which
    instruction caused the failure and why.  Callers should either catch
    this error and fall back to an interpreter path, or propagate it as a
    compilation failure.

    Attributes:
        message: Plain-English description of the failure.  Also available
                 via ``str(exc)``.

    Example::

        raise CIRLoweringError(
            "unsupported op 'call_runtime' at index 3: allocate_list"
        )
    """
