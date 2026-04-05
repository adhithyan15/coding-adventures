"""Virtual Machine — Layer 5 of the computing stack.

A general-purpose stack-based bytecode interpreter, designed like the JVM
or .NET CLR. This VM is language-agnostic: any language (Python, Ruby, or
a custom language) can compile to its bytecode instruction set.

This package provides two VM implementations:

1. **VirtualMachine** — The original VM with a hardcoded instruction set.
   Great for learning and for the basic expression language.

2. **GenericVM** — A pluggable VM where languages register their own opcodes
   via ``register_opcode()``. Use this for language-specific interpreters
   like Starlark or Python.

Key exports:
    - OpCode: The instruction set enumeration (LOAD_CONST, ADD, JUMP, etc.)
    - Instruction: A single bytecode instruction (opcode + optional operand)
    - CodeObject: A compiled unit of code (instructions + constants + names)
    - VMTrace: A snapshot of one execution step (for debugging/visualization)
    - VirtualMachine: The original interpreter that executes CodeObjects
    - GenericVM: The pluggable interpreter with opcode registration
    - BuiltinFunction: A built-in function callable from bytecode
    - assemble_code: Convenience function to build CodeObjects
    - VMError and subclasses: Runtime error types
"""

from virtual_machine.generic_vm import (
    BuiltinFunction,
    GenericVM,
    MaxRecursionError,
    OpcodeHandler,
    TypedVMValue,
    VMTypeError,
)
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
    "GenericVM",
    "BuiltinFunction",
    "OpcodeHandler",
    "assemble_code",
    "VMError",
    "VMTypeError",
    "MaxRecursionError",
    "StackUnderflowError",
    "UndefinedNameError",
    "DivisionByZeroError",
    "InvalidOpcodeError",
    "InvalidOperandError",
    "TypedVMValue",
]
