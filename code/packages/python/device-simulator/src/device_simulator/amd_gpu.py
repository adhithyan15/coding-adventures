"""AMD GPU — device simulator with Shader Engines and Infinity Cache.

=== AMD GPU Architecture ===

AMD organizes compute units (CUs) into **Shader Engines** (SEs). This is
a mid-level hierarchy that NVIDIA doesn't have — CUs within the same SE
share a geometry processor and rasterizer (for graphics), and for compute
workloads, the Command Processor assigns entire work-groups to SEs first.

    ┌────────────────────────────────────────────────────┐
    │                    AMD GPU                          │
    │  ┌──────────────────────────────────────────────┐  │
    │  │       Command Processor (distributor)         │  │
    │  └──────────────┬───────────────────────────────┘  │
    │                 │                                   │
    │  ┌──────────────┴──────────────────┐               │
    │  │      Shader Engine 0             │               │
    │  │  ┌────┐ ┌────┐ ... ┌────┐       │               │
    │  │  │CU 0│ │CU 1│     │CU N│       │               │
    │  │  └────┘ └────┘     └────┘       │               │
    │  └─────────────────────────────────┘               │
    │  ┌──────────────────────────────────┐              │
    │  │      Shader Engine 1             │              │
    │  │  ┌────┐ ┌────┐ ... ┌────┐       │              │
    │  │  │CU..│ │CU..│     │CU..│       │              │
    │  │  └────┘ └────┘     └────┘       │              │
    │  └──────────────────────────────────┘              │
    │  ... more Shader Engines                           │
    │                                                    │
    │  ┌──────────────────────────────────────────────┐ │
    │  │     Infinity Cache (96 MB, ~50 cycle lat.)    │ │
    │  └──────────────┬───────────────────────────────┘ │
    │                 │                                  │
    │  ┌──────────────┴───────────────────────────────┐ │
    │  │           GDDR6 (24 GB, 960 GB/s)             │ │
    │  └──────────────────────────────────────────────┘ │
    └────────────────────────────────────────────────────┘

=== Infinity Cache ===

AMD's Infinity Cache is a large last-level cache (96 MB on RX 7900 XTX)
that sits between the CUs and GDDR. It's similar to an L3 cache but
much larger. It dramatically reduces the effective memory bandwidth
requirement — if data hits in the Infinity Cache, it doesn't need to
go to GDDR at all.

=== Asynchronous Compute Engines (ACEs) ===

AMD GPUs have multiple hardware queues (ACEs) that can dispatch work
simultaneously. This allows overlapping compute and copy operations,
or running multiple kernels concurrently on different CUs.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from cache import Cache, CacheConfig
from clock import Clock
from compute_unit import AMDComputeUnit, AMDCUConfig

from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.protocols import (
    AmdGPUConfig,
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    KernelDescriptor,
    ShaderEngineConfig,
    default_amd_config,
)
from device_simulator.work_distributor import GPUWorkDistributor

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit


class ShaderEngine:
    """A group of CUs that share resources.

    In a real AMD GPU, a Shader Engine shares a geometry processor,
    rasterizer, and some L1 cache. For compute workloads, it mainly
    affects how the Command Processor assigns work — blocks tend to
    be assigned to CUs within the same SE.
    """

    def __init__(
        self, engine_id: int, cus: list[AMDComputeUnit]
    ) -> None:
        self.engine_id = engine_id
        self.cus = cus

    @property
    def idle(self) -> bool:
        """True when all CUs in this SE are idle."""
        return all(cu.idle for cu in self.cus)


class AmdGPU:
    """AMD GPU device simulator.

    Features Shader Engine grouping, Infinity Cache, and multi-queue
    dispatch via ACEs.

    Args:
        config:   AmdGPUConfig or DeviceConfig. Uses defaults if None.
        num_cus:  Shorthand — creates a config with this many CUs.
    """

    def __init__(
        self,
        config: DeviceConfig | None = None,
        num_cus: int = 4,
    ) -> None:
        if config is None:
            config = DeviceConfig(
                name=f"AMD GPU ({num_cus} CUs)",
                architecture="amd_cu",
                num_compute_units=num_cus,
                l2_cache_size=4096,
                l2_cache_latency=150,
                l2_cache_associativity=4,
                l2_cache_line_size=64,
                global_memory_size=16 * 1024 * 1024,
                global_memory_bandwidth=960.0,
                global_memory_latency=350,
                memory_channels=4,
                host_bandwidth=32.0,
                host_latency=100,
                unified_memory=False,
                max_concurrent_kernels=8,
                work_distribution_policy="round_robin",
            )

        self._config = config
        self._clock = Clock(frequency_hz=1_800_000_000)

        # Create CUs
        cu_config = config.cu_config or AMDCUConfig()
        all_cus: list[AMDComputeUnit] = [
            AMDComputeUnit(cu_config, self._clock)
            for _ in range(config.num_compute_units)
        ]
        self._all_cus = all_cus

        # Group into Shader Engines
        if isinstance(config, AmdGPUConfig):
            se_size = config.se_config.cus_per_engine
        else:
            se_size = max(1, config.num_compute_units // 2)

        self._shader_engines: list[ShaderEngine] = []
        for i in range(0, len(all_cus), se_size):
            se_cus = all_cus[i : i + se_size]
            self._shader_engines.append(
                ShaderEngine(len(self._shader_engines), se_cus)
            )

        # Infinity Cache (if AMD-specific config)
        if isinstance(config, AmdGPUConfig) and config.infinity_cache_size > 0:
            ic_size = config.infinity_cache_size
            # Use power of 2 for cache
            ic_size_pow2 = 1 << (ic_size.bit_length() - 1)
            self._infinity_cache = Cache(CacheConfig(
                name="InfinityCache",
                total_size=min(ic_size_pow2, 4096),
                line_size=64,
                associativity=16,
                access_latency=config.infinity_cache_latency,
            ))
        else:
            self._infinity_cache = None

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

        # Work distributor (Command Processor)
        self._distributor = GPUWorkDistributor(
            compute_units=all_cus,
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

        for cu in self._all_cus:
            trace = cu.step(edge)
            cu_traces.append(trace)
            total_active_warps += trace.active_warps
            total_max_warps += trace.total_warps

        device_occupancy = (
            total_active_warps / total_max_warps
            if total_max_warps > 0
            else 0.0
        )

        active_blocks = sum(1 for cu in self._all_cus if not cu.idle)

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
            and all(cu.idle for cu in self._all_cus)
        )

    def reset(self) -> None:
        for cu in self._all_cus:
            cu.reset()
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
        return list(self._all_cus)

    @property
    def shader_engines(self) -> list[ShaderEngine]:
        """Access to Shader Engines (AMD-specific)."""
        return self._shader_engines

    @property
    def global_memory(self) -> SimpleGlobalMemory:
        return self._global_memory
