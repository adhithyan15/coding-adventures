"""Generic IR-to-WASM compiler package."""

from ir_to_wasm_compiler.compiler import (
    FunctionSignature,
    IrToWasmCompiler,
    WasmLoweringError,
    infer_function_signatures_from_comments,
    validate_for_wasm,
)

__all__ = [
    "FunctionSignature",
    "IrToWasmCompiler",
    "WasmLoweringError",
    "infer_function_signatures_from_comments",
    "validate_for_wasm",
]
