"""Virtual Machine — Layer 5 of the computing stack.

A general-purpose stack-based bytecode interpreter, designed like the JVM
or .NET CLR. This VM is language-agnostic: any language (Python, Ruby, or
a custom language) can compile to its bytecode instruction set.

Key exports:
    - OpCode: The instruction set enumeration (LOAD_CONST, ADD, JUMP, etc.)
    - Instruction: A single bytecode instruction (opcode + optional operand)
    - CodeObject: A compiled unit of code (instructions + constants + names)
    - VMTrace: A snapshot of one execution step (for debugging/visualization)
    - VirtualMachine: The interpreter that executes CodeObjects
    - assemble_code: Convenience function to build CodeObjects
    - VMError and subclasses: Runtime error types
"""

from virtual_machine.vm import (
    CallFrame,
    CodeObject,
    DivisionByZeroError,
    Instruction,
    InvalidOpcodeError,
    InvalidOperandError,
    OpCode,
    StackUnderflowError,
    UndefinedNameError,
    VirtualMachine,
    VMError,
    VMTrace,
    assemble_code,
)

__all__ = [
    "OpCode",
    "Instruction",
    "CodeObject",
    "VMTrace",
    "CallFrame",
    "VirtualMachine",
    "assemble_code",
    "VMError",
    "StackUnderflowError",
    "UndefinedNameError",
    "DivisionByZeroError",
    "InvalidOpcodeError",
    "InvalidOperandError",
]
