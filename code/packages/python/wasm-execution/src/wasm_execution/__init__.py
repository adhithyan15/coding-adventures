"""wasm-execution --- WebAssembly 1.0 execution engine.

This package provides the complete WASM execution stack: typed values,
linear memory, tables, instruction handlers, and the execution engine.
"""

__version__ = "0.1.0"

from wasm_execution.const_expr import evaluate_const_expr
from wasm_execution.engine import WasmExecutionEngine, WasmExecutionLimits
from wasm_execution.host_interface import HostFunction, HostInterface, TrapError
from wasm_execution.linear_memory import LinearMemory
from wasm_execution.table import Table
from wasm_execution.values import (
    WasmValue,
    as_f32,
    as_f64,
    as_i32,
    as_i64,
    default_value,
    f32,
    f64,
    i32,
    i64,
)

__all__ = [
    "WasmExecutionEngine",
    "WasmExecutionLimits",
    "LinearMemory",
    "Table",
    "TrapError",
    "HostFunction",
    "HostInterface",
    "WasmValue",
    "i32",
    "i64",
    "f32",
    "f64",
    "default_value",
    "as_i32",
    "as_i64",
    "as_f32",
    "as_f64",
    "evaluate_const_expr",
]
