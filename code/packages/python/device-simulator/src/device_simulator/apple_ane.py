"""Apple Neural Engine — device simulator with unified memory.

=== Apple ANE Architecture ===

The Apple Neural Engine is radically different from GPUs and TPUs.
It's a fixed-function accelerator designed for neural network inference,
optimized for power efficiency over flexibility.

    ┌──────────────────────────────────────────────────┐
    │           Apple Neural Engine                     │
    │                                                   │
    │  ┌────────────────────────────────────────────┐  │
    │  │       DMA Controller (schedule replayer)    │  │
    │  └──────┬─────┬─────┬──────┬─────────────────┘  │
    │         │     │     │      │                      │
    │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐            │
    │  │Core 0│ │Core 1│ │Core 2│ │Core N│            │
    │  │ MAC  │ │ MAC  │ │ MAC  │ │ MAC  │            │
    │  │ Array│ │ Array│ │ Array│ │ Array│            │
    │  │ Act. │ │ Act. │ │ Act. │ │ Act. │            │
    │  │ Pipe │ │ Pipe │ │ Pipe │ │ Pipe │            │
    │  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘            │
    │     └────────┴────────┴────────┘                  │
    │                │                                   │
    │  ┌─────────────┴──────────────────────────────┐  │
    │  │         Shared SRAM (32 MB)                 │  │
    │  └─────────────┬──────────────────────────────┘  │
    │                │                                   │
    │  ┌─────────────┴──────────────────────────────┐  │
    │  │   Unified Memory (shared with CPU & GPU)    │  │
    │  │   No copy needed — just remap page tables   │  │
    │  └────────────────────────────────────────────┘  │
    └──────────────────────────────────────────────────┘

=== Unified Memory: The Game Changer ===

Apple's unified memory architecture means the ANE, CPU, and GPU all
share the same physical memory. When you "copy" data to the ANE, there's
no actual data movement — the system just updates page table mappings.
This eliminates the PCIe bottleneck that plagues discrete GPUs:

    Discrete GPU: Copy 8 MB over PCIe → 125 μs overhead
    Apple ANE:    Remap page tables → ~0 μs overhead

This makes the ANE competitive even for small inference tasks where a
discrete GPU would be bottlenecked by transfer time.

=== Compiler-Driven Scheduling ===

Unlike GPUs (which have hardware warp schedulers) and TPUs (which have
a sequencer), the ANE relies entirely on the CoreML compiler to generate
a fixed execution schedule. The hardware simply replays this schedule.

This means:
- No dynamic scheduling overhead (saves power)
- Predictable execution time (good for real-time applications)
- Less flexible (can only run workloads the compiler supports)
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from clock import Clock
from compute_unit import ANECoreConfig, NeuralEngineCore

from device_simulator.global_memory import SimpleGlobalMemory
from device_simulator.protocols import (
    ANEConfig,
    DeviceConfig,
    DeviceStats,
    DeviceTrace,
    KernelDescriptor,
    default_apple_config,
)
from device_simulator.work_distributor import ANEScheduleReplayer

if TYPE_CHECKING:
    from clock import ClockEdge
    from compute_unit import ComputeUnit


class AppleANE:
    """Apple Neural Engine device simulator.

    Features unified memory (zero-copy host transfers), shared SRAM,
    compiler-driven schedule replay, and DMA-based data movement.

    Args:
        config:    ANEConfig or DeviceConfig. Uses defaults if None.
        num_cores: Shorthand — number of NE cores.
    """

    def __init__(
        self,
        config: DeviceConfig | None = None,
        num_cores: int = 4,
    ) -> None:
        if config is None:
            config = ANEConfig(
                name=f"Apple ANE ({num_cores} cores)",
                architecture="apple_ane_core",
                num_compute_units=num_cores,
                l2_cache_size=0,
                l2_cache_latency=0,
                l2_cache_associativity=0,
                global_memory_size=16 * 1024 * 1024,
                global_memory_bandwidth=200.0,
                global_memory_latency=100,
                memory_channels=8,
                host_bandwidth=200.0,
                host_latency=0,
                unified_memory=True,
                max_concurrent_kernels=1,
                work_distribution_policy="scheduled",
                shared_sram_size=4 * 1024 * 1024,
                sram_bandwidth=1000.0,
                sram_latency=5,
                dma_channels=4,
                dma_bandwidth=100.0,
            )

        self._config = config
        self._clock = Clock(frequency_hz=1_000_000_000)

        # Create NE cores
        core_config = config.cu_config or ANECoreConfig()
        self._cores: list[NeuralEngineCore] = [
            NeuralEngineCore(core_config, self._clock)
            for _ in range(config.num_compute_units)
        ]

        # Global memory (unified — zero-copy)
        self._global_memory = SimpleGlobalMemory(
            capacity=config.global_memory_size,
            bandwidth=config.global_memory_bandwidth,
            latency=config.global_memory_latency,
            channels=config.memory_channels,
            host_bandwidth=config.host_bandwidth,
            host_latency=config.host_latency,
            unified=config.unified_memory,
        )

        # Schedule replayer (compiler-driven)
        dma_latency = 10
        compute_latency = 20
        if isinstance(config, ANEConfig):
            # Scale latencies based on DMA bandwidth
            dma_latency = max(1, int(1024 / config.dma_bandwidth))
            compute_latency = 20

        self._replayer = ANEScheduleReplayer(
            compute_units=self._cores,
            dma_latency=dma_latency,
            compute_latency=compute_latency,
            activate_latency=5,
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
        """Copy from host — zero-cost on unified memory!

        On Apple's unified memory, this doesn't actually copy data.
        The CPU and ANE share the same physical memory. The 'copy'
        just updates page table mappings.
        """
        return self._global_memory.copy_from_host(dst, data)

    def memcpy_device_to_host(
        self, src: int, size: int
    ) -> tuple[bytes, int]:
        """Copy to host — zero-cost on unified memory!"""
        return self._global_memory.copy_to_host(src, size)

    # --- Operation launch ---

    def launch_kernel(self, kernel: KernelDescriptor) -> None:
        """Submit an operation to the schedule replayer.

        The compiler (us) generates a complete execution schedule
        including DMA loads, compute, activation, and DMA stores.
        """
        self._replayer.submit_operation(kernel)
        self._kernels_launched += 1

    # --- Simulation ---

    def step(self, clock_edge: ClockEdge | None = None) -> DeviceTrace:
        self._cycle += 1

        if clock_edge is None:
            edge = self._clock.tick()
        else:
            edge = clock_edge

        # Replay the next step in the compiler-generated schedule
        schedule_actions = self._replayer.step()

        # Step all cores
        cu_traces = []
        for core in self._cores:
            trace = core.step(edge)
            cu_traces.append(trace)

        active_cores = sum(1 for core in self._cores if not core.idle)

        return DeviceTrace(
            cycle=self._cycle,
            device_name=self._config.name,
            distributor_actions=tuple(schedule_actions),
            pending_blocks=self._replayer.pending_count,
            active_blocks=active_cores,
            cu_traces=tuple(cu_traces),
            device_occupancy=(
                active_cores / len(self._cores)
                if self._cores
                else 0.0
            ),
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
        return self._replayer.idle

    def reset(self) -> None:
        for core in self._cores:
            core.reset()
        self._global_memory.reset()
        self._replayer.reset()
        self._cycle = 0
        self._kernels_launched = 0

    # --- Observability ---

    @property
    def stats(self) -> DeviceStats:
        return DeviceStats(
            total_cycles=self._cycle,
            total_kernels_launched=self._kernels_launched,
            total_blocks_dispatched=self._replayer.total_dispatched,
            global_memory_stats=self._global_memory.stats,
        )

    @property
    def compute_units(self) -> list[ComputeUnit]:
        return list(self._cores)

    @property
    def global_memory(self) -> SimpleGlobalMemory:
        return self._global_memory

    @property
    def is_unified_memory(self) -> bool:
        """True — Apple ANE always uses unified memory."""
        return True
