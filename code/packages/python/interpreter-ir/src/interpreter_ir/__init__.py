"""interpreter-ir — dynamic bytecode IR with feedback slots for the LANG pipeline.

This package defines ``IIRInstr``, ``IIRFunction``, and ``IIRModule``:
the shared bytecode format that all interpreted languages in this repository
compile to (spec LANG01).

Any language whose compiler emits ``IIRModule`` automatically gains:

- ``vm-core``         — a generic register VM interpreter (LANG02)
- ``jit-core``        — a JIT compiler with tiered promotion (LANG03)
- ``aot-core``        — ahead-of-time compilation (LANG04)
- ``debug-integration`` — VSCode breakpoints and stepping (LANG06)
- ``lsp-integration`` — language server for editor intelligence (LANG07)
- ``repl-integration`` — interactive REPL sessions (LANG08)
- ``notebook-kernel`` — Jupyter-compatible notebook kernel (LANG09)

Quick start::

    from interpreter_ir import IIRInstr, IIRFunction, IIRModule
    from interpreter_ir import FunctionTypeStatus, serialise, deserialise

    instrs = [
        IIRInstr("add", "v0", ["a", "b"], "u8"),
        IIRInstr("ret",  None, ["v0"],    "u8"),
    ]
    fn = IIRFunction(
        name="add",
        params=[("a", "u8"), ("b", "u8")],
        return_type="u8",
        instructions=instrs,
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )
    module = IIRModule(name="example", functions=[fn], entry_point="add")

    raw = serialise(module)
    restored = deserialise(raw)
    assert restored == module
"""

from __future__ import annotations

from interpreter_ir.exception_table import CATCH_ALL, ExceptionTableEntry
from interpreter_ir.function import FunctionTypeStatus, IIRFunction
from interpreter_ir.instr import IIRInstr
from interpreter_ir.module import IIRModule
from interpreter_ir.opcodes import (
    ALL_OPS,
    ALLOCATING_OPS,
    ARITHMETIC_OPS,
    BITWISE_OPS,
    BRANCH_OPS,
    CALL_OPS,
    CMP_OPS,
    COERCION_OPS,
    CONCRETE_TYPES,
    CONTROL_OPS,
    DYNAMIC_TYPE,
    HANDLER_OPS,
    HEAP_OPS,
    IO_OPS,
    MEMORY_OPS,
    POLYMORPHIC_TYPE,
    REF_PREFIX,
    REF_SUFFIX,
    SIDE_EFFECT_OPS,
    SYSCALL_CHECKED_OPS,
    THROW_OPS,
    VALUE_OPS,
    is_ref_type,
    make_ref_type,
    unwrap_ref_type,
)
from interpreter_ir.serialise import deserialise, serialise
from interpreter_ir.slot_state import (
    MAX_POLYMORPHIC_OBSERVATIONS,
    SlotKind,
    SlotState,
)

__all__ = [
    # Core types
    "IIRInstr",
    "IIRFunction",
    "IIRModule",
    "FunctionTypeStatus",
    # VMCOND00 Phase 2 — exception table
    "ExceptionTableEntry",
    "CATCH_ALL",
    # Feedback slot state machine (LANG17)
    "SlotKind",
    "SlotState",
    "MAX_POLYMORPHIC_OBSERVATIONS",
    # Serialisation
    "serialise",
    "deserialise",
    # Opcode sets
    "ALL_OPS",
    "ALLOCATING_OPS",
    "ARITHMETIC_OPS",
    "BITWISE_OPS",
    "BRANCH_OPS",
    "CALL_OPS",
    "CMP_OPS",
    "COERCION_OPS",
    "CONTROL_OPS",
    "HEAP_OPS",
    "IO_OPS",
    "MEMORY_OPS",
    "SIDE_EFFECT_OPS",
    "HANDLER_OPS",
    "SYSCALL_CHECKED_OPS",
    "THROW_OPS",
    "VALUE_OPS",
    # Type constants and helpers
    "CONCRETE_TYPES",
    "DYNAMIC_TYPE",
    "POLYMORPHIC_TYPE",
    "REF_PREFIX",
    "REF_SUFFIX",
    "is_ref_type",
    "make_ref_type",
    "unwrap_ref_type",
]
