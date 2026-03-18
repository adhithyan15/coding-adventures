"""Bytecode Compiler — Layer 4a of the computing stack.

Compiles ASTs (from the parser) into stack-machine bytecode (for the VM).

The compiler is the bridge between human-readable syntax and machine-executable
instructions. It walks the Abstract Syntax Tree produced by the parser and emits
a flat sequence of stack operations that the Virtual Machine can execute.

This package includes multiple backends that compile the same AST to different
bytecode formats:

- **BytecodeCompiler** — Targets our custom VM (the original backend).
- **JVMCompiler** — Targets the Java Virtual Machine (real JVM bytecode bytes).
- **CLRCompiler** — Targets the .NET Common Language Runtime (real CLR IL bytes).
- **WASMCompiler** — Targets WebAssembly (real WASM bytecode bytes).

Usage::

    from bytecode_compiler import BytecodeCompiler, compile_source

    # End-to-end: source code -> CodeObject
    code = compile_source("x = 1 + 2")

    # Or step by step: AST -> CodeObject
    from lang_parser import Parser
    from lexer import Lexer

    tokens = Lexer("x = 1 + 2").tokenize()
    ast = Parser(tokens).parse()
    compiler = BytecodeCompiler()
    code = compiler.compile(ast)

    # JVM backend:
    from bytecode_compiler import JVMCompiler
    jvm_code = JVMCompiler().compile(ast)

    # CLR backend:
    from bytecode_compiler import CLRCompiler
    clr_code = CLRCompiler().compile(ast)

    # WASM backend:
    from bytecode_compiler import WASMCompiler
    wasm_code = WASMCompiler().compile(ast)
"""

from bytecode_compiler.clr_compiler import CLRCodeObject, CLRCompiler
from bytecode_compiler.compiler import BytecodeCompiler, compile_source
from bytecode_compiler.jvm_compiler import JVMCodeObject, JVMCompiler
from bytecode_compiler.wasm_compiler import WASMCodeObject, WASMCompiler

__all__ = [
    "BytecodeCompiler",
    "CLRCodeObject",
    "CLRCompiler",
    "JVMCodeObject",
    "JVMCompiler",
    "WASMCodeObject",
    "WASMCompiler",
    "compile_source",
]
