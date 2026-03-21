"""Google TPU — device simulator with Scalar/Vector/MXU pipeline.

=== TPU Architecture ===

The TPU is fundamentally different from GPUs. Instead of thousands of
small cores executing thread programs, the TPU has:

1. **One large MXU** (Matrix Multiply Unit) — a 128×128 systolic array
   that multiplies entire matrices in hardware.
2. **A vector unit** — handles element-wise operations (activation
   functions, normalization, softmax).
3. **A scalar unit** — handles control flow, address calculation,
   and loop counters.

These three units form a **pipeline**: while the MXU processes one
matrix tile, the vector unit post-processes the previous tile, and
the scalar unit prepares the next tile.

    ┌────────────────────────────────────────────┐
    │              Google TPU                      │
    │                                              │
    │  ┌──────────────────────────────────────┐   │
    │  │        Sequencer (control unit)       │   │
    │  └────┬──────────┬──────────┬───────────┘   │
    │       │          │          │                │
    │  ┌────┴──┐  ┌────┴────┐  ┌─┴──────────┐    │
    │  │Scalar │  │ Vector  │  │    MXU      │    │
    │  │ Unit  │  │  Unit   │  │  (128×128)  │    │
    │  │       │  │(128 wide│  │  Systolic   │    │
    │  │Control│  │ elem-   │  │   Array     │    │
    │  │ flow  │  │ wise)   │  │             │    │
    │  └───────┘  └─────────┘  └─────────────┘    │
    │                                              │
    │  ┌──────────────────────────────────────┐   │
    │  │    Transpose / Permute Unit           │   │
    │  └──────────────────────────────────────┘   │
    │                                              │
    │  ┌──────────────────────────────────────┐   │
    │  │      HBM2e (32 GB, 1.2 TB/s)         │   │
    │  └──────────────────────────────────────┘   │
    └──────────────────────────────────────────────┘

=== No Thread Blocks ===

TPUs don't have threads, warps, or thread blocks. The programming model
is completely different:

    GPU: "Run this program on 65,536 threads"
    TPU: "Multiply this 1024×512 matrix by this 512×768 matrix"

The TPU sequencer tiles the large matrix operation into MXU-sized
chunks (128×128) and feeds them through the pipeline automatically.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from clock import Clock
from compute_unit import MXUConfig, MatrixMultiplyUnit

from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.protocols import (
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    KernelDescriptor,
    TPUConfig,
    default_tpu_config,
)
from device_simulator.work_distributor import TPUSequencer

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit


class GoogleTPU:
    """Google TPU device simulator.

    Features a Scalar/Vector/MXU pipeline, HBM memory, and an optional
    ICI interconnect for multi-chip communication.

    Args:
        config:   TPUConfig or DeviceConfig. Uses defaults if None.
        mxu_size: Shorthand — systolic array dimension (e.g., 128).
    """

    def __init__(
        self,
        config: DeviceConfig | None = None,
        mxu_size: int = 4,
    ) -> None:
        if config is None:
            config = TPUConfig(
                name=f"Google TPU (MXU {mxu_size}×{mxu_size})",
                architecture="google_mxu",
                num_compute_units=1,
                l2_cache_size=0,
                l2_cache_latency=0,
                l2_cache_associativity=0,
                global_memory_size=16 * 1024 * 1024,
                global_memory_bandwidth=1200.0,
                global_memory_latency=300,
                memory_channels=4,
                host_bandwidth=500.0,
                host_latency=100,
                unified_memory=False,
                max_concurrent_kernels=1,
                work_distribution_policy="sequential",
                vector_unit_width=mxu_size,
                scalar_registers=32,
                transpose_unit=True,
            )

        self._config = config
        self._clock = Clock(frequency_hz=1_000_000_000)

        # Create MXU
        mxu_config = config.cu_config or MXUConfig()
        self._mxu = MatrixMultiplyUnit(mxu_config, self._clock)

        # The sequencer orchestrates Scalar → MXU → Vector pipeline
        vec_width = (
            config.vector_unit_width
            if isinstance(config, TPUConfig)
            else mxu_size
        )
        self._sequencer = TPUSequencer(
            mxu=self._mxu,
            mxu_size=mxu_size,
            vector_width=vec_width,
            scalar_latency=5,
            mxu_latency=20,
            vector_latency=10,
        )

        # Global memory (HBM)
        self._global_memory = SimpleGlobalMemory(
            capacity=config.global_memory_size,
            bandwidth=config.global_memory_bandwidth,
            latency=config.global_memory_latency,
            channels=config.memory_channels,
            host_bandwidth=config.host_bandwidth,
            host_latency=config.host_latency,
            unified=config.unified_memory,
        )

        self._cycle = 0
        self._kernels_launched = 0

    # --- Identity ---

    @property
    def name(self) -> str:
        return self._config.name

    @property
    def config(self) -> DeviceConfig:
        return self._config

    # --- Memory management ---

    def malloc(self, size: int) -> int:
        return self._global_memory.allocate(size)

    def free(self, address: int) -> None:
        self._global_memory.free(address)

    def memcpy_host_to_device(self, dst: int, data: bytes) -> int:
        return self._global_memory.copy_from_host(dst, data)

    def memcpy_device_to_host(
        self, src: int, size: int
    ) -> tuple[bytes, int]:
        return self._global_memory.copy_to_host(src, size)

    # --- Operation launch ---

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        """Submit an operation (matmul, etc.) to the sequencer."""
        self._sequencer.submit_operation(kernel)
        self._kernels_launched += 1

    # --- Simulation ---

    def step(self, clock_edge: ClockEdge | None = None) -> DeviceTrace:
        self._cycle += 1

        if clock_edge is None:
            edge = self._clock.tick()
        else:
            edge = clock_edge

        # Advance the Scalar → MXU → Vector pipeline
        seq_actions = self._sequencer.step()

        # Also step the MXU compute unit
        cu_trace = self._mxu.step(edge)

        return DeviceTrace(
            cycle=self._cycle,
            device_name=self._config.name,
            distributor_actions=tuple(seq_actions),
            pending_blocks=self._sequencer.pending_count,
            active_blocks=0 if self._sequencer.idle else 1,
            cu_traces=(cu_trace,),
            device_occupancy=0.0 if self._sequencer.idle else 1.0,
        )

    def run(self, max_cycles: int = 10000) -> list[DeviceTrace]:
        traces: list[DeviceTrace] = []
        for _ in range(max_cycles):
            trace = self.step()
            traces.append(trace)
            if self.idle:
                break
        return traces

    @property
    def idle(self) -> bool:
        return self._sequencer.idle

    def reset(self) -> None:
        self._mxu.reset()
        self._sequencer.reset()
        self._global_memory.reset()
        self._cycle = 0
        self._kernels_launched = 0

    # --- Observability ---

    @property
    def stats(self) -> DeviceStats:
        return DeviceStats(
            total_cycles=self._cycle,
            total_kernels_launched=self._kernels_launched,
            total_blocks_dispatched=self._sequencer.total_dispatched,
            global_memory_stats=self._global_memory.stats,
        )

    @property
    def compute_units(self) -> list[ComputeUnit]:
        return [self._mxu]

    @property
    def global_memory(self) -> SimpleGlobalMemory:
        return self._global_memory
