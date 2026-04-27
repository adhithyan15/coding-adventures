"""brainfuck_ir_compiler — Brainfuck AOT compiler frontend.

This package compiles Brainfuck source text into the general-purpose IR
defined by the ``compiler_ir`` package. It is the Brainfuck-specific
frontend of the AOT compiler pipeline.

The compiler produces two outputs:

  1. An ``IrProgram`` containing the compiled IR instructions
  2. A ``SourceMapChain`` with ``SourceToAst`` and ``AstToIr`` segments

Quick Start
-----------

::

    from brainfuck import parse_brainfuck
    from brainfuck_ir_compiler import compile_brainfuck, release_config
    from compiler_ir import print_ir

    ast = parse_brainfuck("+.")
    result = compile_brainfuck(ast, "hello.bf", release_config())

    # Print the IR text
    print(print_ir(result.program))

    # Inspect the source map
    entries = result.source_map.source_to_ast.entries

Submodules
----------

- ``build_config`` — ``BuildConfig``, ``debug_config()``, ``release_config()``
- ``compiler``     — ``compile_brainfuck()``, ``CompileResult``
"""

from brainfuck_ir_compiler.build_config import (
    BuildConfig,
    debug_config,
    release_config,
)
from brainfuck_ir_compiler.compiler import CompileResult, compile_brainfuck

__all__ = [
    "BuildConfig",
    "CompileResult",
    "compile_brainfuck",
    "debug_config",
    "release_config",
]
