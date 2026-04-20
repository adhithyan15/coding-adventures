"""algol-wasm-compiler — Package the first ALGOL 60 compiler subset as WebAssembly

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

from algol_wasm_compiler.compiler import (
    AlgolWasmCompiler,
    AlgolWasmError,
    AlgolWasmResult,
    compile_source,
    pack_source,
    write_wasm_file,
)

__version__ = "0.1.0"

__all__ = [
    "AlgolWasmCompiler",
    "AlgolWasmError",
    "AlgolWasmResult",
    "compile_source",
    "pack_source",
    "write_wasm_file",
]
