"""NVIDIA GPU — device simulator with GigaThread Engine.

=== NVIDIA GPU Architecture ===

The NVIDIA GPU is the most widely-used accelerator for machine learning.
Its architecture is built around Streaming Multiprocessors (SMs), each
of which can independently schedule and execute thousands of threads.

    ┌─────────────────────────────────────────────────┐
    │                  NVIDIA GPU                      │
    │                                                  │
    │  ┌────────────────────────────────────────────┐  │
    │  │        GigaThread Engine (distributor)      │  │
    │  └──────────────┬─────────────────────────────┘  │
    │                 │                                 │
    │  ┌─────┐ ┌─────┐ ┌─────┐ ... ┌─────┐           │
    │  │SM 0 │ │SM 1 │ │SM 2 │     │SM N │           │
    │  └──┬──┘ └──┬──┘ └──┬──┘     └──┬──┘           │
    │     └───────┴───────┴────────────┘               │
    │                 │                                 │
    │  ┌──────────────┴─────────────────────────────┐  │
    │  │            L2 Cache (shared)                │  │
    │  └──────────────┬─────────────────────────────┘  │
    │                 │                                 │
    │  ┌──────────────┴─────────────────────────────┐  │
    │  │          HBM3 (80 GB, 3.35 TB/s)           │  │
    │  └────────────────────────────────────────────┘  │
    └─────────────────────────────────────────────────┘

=== GigaThread Engine ===

The GigaThread Engine is the top-level work distributor. When a kernel
is launched, it:

1. Creates thread blocks from the grid dimensions
2. Assigns blocks to SMs with available resources
3. As SMs complete blocks, assigns new ones
4. Continues until all blocks are dispatched

This creates **waves** of execution:
- Wave 1: Fill all SMs to capacity
- Wave 2: As SMs finish, refill them
- ...until all blocks are done
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from cache import Cache, CacheConfig
from clock import Clock
from compute_unit import SMConfig, StreamingMultiprocessor

from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.protocols import (
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    KernelDescriptor,
    default_nvidia_config,
)
from device_simulator.work_distributor import GPUWorkDistributor

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit


class NvidiaGPU:
    """NVIDIA GPU device simulator.

    Creates multiple SMs, an L2 cache, global memory (HBM), and a
    GigaThread Engine to distribute thread blocks across SMs.

    === Usage ===

        from device_simulator import NvidiaGPU
        from gpu_core import limm, halt

        # Create with small config for testing
        gpu = NvidiaGPU(num_sms=4)

        # Allocate and copy data
        addr = gpu.malloc(1024)
        gpu.memcpy_host_to_device(addr, b'\\x00' * 1024)

        # Launch kernel
        gpu.launch_kernel(KernelDescriptor(
            name="saxpy",
            program=[limm(0, 2.0), halt()],
            grid_dim=(4, 1, 1),
            block_dim=(32, 1, 1),
        ))

        # Run to completion
        traces = gpu.run(1000)

    Args:
        config:  Full DeviceConfig (uses default_nvidia_config if None).
        num_sms: Shorthand — creates a config with this many SMs.
                 Ignored if config is provided.
    """

    def __init__(
        self,
        config: DeviceConfig | None = None,
        num_sms: int = 4,
    ) -> None:
        if config is None:
            config = DeviceConfig(
                name=f"NVIDIA GPU ({num_sms} SMs)",
                architecture="nvidia_sm",
                num_compute_units=num_sms,
                l2_cache_size=4096,
                l2_cache_latency=200,
                l2_cache_associativity=4,
                l2_cache_line_size=64,
                global_memory_size=16 * 1024 * 1024,
                global_memory_bandwidth=1000.0,
                global_memory_latency=400,
                memory_channels=4,
                host_bandwidth=64.0,
                host_latency=100,
                unified_memory=False,
                max_concurrent_kernels=128,
                work_distribution_policy="round_robin",
            )

        self._config = config
        self._clock = Clock(frequency_hz=1_500_000_000)

        # Create SMs
        sm_config = config.cu_config or SMConfig(
            max_warps=8,
            num_schedulers=2,
            shared_memory_size=4096,
            register_file_size=8192,
        )
        self._sms: list[StreamingMultiprocessor] = [
            StreamingMultiprocessor(sm_config, self._clock)
            for _ in range(config.num_compute_units)
        ]

        # L2 cache (reuse existing cache package)
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

        # Work distributor (GigaThread Engine)
        self._distributor = GPUWorkDistributor(
            compute_units=self._sms,
            policy=config.work_distribution_policy,
        )

        # Stats
        self._cycle = 0
        self._total_l2_hits = 0
        self._total_l2_misses = 0
        self._kernels_launched = 0

    # --- Identity ---

    @property
    def name(self) -> str:
        """Device name."""
        return self._config.name

    @property
    def config(self) -> DeviceConfig:
        """Full device configuration."""
        return self._config

    # --- Memory management ---

    def malloc(self, size: int) -> int:
        """Allocate device memory."""
        return self._global_memory.allocate(size)

    def free(self, address: int) -> None:
        """Free device memory."""
        self._global_memory.free(address)

    def memcpy_host_to_device(self, dst: int, data: bytes) -> int:
        """Copy from host to device. Returns cycles consumed."""
        return self._global_memory.copy_from_host(dst, data)

    def memcpy_device_to_host(
        self, src: int, size: int
    ) -> tuple[bytes, int]:
        """Copy from device to host."""
        return self._global_memory.copy_to_host(src, size)

    # --- Kernel launch ---

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        """Submit a kernel for execution via the GigaThread Engine."""
        self._distributor.submit_kernel(kernel)
        self._kernels_launched += 1

    # --- Simulation ---

    def step(self, clock_edge: ClockEdge | None = None) -> DeviceTrace:
        """Advance the entire device by one clock cycle.

        1. GigaThread assigns pending blocks to SMs with free resources
        2. Each SM steps (scheduler picks warps, engines execute)
        3. Collect traces from all SMs
        4. Build device-wide trace
        """
        self._cycle += 1

        if clock_edge is None:
            edge = self._clock.tick()
        else:
            edge = clock_edge

        # 1. Distribute pending blocks to SMs
        dist_actions = self._distributor.step()

        # 2. Step all SMs
        cu_traces = []
        total_active_warps = 0
        total_max_warps = 0

        for sm in self._sms:
            trace = sm.step(edge)
            cu_traces.append(trace)
            total_active_warps += trace.active_warps
            total_max_warps += trace.total_warps

        # 3. Compute device-level metrics
        device_occupancy = (
            total_active_warps / total_max_warps
            if total_max_warps > 0
            else 0.0
        )

        active_blocks = sum(
            1 for sm in self._sms if not sm.idle
        )

        return DeviceTrace(
            cycle=self._cycle,
            device_name=self._config.name,
            distributor_actions=tuple(dist_actions),
            pending_blocks=self._distributor.pending_count,
            active_blocks=active_blocks,
            cu_traces=tuple(cu_traces),
            l2_hits=0,
            l2_misses=0,
            memory_transactions=0,
            memory_bandwidth_used=0.0,
            total_active_warps=total_active_warps,
            device_occupancy=device_occupancy,
            flops_this_cycle=0,
        )

    def run(self, max_cycles: int = 10000) -> list[DeviceTrace]:
        """Run until all work is done or max_cycles reached."""
        traces: list[DeviceTrace] = []
        for _ in range(max_cycles):
            trace = self.step()
            traces.append(trace)
            if self.idle:
                break
        return traces

    @property
    def idle(self) -> bool:
        """True when all SMs are idle and no pending blocks remain."""
        return (
            self._distributor.pending_count == 0
            and all(sm.idle for sm in self._sms)
        )

    def reset(self) -> None:
        """Reset everything."""
        for sm in self._sms:
            sm.reset()
        self._global_memory.reset()
        self._distributor.reset()
        if self._l2 is not None:
            self._l2 = Cache(CacheConfig(
                name="L2",
                total_size=self._config.l2_cache_size,
                line_size=self._config.l2_cache_line_size,
                associativity=self._config.l2_cache_associativity,
                access_latency=self._config.l2_cache_latency,
            ))
        self._cycle = 0
        self._total_l2_hits = 0
        self._total_l2_misses = 0
        self._kernels_launched = 0

    # --- Observability ---

    @property
    def stats(self) -> DeviceStats:
        """Aggregate statistics."""
        active_cycles = sum(
            1 for _ in range(self._cycle)
        ) if self._cycle > 0 else 0

        return DeviceStats(
            total_cycles=self._cycle,
            active_cycles=active_cycles,
            idle_cycles=0,
            total_kernels_launched=self._kernels_launched,
            total_blocks_dispatched=self._distributor.total_dispatched,
            global_memory_stats=self._global_memory.stats,
        )

    @property
    def compute_units(self) -> list[ComputeUnit]:
        """Direct access to SMs."""
        return list(self._sms)

    @property
    def global_memory(self) -> SimpleGlobalMemory:
        """Access to device memory."""
        return self._global_memory
