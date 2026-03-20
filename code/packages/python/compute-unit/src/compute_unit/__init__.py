"""Compute Unit — Layer 7 of the accelerator computing stack.

This package implements five different compute unit architectures, showing
how different vendors organize parallel execution engines, schedulers,
shared memory, and caches into working computational building blocks.

    Layer 9:  gpu-core (one core, one instruction at a time)
        |
    Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
        |
    Layer 7:  compute-unit (THIS PACKAGE)
        |
        +-- StreamingMultiprocessor  -- NVIDIA SM
        +-- AMDComputeUnit           -- AMD CU (GCN/RDNA)
        +-- MatrixMultiplyUnit       -- Google TPU MXU
        +-- XeCore                   -- Intel Xe Core
        +-- NeuralEngineCore         -- Apple ANE Core

Basic usage:
    >>> from compute_unit import StreamingMultiprocessor, SMConfig, WorkItem
    >>> from clock import Clock
    >>> from gpu_core import limm, fmul, halt
    >>> clock = Clock(frequency_hz=1_500_000_000)
    >>> sm = StreamingMultiprocessor(SMConfig(max_warps=8), clock)
    >>> sm.dispatch(WorkItem(
    ...     work_id=0,
    ...     program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
    ...     thread_count=64,
    ... ))
    >>> traces = sm.run()
    >>> print(f"Completed in {len(traces)} cycles, occupancy: {sm.occupancy:.1%}")
"""

from compute_unit.amd_compute_unit import AMDComputeUnit, AMDCUConfig, WavefrontSlot
from compute_unit.matrix_multiply_unit import MatrixMultiplyUnit, MXUConfig
from compute_unit.neural_engine_core import ANECoreConfig, NeuralEngineCore
from compute_unit.protocols import (
    Architecture,
    ComputeUnit,
    ComputeUnitTrace,
    SchedulingPolicy,
    SharedMemory,
    WarpState,
    WorkItem,
)
from compute_unit.streaming_multiprocessor import (
    ResourceError,
    SMConfig,
    StreamingMultiprocessor,
    WarpScheduler,
    WarpSlot,
)
from compute_unit.xe_core import XeCore, XeCoreConfig

__all__ = [
    # Protocols and types
    "ComputeUnit",
    "Architecture",
    "WarpState",
    "SchedulingPolicy",
    "WorkItem",
    "ComputeUnitTrace",
    "SharedMemory",
    "ResourceError",
    # NVIDIA SM
    "StreamingMultiprocessor",
    "SMConfig",
    "WarpSlot",
    "WarpScheduler",
    # AMD CU
    "AMDComputeUnit",
    "AMDCUConfig",
    "WavefrontSlot",
    # Google TPU MXU
    "MatrixMultiplyUnit",
    "MXUConfig",
    # Intel Xe Core
    "XeCore",
    "XeCoreConfig",
    # Apple ANE Core
    "NeuralEngineCore",
    "ANECoreConfig",
]
