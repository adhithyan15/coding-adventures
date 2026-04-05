"""dispatch.py --- Central registration of all WASM instruction handlers."""

from __future__ import annotations

from virtual_machine.generic_vm import GenericVM

from wasm_execution.instructions.conversion import register_conversion
from wasm_execution.instructions.memory import register_memory
from wasm_execution.instructions.numeric_f32 import register_numeric_f32
from wasm_execution.instructions.numeric_f64 import register_numeric_f64
from wasm_execution.instructions.numeric_i32 import register_numeric_i32
from wasm_execution.instructions.numeric_i64 import register_numeric_i64
from wasm_execution.instructions.parametric import register_parametric
from wasm_execution.instructions.variable import register_variable


def register_all_instructions(vm: GenericVM) -> None:
    """Register all non-control-flow WASM instruction handlers on the VM."""
    register_numeric_i32(vm)
    register_numeric_i64(vm)
    register_numeric_f32(vm)
    register_numeric_f64(vm)
    register_conversion(vm)
    register_variable(vm)
    register_parametric(vm)
    register_memory(vm)
