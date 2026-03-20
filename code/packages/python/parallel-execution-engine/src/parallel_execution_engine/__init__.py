"""Parallel Execution Engine — Layer 8 of the accelerator computing stack.

This package implements five different parallel execution models, showing
how different accelerator architectures (GPU, TPU, NPU) organize parallel
computation. Each engine takes many processing elements and orchestrates
them to execute in parallel.

    Layer 9:  gpu-core (one core, one instruction at a time)
        │
    Layer 8:  parallel-execution-engine (THIS PACKAGE)
        │
        ├── WarpEngine      — SIMT (NVIDIA/ARM Mali)
        ├── WavefrontEngine — SIMD (AMD GCN/RDNA)
        ├── SystolicArray   — Dataflow (Google TPU)
        ├── MACArrayEngine  — Scheduled MAC (Apple ANE/NPU)
        └── SubsliceEngine  — Hybrid SIMD (Intel Xe)

Basic usage:
    >>> from parallel_execution_engine import WarpEngine, WarpConfig
    >>> from clock import Clock
    >>> from gpu_core import limm, fmul, halt
    >>> clock = Clock()
    >>> engine = WarpEngine(WarpConfig(warp_width=4), clock)
    >>> engine.load_program([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()])
    >>> traces = engine.run()
    >>> engine.threads[0].core.registers.read_float(2)
    6.0
"""

from parallel_execution_engine.mac_array_engine import (
    ActivationFunction,
    MACArrayConfig,
    MACArrayEngine,
    MACOperation,
    MACScheduleEntry,
)
from parallel_execution_engine.protocols import (
    DataflowInfo,
    DivergenceInfo,
    EngineTrace,
    ExecutionModel,
    ParallelExecutionEngine,
)
from parallel_execution_engine.subslice_engine import (
    ExecutionUnit,
    SubsliceConfig,
    SubsliceEngine,
)
from parallel_execution_engine.systolic_array import (
    SystolicArray,
    SystolicConfig,
    SystolicPE,
)
from parallel_execution_engine.warp_engine import (
    DivergenceStackEntry,
    ThreadContext,
    WarpConfig,
    WarpEngine,
)
from parallel_execution_engine.wavefront_engine import (
    ScalarRegisterFile,
    VectorRegisterFile,
    WavefrontConfig,
    WavefrontEngine,
)

__all__ = [
    # Protocols and types
    "ParallelExecutionEngine",
    "ExecutionModel",
    "EngineTrace",
    "DivergenceInfo",
    "DataflowInfo",
    # SIMT (NVIDIA/ARM Mali)
    "WarpEngine",
    "WarpConfig",
    "ThreadContext",
    "DivergenceStackEntry",
    # SIMD (AMD)
    "WavefrontEngine",
    "WavefrontConfig",
    "VectorRegisterFile",
    "ScalarRegisterFile",
    # Systolic (Google TPU)
    "SystolicArray",
    "SystolicConfig",
    "SystolicPE",
    # Scheduled MAC (NPU)
    "MACArrayEngine",
    "MACArrayConfig",
    "MACScheduleEntry",
    "MACOperation",
    "ActivationFunction",
    # Intel Xe
    "SubsliceEngine",
    "SubsliceConfig",
    "ExecutionUnit",
]
