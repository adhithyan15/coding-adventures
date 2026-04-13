"""WASM Simulator — Layer 4c of the computing stack.

Minimal WebAssembly stack-based VM.
"""

from __future__ import annotations

from wasm_simulator.simulator import WasmSimulator
from wasm_simulator.state import WasmState

__all__ = ["WasmSimulator", "WasmState"]
