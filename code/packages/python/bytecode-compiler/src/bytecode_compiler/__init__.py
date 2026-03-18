"""Bytecode Compiler — Layer 4a of the computing stack.

Compiles ASTs (from the parser) into stack-machine bytecode (for the VM).

The compiler is the bridge between human-readable syntax and machine-executable
instructions. It walks the Abstract Syntax Tree produced by the parser and emits
a flat sequence of stack operations that the Virtual Machine can execute.

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
"""

from bytecode_compiler.compiler import BytecodeCompiler, compile_source

__all__ = [
    "BytecodeCompiler",
    "compile_source",
]
