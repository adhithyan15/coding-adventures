"""Device Simulator — Layer 6 of the accelerator computing stack.

This package simulates complete accelerator devices, assembling multiple
compute units (Layer 7) with global memory, L2 cache, and work distribution
into full devices that can launch and execute kernels.

    Layer 9:  gpu-core (one core, one instruction at a time)
        |
    Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
        |
    Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
        |
    Layer 6:  device-simulator (THIS PACKAGE)
        |
        +-- NvidiaGPU       -- many SMs + HBM + L2 + GigaThread
        +-- AmdGPU          -- CUs in Shader Engines + Infinity Cache
        +-- GoogleTPU       -- Scalar/Vector/MXU pipeline + HBM
        +-- IntelGPU        -- Xe-Cores in Xe-Slices + L2
        +-- AppleANE        -- NE cores + SRAM + DMA + unified memory

Basic usage:
    >>> from device_simulator import NvidiaGPU, KernelDescriptor
    >>> from gpu_core import limm, halt
    >>> gpu = NvidiaGPU(num_sms=4)
    >>> gpu.launch_kernel(KernelDescriptor(
    ...     name="test",
    ...     program=[limm(0, 42.0), halt()],
    ...     grid_dim=(2, 1, 1),
    ...     block_dim=(32, 1, 1),
    ... ))
    >>> traces = gpu.run(1000)
    >>> print(f"Completed in {len(traces)} cycles")
"""

from device_simulator.amd_gpu import AmdGPU, ShaderEngine
from device_simulator.apple_ane import AppleANE
from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.google_tpu import GoogleTPU
from device_simulator.intel_gpu import IntelGPU, XeSlice
from device_simulator.nvidia_gpu import NvidiaGPU
from device_simulator.protocols import (
    ANEConfig,
    AcceleratorDevice,
    AmdGPUConfig,
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    GlobalMemoryStats,
    ICILink,
    IntelGPUConfig,
    KernelDescriptor,
    MemoryTransaction,
    ShaderEngineConfig,
    TPUConfig,
    XeSliceConfig,
    default_amd_config,
    default_apple_config,
    default_intel_config,
    default_nvidia_config,
    default_tpu_config,
)
from device_simulator.work_distributor import (
    ANEScheduleReplayer,
    GPUWorkDistributor,
    ScheduleEntry,
    TileOperation,
    TPUSequencer,
)

__all__ = [
    # Devices
    "NvidiaGPU",
    "AmdGPU",
    "GoogleTPU",
    "IntelGPU",
    "AppleANE",
    # Protocols and types
    "AcceleratorDevice",
    "DeviceConfig",
    "DeviceTrace",
    "DeviceStats",
    "KernelDescriptor",
    "GlobalMemoryStats",
    "MemoryTransaction",
    # Vendor-specific configs
    "AmdGPUConfig",
    "ShaderEngineConfig",
    "IntelGPUConfig",
    "XeSliceConfig",
    "TPUConfig",
    "ICILink",
    "ANEConfig",
    # Default configs
    "default_nvidia_config",
    "default_amd_config",
    "default_tpu_config",
    "default_intel_config",
    "default_apple_config",
    # Components
    "SimpleGlobalMemory",
    "GPUWorkDistributor",
    "TPUSequencer",
    "ANEScheduleReplayer",
    "TileOperation",
    "ScheduleEntry",
    "ShaderEngine",
    "XeSlice",
]
