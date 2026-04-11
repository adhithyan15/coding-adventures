"""wasm-runtime --- Complete WebAssembly 1.0 runtime."""

__version__ = "0.1.0"

from wasm_runtime.instance import WasmInstance
from wasm_runtime.runtime import WasmRuntime
from wasm_runtime.wasi_stub import (
    ProcExitError,
    SystemClock,
    SystemRandom,
    WasiClock,
    WasiConfig,
    WasiRandom,
    WasiStub,
)

__all__ = [
    "WasmRuntime",
    "WasmInstance",
    "WasiStub",
    "WasiConfig",
    "WasiClock",
    "WasiRandom",
    "SystemClock",
    "SystemRandom",
    "ProcExitError",
]
