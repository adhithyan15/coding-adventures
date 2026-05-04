"""Generic IR-to-WASM compiler package.

LANG20: ``WASMCodeGenerator`` implements ``CodeGenerator[IrProgram, WasmModule]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.
"""

from ir_to_wasm_compiler.compiler import (
    FunctionSignature,
    IrToWasmCompiler,
    WasmLoweringError,
    infer_function_signatures_from_comments,
    validate_for_wasm,
)
from ir_to_wasm_compiler.generator import WASMCodeGenerator

__all__ = [
    "FunctionSignature",
    "IrToWasmCompiler",
    "WASMCodeGenerator",
    "WasmLoweringError",
    "infer_function_signatures_from_comments",
    "validate_for_wasm",
]
