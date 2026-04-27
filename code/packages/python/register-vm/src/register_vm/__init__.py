"""Register-based virtual machine — public API.

This package implements a standalone register-based bytecode interpreter
inspired by V8's Ignition engine.  It is intentionally independent of the
stack-based ``virtual-machine`` package; everything needed to compile,
assemble, and execute register bytecode lives here.

Architecture overview
---------------------
Unlike a stack machine (where operands are pushed/popped from a value stack),
a register machine uses a fixed file of named registers.  V8 Ignition adds an
*accumulator* — a single hidden register that is the implicit source and
destination for most operations.  This design:

* Reduces instruction size: most opcodes only need one explicit operand
  (the register to operate with).
* Maps well to hardware CPUs (which also have a dedicated accumulator or
  flags register).
* Simplifies the JIT: the accumulator's type is always known after any
  monomorphic inline cache hit.

Quick start
-----------
::

    from register_vm import CodeObject, RegisterInstruction, Opcode, execute

    # Compute 3 + 4 and return the result.
    code = CodeObject(
        instructions=[
            RegisterInstruction(Opcode.LDA_SMI, [3]),   # acc = 3
            RegisterInstruction(Opcode.STAR, [0]),       # r0 = acc
            RegisterInstruction(Opcode.LDA_SMI, [4]),   # acc = 4
            RegisterInstruction(Opcode.ADD, [0]),        # acc = acc + r0
            RegisterInstruction(Opcode.RETURN),
        ],
        constants=[],
        names=[],
        register_count=1,
        feedback_slot_count=0,
    )
    result = execute(code)
    assert result.return_value == 7

Tracing
-------
::

    result, trace = execute_with_trace(code)
    for step in trace:
        print(f"ip={step.ip:2d}  {Opcode(step.instruction.opcode).name:<30s}"
              f"  acc: {step.acc_before!r:10} → {step.acc_after!r}")

Feedback vectors
----------------
Every call frame maintains a ``FeedbackSlot`` list.  Binary arithmetic
operations update the slot to track what types they see (numbers, strings,
etc.).  As more distinct type combinations appear, the slot progresses:

    Uninitialized → Monomorphic → Polymorphic → Megamorphic

A JIT optimizer would use this data to specialize the hot path; here it
is recorded for educational purposes.
"""

from register_vm.generic_vm import (
    GenericRegisterVM,
    GenericTrace,
    GenericVMError,
    RegisterFrame,
)
from register_vm.opcodes import Opcode
from register_vm.types import (
    UNDEFINED,
    CallFrame,
    CodeObject,
    Context,
    FeedbackSlot,
    RegisterInstruction,
    SlotMegamorphic,
    SlotMonomorphic,
    SlotPolymorphic,
    SlotUninitialized,
    TraceStep,
    VMError,
    VMFunction,
    VMObject,
    VMResult,
    VMValue,
)
from register_vm.vm import RegisterVM, execute, execute_with_trace

__all__ = [
    # Core types
    "CodeObject",
    "RegisterInstruction",
    "VMResult",
    "VMError",
    "TraceStep",
    "VMValue",
    "VMObject",
    "VMFunction",
    "CallFrame",
    "Context",
    "UNDEFINED",
    # Feedback
    "FeedbackSlot",
    "SlotUninitialized",
    "SlotMonomorphic",
    "SlotPolymorphic",
    "SlotMegamorphic",
    # Opcode enum
    "Opcode",
    # VM
    "RegisterVM",
    # Generic pluggable VM
    "GenericRegisterVM",
    "GenericTrace",
    "GenericVMError",
    "RegisterFrame",
    # Convenience functions
    "execute",
    "execute_with_trace",
]
