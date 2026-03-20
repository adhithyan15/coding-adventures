"""Intel GPU — device simulator with Xe-Slices.

=== Intel GPU Architecture (Xe-HPG / Arc) ===

Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing
a large L1 cache. This is similar to AMD's Shader Engines but at a
different granularity.

    ┌──────────────────────────────────────────────────┐
    │                Intel GPU                          │
    │  ┌────────────────────────────────────────────┐  │
    │  │     Command Streamer (distributor)          │  │
    │  └──────────────┬─────────────────────────────┘  │
    │                 │                                 │
    │  ┌──────────────┴──────────────────────────────┐ │
    │  │         Xe-Slice 0                           │ │
    │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────┐ │ │
    │  │  │XeCore 0│ │XeCore 1│ │XeCore 2│ │XeCo 3│ │ │
    │  │  │ 8 EUs  │ │ 8 EUs  │ │ 8 EUs  │ │8 EUs │ │ │
    │  │  └────────┘ └────────┘ └────────┘ └──────┘ │ │
    │  │  L1 Cache (192 KB shared across Xe-Cores)   │ │
    │  └─────────────────────────────────────────────┘ │
    │  ┌─────────────────────────────────────────────┐ │
    │  │         Xe-Slice 1 (same structure)          │ │
    │  └─────────────────────────────────────────────┘ │
    │  ... (4-8 Xe-Slices)                             │
    │                                                   │
    │  ┌──────────────────────────────────────────────┐│
    │  │         L2 Cache (16 MB shared)              ││
    │  └──────────────┬───────────────────────────────┘│
    │                 │                                 │
    │  ┌──────────────┴───────────────────────────────┐│
    │  │        GDDR6 (16 GB, 512 GB/s)               ││
    │  └──────────────────────────────────────────────┘│
    └──────────────────────────────────────────────────┘

=== Xe-Slice Hierarchy ===

The key architectural difference from NVIDIA (flat array of SMs) is the
extra grouping level. The Command Streamer assigns work to Xe-Slices
first, then each slice distributes to its Xe-Cores.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from cache import Cache, CacheConfig
from clock import Clock
from compute_unit import XeCore, XeCoreConfig

from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.protocols import (
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    IntelGPUConfig,
    KernelDescriptor,
    XeSliceConfig,
    default_intel_config,
)
from device_simulator.work_distributor import GPUWorkDistributor

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit


class XeSlice:
    """A group of Xe-Cores sharing an L1 cache.

    In real Intel hardware, a Xe-Slice contains 4 Xe-Cores that share
    a 192 KB L1 cache. The shared L1 enables cooperative data reuse
    between Xe-Cores in the same slice.
    """

    def __init__(
        self, slice_id: int, xe_cores: list[XeCore]
    ) -> None:
        self.slice_id = slice_id
        self.xe_cores = xe_cores

    @property
    def idle(self) -> bool:
        return all(core.idle for core in self.xe_cores)


class IntelGPU:
    """Intel GPU device simulator.

    Features Xe-Slice grouping, shared L1 per slice, L2 cache, and
    the Command Streamer for work distribution.

    Args:
        config:    IntelGPUConfig or DeviceConfig. Uses defaults if None.
        num_cores: Shorthand — total Xe-Cores.
    """

    def __init__(
        self,
        config: DeviceConfig | None = None,
        num_cores: int = 4,
    ) -> None:
        if config is None:
            config = DeviceConfig(
                name=f"Intel GPU ({num_cores} Xe-Cores)",
                architecture="intel_xe_core",
                num_compute_units=num_cores,
                l2_cache_size=4096,
                l2_cache_latency=180,
                l2_cache_associativity=4,
                l2_cache_line_size=64,
                global_memory_size=16 * 1024 * 1024,
                global_memory_bandwidth=512.0,
                global_memory_latency=350,
                memory_channels=4,
                host_bandwidth=32.0,
                host_latency=100,
                unified_memory=False,
                max_concurrent_kernels=16,
                work_distribution_policy="round_robin",
            )

        self._config = config
        self._clock = Clock(frequency_hz=2_100_000_000)

        # Create Xe-Cores
        core_config = config.cu_config or XeCoreConfig()
        all_cores: list[XeCore] = [
            XeCore(core_config, self._clock)
            for _ in range(config.num_compute_units)
        ]
        self._all_cores = all_cores

        # Group into Xe-Slices
        if isinstance(config, IntelGPUConfig):
            cores_per_slice = config.slice_config.xe_cores_per_slice
        else:
            cores_per_slice = max(1, config.num_compute_units // 2)

        self._xe_slices: list[XeSlice] = []
        for i in range(0, len(all_cores), cores_per_slice):
            slice_cores = all_cores[i : i + cores_per_slice]
            self._xe_slices.append(
                XeSlice(len(self._xe_slices), slice_cores)
            )

        # L2 cache
        if config.l2_cache_size > 0:
            self._l2 = Cache(CacheConfig(
                name="L2",
                total_size=config.l2_cache_size,
                line_size=config.l2_cache_line_size,
                associativity=config.l2_cache_associativity,
                access_latency=config.l2_cache_latency,
            ))
        else:
            self._l2 = None

        # Global memory
        self._global_memory = SimpleGlobalMemory(
            capacity=config.global_memory_size,
            bandwidth=config.global_memory_bandwidth,
            latency=config.global_memory_latency,
            channels=config.memory_channels,
            host_bandwidth=config.host_bandwidth,
            host_latency=config.host_latency,
            unified=config.unified_memory,
        )

        # Work distributor (Command Streamer)
        self._distributor = GPUWorkDistributor(
            compute_units=all_cores,
            policy=config.work_distribution_policy,
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

    # --- Kernel launch ---

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        self._distributor.submit_kernel(kernel)
        self._kernels_launched += 1

    # --- Simulation ---

    def step(self, clock_edge: ClockEdge | None = None) -> DeviceTrace:
        self._cycle += 1

        if clock_edge is None:
            edge = self._clock.tick()
        else:
            edge = clock_edge

        dist_actions = self._distributor.step()

        cu_traces = []
        total_active_warps = 0
        total_max_warps = 0

        for core in self._all_cores:
            trace = core.step(edge)
            cu_traces.append(trace)
            total_active_warps += trace.active_warps
            total_max_warps += trace.total_warps

        device_occupancy = (
            total_active_warps / total_max_warps
            if total_max_warps > 0
            else 0.0
        )

        active_blocks = sum(
            1 for core in self._all_cores if not core.idle
        )

        return DeviceTrace(
            cycle=self._cycle,
            device_name=self._config.name,
            distributor_actions=tuple(dist_actions),
            pending_blocks=self._distributor.pending_count,
            active_blocks=active_blocks,
            cu_traces=tuple(cu_traces),
            total_active_warps=total_active_warps,
            device_occupancy=device_occupancy,
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
        return (
            self._distributor.pending_count == 0
            and all(core.idle for core in self._all_cores)
        )

    def reset(self) -> None:
        for core in self._all_cores:
            core.reset()
        self._global_memory.reset()
        self._distributor.reset()
        self._cycle = 0
        self._kernels_launched = 0

    # --- Observability ---

    @property
    def stats(self) -> DeviceStats:
        return DeviceStats(
            total_cycles=self._cycle,
            total_kernels_launched=self._kernels_launched,
            total_blocks_dispatched=self._distributor.total_dispatched,
            global_memory_stats=self._global_memory.stats,
        )

    @property
    def compute_units(self) -> list[ComputeUnit]:
        return list(self._all_cores)

    @property
    def xe_slices(self) -> list[XeSlice]:
        """Access to Xe-Slices (Intel-specific)."""
        return self._xe_slices

    @property
    def global_memory(self) -> SimpleGlobalMemory:
        return self._global_memory
