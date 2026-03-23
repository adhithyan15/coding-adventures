"""Bytecode Compiler — Layer 4a of the computing stack.

Compiles ASTs (from the parser) into stack-machine bytecode (for the VM).

The compiler is the bridge between human-readable syntax and machine-executable
instructions. It walks the Abstract Syntax Tree produced by the parser and emits
a flat sequence of stack operations that the Virtual Machine can execute.

This package includes multiple backends and a pluggable compiler framework:

**Pluggable Framework:**

- **GenericCompiler** — A pluggable compiler where languages register their own
  AST rule handlers via ``register_rule()``. Use this for grammar-driven parsers.
- **CompilerScope** — Tracks local variables within function scopes.

**Fixed Backends:**

- **BytecodeCompiler** — Targets our custom VM (the original backend).
- **JVMCompiler** — Targets the Java Virtual Machine (real JVM bytecode bytes).
- **CLRCompiler** — Targets the .NET Common Language Runtime (real CLR IL bytes).
- **WASMCompiler** — Targets WebAssembly (real WASM bytecode bytes).

Usage::

    from bytecode_compiler import GenericCompiler

    compiler = GenericCompiler()
    compiler.register_rule("assign_stmt", compile_assign)
    compiler.register_rule("arith", compile_arith)
    code = compiler.compile(ast)
"""

from bytecode_compiler.clr_compiler import CLRCodeObject, CLRCompiler
from bytecode_compiler.compiler import BytecodeCompiler, compile_source
from bytecode_compiler.generic_compiler import (
    CompilerError,
    CompilerScope,
    GenericCompiler,
    UnhandledRuleError,
)
from bytecode_compiler.jvm_compiler import JVMCodeObject, JVMCompiler
from bytecode_compiler.wasm_compiler import WASMCodeObject, WASMCompiler

__all__ = [
    "BytecodeCompiler",
    "CLRCodeObject",
    "CLRCompiler",
    "CompilerError",
    "CompilerScope",
    "GenericCompiler",
    "JVMCodeObject",
    "JVMCompiler",
    "UnhandledRuleError",
    "WASMCodeObject",
    "WASMCompiler",
    "compile_source",
]
