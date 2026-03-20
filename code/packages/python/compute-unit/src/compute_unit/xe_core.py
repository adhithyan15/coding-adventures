"""XeCore — Intel Xe Core simulator.

=== What is an Xe Core? ===

Intel's Xe Core is a hybrid: it combines SIMD execution units (like AMD)
with hardware threads (like NVIDIA), wrapped in a unique organizational
structure. It's the building block of Intel's Arc GPUs and Data Center
GPUs (Ponte Vecchio, Flex series).

=== Architecture ===

An Xe Core contains:
- **Execution Units (EUs)**: 8-16 per Xe Core, each with its own ALU
- **Hardware threads**: 7 threads per EU for latency hiding
- **SIMD width**: SIMD8 (or SIMD16/32 on newer architectures)
- **SLM (Shared Local Memory)**: 64 KB, similar to NVIDIA's shared memory
- **Thread dispatcher**: distributes work to EU threads

    XeCore
    +---------------------------------------------------------------+
    |  Thread Dispatcher                                            |
    |  +----------------------------------------------------------+ |
    |  | Dispatches work to available EU thread slots               | |
    |  +----------------------------------------------------------+ |
    |                                                               |
    |  +------------------+ +------------------+                    |
    |  | EU 0             | | EU 1             |                    |
    |  | Thread 0: SIMD8  | | Thread 0: SIMD8  |                    |
    |  | Thread 1: SIMD8  | | Thread 1: SIMD8  |                    |
    |  | ...              | | ...              |                    |
    |  | Thread 6: SIMD8  | | Thread 6: SIMD8  |                    |
    |  | Thread Arbiter   | | Thread Arbiter   |                    |
    |  +------------------+ +------------------+                    |
    |  ... (EU 2 through EU 15)                                     |
    |                                                               |
    |  Shared Local Memory (SLM): 64 KB                             |
    |  L1 Cache: 192 KB                                             |
    +---------------------------------------------------------------+

=== How Xe Differs from NVIDIA and AMD ===

    NVIDIA SM:  4 schedulers, each manages many warps
    AMD CU:     4 SIMD units, each runs wavefronts
    Intel Xe:   8-16 EUs, each has 7 threads, each thread does SIMD8

The key insight: Intel puts the thread-level parallelism INSIDE each EU
(7 threads per EU), while NVIDIA puts it across warps (64 warps per SM)
and AMD puts it across wavefronts (40 wavefronts per CU).

Total parallelism:
    NVIDIA SM: 64 warps x 32 threads = 2048 threads
    AMD CU:    40 wavefronts x 64 lanes = 2560 lanes
    Intel Xe:  16 EUs x 7 threads x 8 SIMD = 896 lanes
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat
from gpu_core import GenericISA
from parallel_execution_engine import (
    SubsliceConfig,
    SubsliceEngine,
)

from compute_unit.protocols import (
    Architecture,
    ComputeUnitTrace,
    SchedulingPolicy,
    SharedMemory,
    WorkItem,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge
    from gpu_core import InstructionSet


# ---------------------------------------------------------------------------
# XeCoreConfig — configuration for an Intel Xe Core
# ---------------------------------------------------------------------------


@dataclass
class XeCoreConfig:
    """Configuration for an Intel Xe Core.

    Real-world Xe Core configurations:

        Parameter           | Xe-LP (iGPU) | Xe-HPG (Arc)  | Xe-HPC
        ────────────────────┼──────────────┼───────────────┼──────────
        EUs per Xe Core     | 16           | 16            | 16
        Threads per EU      | 7            | 8             | 8
        SIMD width          | 8            | 8 (or 16)     | 8/16/32
        GRF per EU          | 128          | 128           | 128
        SLM size            | 64 KB        | 64 KB         | 128 KB
        L1 cache            | 192 KB       | 192 KB        | 384 KB

    Fields:
        num_eus:               Execution Units per Xe Core.
        threads_per_eu:        Hardware threads per EU.
        simd_width:            SIMD vector width.
        grf_per_eu:            General Register File entries per EU.
        slm_size:              Shared Local Memory size in bytes.
        l1_cache_size:         L1 cache in bytes.
        instruction_cache_size: Instruction cache in bytes.
        scheduling_policy:     Thread dispatcher scheduling policy.
        float_format:          FP format for computation.
        isa:                   Instruction set architecture.
        memory_latency_cycles: Cycles for a global memory access.
    """

    num_eus: int = 16
    threads_per_eu: int = 7
    simd_width: int = 8
    grf_per_eu: int = 128

    slm_size: int = 65536
    l1_cache_size: int = 196608
    instruction_cache_size: int = 65536

    scheduling_policy: SchedulingPolicy = SchedulingPolicy.ROUND_ROBIN
    float_format: FloatFormat = FP32
    isa: InstructionSet = field(default_factory=GenericISA)
    memory_latency_cycles: int = 200


# ---------------------------------------------------------------------------
# XeCore — the main Intel Xe Core simulator
# ---------------------------------------------------------------------------


class XeCore:
    """Intel Xe Core simulator.

    Manages Execution Units (EUs) with hardware threads, SLM, and a
    thread dispatcher that distributes work across EU threads.

    === How Work Distribution Works ===

    When a work group is dispatched to an Xe Core:
    1. The thread dispatcher calculates how many EU threads are needed
    2. Each thread gets a portion of the work (SIMD8 of the total)
    3. The EU's thread arbiter round-robins among active threads
    4. SLM is shared among all threads in the work group

    === Latency Hiding in Xe ===

    With 7 threads per EU, when one thread stalls on a memory access,
    the EU arbiter switches to another ready thread on the NEXT cycle
    (zero-penalty switching, just like NVIDIA warp switching). The
    difference is granularity: Intel hides latency at the EU level
    with 7 threads, while NVIDIA hides it at the SM level with 64 warps.

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock()
        >>> xe = XeCore(XeCoreConfig(num_eus=2, threads_per_eu=2, simd_width=4), clock)
        >>> xe.dispatch(WorkItem(
        ...     work_id=0,
        ...     program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
        ...     thread_count=16,
        ... ))
        >>> traces = xe.run()
    """

    def __init__(self, config: XeCoreConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # SLM (Shared Local Memory)
        self._slm = SharedMemory(size=config.slm_size)

        # SubsliceEngine handles the EU + thread hierarchy
        self._engine = SubsliceEngine(
            SubsliceConfig(
                num_eus=config.num_eus,
                threads_per_eu=config.threads_per_eu,
                simd_width=config.simd_width,
                grf_size=config.grf_per_eu,
                slm_size=config.slm_size,
                float_format=config.float_format,
                isa=config.isa,
            ),
            clock,
        )

        self._idle_flag = True
        self._work_items: list[WorkItem] = []

    # --- Properties ---

    @property
    def name(self) -> str:
        """Compute unit name."""
        return "XeCore"

    @property
    def architecture(self) -> Architecture:
        """This is an Intel Xe Core."""
        return Architecture.INTEL_XE_CORE

    @property
    def idle(self) -> bool:
        """True if no work remains."""
        if not self._work_items and self._idle_flag:
            return True
        return self._idle_flag and self._engine.halted

    @property
    def config(self) -> XeCoreConfig:
        """The Xe Core configuration."""
        return self._config

    @property
    def slm(self) -> SharedMemory:
        """Access to Shared Local Memory."""
        return self._slm

    @property
    def engine(self) -> SubsliceEngine:
        """Access to the underlying SubsliceEngine."""
        return self._engine

    # --- Dispatch ---

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch a work group to this Xe Core.

        Loads the program into the SubsliceEngine and sets per-thread
        register values.

        Args:
            work: The WorkItem to dispatch.
        """
        self._work_items.append(work)
        self._idle_flag = False

        if work.program is not None:
            self._engine.load_program(work.program)

        # Set per-thread data across EUs
        for global_tid, regs in work.per_thread_data.items():
            # Map global thread ID to (eu, thread, lane)
            total_lanes = self._config.simd_width
            thread_total = total_lanes * self._config.threads_per_eu
            eu_id = global_tid // thread_total
            remainder = global_tid % thread_total
            thread_id = remainder // total_lanes
            lane = remainder % total_lanes

            if eu_id < self._config.num_eus:
                for reg, val in regs.items():
                    self._engine.set_eu_thread_lane_register(
                        eu_id, thread_id, lane, reg, val
                    )

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """Advance one cycle.

        Delegates to the SubsliceEngine which manages EU thread arbitration.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            A ComputeUnitTrace for this cycle.
        """
        self._cycle += 1

        engine_trace = self._engine.step(clock_edge)

        if self._engine.halted:
            self._idle_flag = True

        active = engine_trace.active_count

        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action=engine_trace.description,
            active_warps=1 if active > 0 else 0,
            total_warps=1,
            engine_traces={0: engine_trace},
            shared_memory_used=0,
            shared_memory_total=self._config.slm_size,
            register_file_used=(
                self._config.grf_per_eu * self._config.num_eus
            ),
            register_file_total=(
                self._config.grf_per_eu * self._config.num_eus
            ),
            occupancy=1.0 if active > 0 else 0.0,
        )

    def run(self, max_cycles: int = 100000) -> list[ComputeUnitTrace]:
        """Run until all work completes or max_cycles."""
        from clock import ClockEdge

        traces: list[ComputeUnitTrace] = []
        for cycle_num in range(1, max_cycles + 1):
            edge = ClockEdge(
                cycle=cycle_num, value=1, is_rising=True, is_falling=False
            )
            trace = self.step(edge)
            traces.append(trace)
            if self.idle:
                break
        return traces

    def reset(self) -> None:
        """Reset all state."""
        self._engine.reset()
        self._slm.reset()
        self._work_items.clear()
        self._idle_flag = True
        self._cycle = 0

    def __repr__(self) -> str:
        return (
            f"XeCore(eus={self._config.num_eus}, "
            f"threads_per_eu={self._config.threads_per_eu}, "
            f"idle={self.idle})"
        )
