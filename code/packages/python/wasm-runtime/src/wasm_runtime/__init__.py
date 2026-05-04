"""wasm-runtime --- Complete WebAssembly 1.0 runtime."""

__version__ = "0.1.0"

from wasm_execution import WasmExecutionLimits

from wasm_runtime.instance import WasmInstance
from wasm_runtime.runtime import WasmRuntime
from wasm_runtime.wasi_host import (
    ProcExitError,
    SystemClock,
    SystemRandom,
    WasiClock,
    WasiConfig,
    WasiHost,
    WasiRandom,
)

__all__ = [
    "WasmRuntime",
    "WasmExecutionLimits",
    "WasmInstance",
    "WasiHost",
    "WasiConfig",
    "WasiClock",
    "WasiRandom",
    "SystemClock",
    "SystemRandom",
    "ProcExitError",
]
