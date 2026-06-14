"""cir-to-compiler-ir — lowers ``list[CIRInstr]`` to ``IrProgram``.

This package is the LANG21 bridge in the Tetrad compilation pipeline.
It sits between the JIT/AOT specialisation pass (which produces ``CIRInstr``
lists) and the target-independent IR backends (which consume ``IrProgram``):

.. code-block:: text

    jit-core.specialise()  →  list[CIRInstr]
        → lower_cir_to_ir_program()              ← this package
        → IrProgram
        → WASMCodeGenerator / JVMCodeGenerator / etc.

Public API
----------
``lower_cir_to_ir_program(instrs, entry_label="_start")``
    The main function.  Validates the instruction list, then runs the
    two-pass lowering algorithm.  Raises ``CIRLoweringError`` on failure.

``validate_cir_for_lowering(instrs)``
    Returns all validation errors as a list of strings without raising.
    Useful for diagnostics and IDE integrations.

``CIRLoweringError``
    Raised when lowering fails due to an unsupported or unknown CIR op.

Example
-------
::

    from codegen_core import CIRInstr
    from cir_to_compiler_ir import lower_cir_to_ir_program

    instrs = [
        CIRInstr("const_i32", "x", [40], "i32"),
        CIRInstr("const_i32", "y", [2],  "i32"),
        CIRInstr("add_i32",   "z", ["x", "y"], "i32"),
        CIRInstr("ret_void",  None, [], "void"),
    ]
    prog = lower_cir_to_ir_program(instrs)
    # prog is ready for ir_to_wasm_compiler, ir_to_jvm_class_file, etc.
"""

from cir_to_compiler_ir.errors import CIRLoweringError
from cir_to_compiler_ir.lowering import lower_cir_to_ir_program
from cir_to_compiler_ir.validator import validate_cir_for_lowering

__all__ = [
    "CIRLoweringError",
    "lower_cir_to_ir_program",
    "validate_cir_for_lowering",
]
