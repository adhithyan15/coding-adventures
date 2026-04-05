"""instance.py --- WasmInstance: a live runtime instance of a WASM module."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from wasm_types import FuncType, FunctionBody, GlobalType, WasmModule

from wasm_execution import LinearMemory, Table, WasmValue


@dataclass
class WasmInstance:
    """A live, executable instance of a WASM module."""

    module: WasmModule
    memory: LinearMemory | None
    tables: list[Table]
    globals: list[WasmValue]
    global_types: list[GlobalType]
    func_types: list[FuncType]
    func_bodies: list[FunctionBody | None]
    host_functions: list[Any | None]
    exports: dict[str, dict[str, Any]] = field(default_factory=dict)
    host: Any | None = None
