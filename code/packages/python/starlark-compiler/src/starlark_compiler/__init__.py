"""Starlark Compiler — Compiles Starlark ASTs into bytecode.

This package provides a Starlark-specific compiler built on the
``GenericCompiler`` framework. It registers handlers for all 55 Starlark
grammar rules, translating the AST produced by ``starlark_parser`` into
bytecode that the Starlark VM can execute.

Key exports:
    - compile_starlark: One-call source → bytecode compilation.
    - create_starlark_compiler: Factory that returns a configured GenericCompiler.
    - Op: The Starlark opcode enumeration.
"""

from starlark_compiler.compiler import compile_starlark, create_starlark_compiler
from starlark_compiler.opcodes import (
    AUGMENTED_ASSIGN_MAP,
    BINARY_OP_MAP,
    COMPARE_OP_MAP,
    Op,
)

__all__ = [
    "Op",
    "BINARY_OP_MAP",
    "COMPARE_OP_MAP",
    "AUGMENTED_ASSIGN_MAP",
    "compile_starlark",
    "create_starlark_compiler",
]
