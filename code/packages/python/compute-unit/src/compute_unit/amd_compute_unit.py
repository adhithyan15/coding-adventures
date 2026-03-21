"""AMDComputeUnit — AMD Compute Unit (GCN/RDNA) simulator.

=== How AMD CUs Differ from NVIDIA SMs ===

While NVIDIA and AMD GPUs look similar from the outside, their internal
organization is quite different:

    NVIDIA SM:                          AMD CU (GCN):
    ─────────                           ──────────────
    4 warp schedulers                   4 SIMD units (16-wide each)
    Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
    Total: 128 threads/cycle            Total: 64 lanes x 4 = 256 lanes/cycle

    Register file: unified              Register file: per-SIMD VGPR
    Shared memory: explicit             LDS: explicit (similar to shared mem)
    Warp scheduling: hardware           Wavefront scheduling: hardware
    Scalar unit: per-thread             Scalar unit: SHARED by wavefront

=== The Scalar Unit — AMD's Key Innovation ===

The scalar unit executes operations that are the SAME across all lanes:
- Address computation (base_addr + offset)
- Loop counters (i++)
- Branch conditions (if i < N)
- Constants (pi, epsilon, etc.)

Instead of doing this 64 times (once per lane), AMD does it ONCE in the
scalar unit and broadcasts the result. This saves power and register space.

NVIDIA doesn't have this distinction — every thread independently computes
everything, even values that are identical across the warp.

=== Architecture Diagram ===

    AMDComputeUnit (GCN-style)
    +---------------------------------------------------------------+
    |                                                               |
    |  Wavefront Scheduler                                          |
    |  +----------------------------------------------------------+ |
    |  | wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...    | |
    |  +----------------------------------------------------------+ |
    |                                                               |
    |  +------------------+ +------------------+                    |
    |  | SIMD Unit 0      | | SIMD Unit 1      |                    |
    |  | 16-wide ALU      | | 16-wide ALU      |                    |
    |  | VGPR: 256        | | VGPR: 256        |                    |
    |  +------------------+ +------------------+                    |
    |  +------------------+ +------------------+                    |
    |  | SIMD Unit 2      | | SIMD Unit 3      |                    |
    |  | 16-wide ALU      | | 16-wide ALU      |                    |
    |  +------------------+ +------------------+                    |
    |                                                               |
    |  +------------------+                                         |
    |  | Scalar Unit      |  <- executes once for all lanes         |
    |  | SGPR: 104        |  (address computation, flow control)    |
    |  +------------------+                                         |
    |                                                               |
    |  Shared Resources:                                            |
    |  +-----------------------------------------------------------+|
    |  | LDS (Local Data Share): 64 KB                              ||
    |  | L1 Vector Cache: 16 KB                                     ||
    |  | L1 Scalar Cache: 16 KB                                     ||
    |  +-----------------------------------------------------------+|
    +---------------------------------------------------------------+
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat
from gpu_core import GenericISA
from parallel_execution_engine import (
    EngineTrace,
    WavefrontConfig,
    WavefrontEngine,
)

from compute_unit.protocols import (
    Architecture,
    ComputeUnitTrace,
    SchedulingPolicy,
    SharedMemory,
    WarpState,
    WorkItem,
)
from compute_unit.streaming_multiprocessor import ResourceError

if TYPE_CHECKING:
    from clock import Clock, ClockEdge
    from gpu_core import InstructionSet


# ---------------------------------------------------------------------------
# AMDCUConfig — configuration for an AMD-style Compute Unit
# ---------------------------------------------------------------------------


@dataclass
class AMDCUConfig:
    """Configuration for an AMD-style Compute Unit.

    Real-world CU configurations:

        Parameter            | GCN (Vega)   | RDNA2 (RX 6000) | RDNA3
        ─────────────────────┼──────────────┼──────────────────┼──────
        SIMD units           | 4            | 2 (per CU)       | 2
        Wave width           | 64           | 32 (native)      | 32
        Max wavefronts       | 40           | 32               | 32
        VGPRs per SIMD       | 256          | 256              | 256
        SGPRs                | 104          | 104              | 104
        LDS size             | 64 KB        | 128 KB           | 128 KB
        L1 vector cache      | 16 KB        | 128 KB           | 128 KB

    Fields:
        num_simd_units:      Number of SIMD units (vector ALUs).
        wave_width:          Lanes per wavefront (64 for GCN, 32 for RDNA).
        max_wavefronts:      Maximum resident wavefronts.
        max_work_groups:     Maximum resident work groups.
        scheduling_policy:   How the scheduler picks wavefronts.
        vgpr_per_simd:       Vector GPRs per SIMD unit.
        sgpr_count:          Scalar GPRs (shared across all wavefronts).
        lds_size:            Local Data Share size in bytes.
        l1_vector_cache:     L1 vector cache in bytes.
        l1_scalar_cache:     L1 scalar cache in bytes.
        l1_instruction_cache: L1 instruction cache in bytes.
        float_format:        FP format for computation.
        isa:                 Instruction set architecture.
        memory_latency_cycles: Cycles for a global memory access.
    """

    num_simd_units: int = 4
    wave_width: int = 64
    max_wavefronts: int = 40
    max_work_groups: int = 16
    scheduling_policy: SchedulingPolicy = SchedulingPolicy.LRR

    # Register files
    vgpr_per_simd: int = 256
    sgpr_count: int = 104

    # Memory
    lds_size: int = 65536
    l1_vector_cache: int = 16384
    l1_scalar_cache: int = 16384
    l1_instruction_cache: int = 32768

    float_format: FloatFormat = FP32
    isa: InstructionSet = field(default_factory=GenericISA)
    memory_latency_cycles: int = 200


# ---------------------------------------------------------------------------
# WavefrontSlot — tracks one wavefront's state
# ---------------------------------------------------------------------------


@dataclass
class WavefrontSlot:
    """One wavefront in the AMD CU's scheduler.

    Similar to WarpSlot in the NVIDIA SM, but for AMD wavefronts.
    Each slot tracks the wavefront's state and which SIMD unit
    it's assigned to.

    Fields:
        wave_id:        Unique identifier for this wavefront.
        work_id:        Which WorkItem this wavefront belongs to.
        state:          Current state (READY, STALLED_MEMORY, etc.).
        simd_unit:      Which SIMD unit this wavefront is assigned to.
        engine:         The WavefrontEngine executing this wavefront.
        stall_counter:  Cycles remaining until stall resolves.
        age:            Cycles since last issued (for scheduling).
        vgprs_used:     VGPRs allocated for this wavefront.
    """

    wave_id: int
    work_id: int
    state: WarpState
    simd_unit: int
    engine: WavefrontEngine
    stall_counter: int = 0
    age: int = 0
    vgprs_used: int = 0


# ---------------------------------------------------------------------------
# AMDComputeUnit — the main CU simulator
# ---------------------------------------------------------------------------


class AMDComputeUnit:
    """AMD Compute Unit (GCN/RDNA) simulator.

    Manages wavefronts across SIMD units, with scalar unit support,
    LDS (Local Data Share), and wavefront scheduling.

    === Key Differences from StreamingMultiprocessor ===

    1. **SIMD units instead of warp schedulers**: Each SIMD unit is a
       16-wide vector ALU. A 64-wide wavefront takes 4 cycles to execute
       on a 16-wide SIMD unit (the wavefront is "over-scheduled").

    2. **Scalar unit**: Operations common to all lanes execute once on
       the scalar unit instead of per-lane. We simulate this by tracking
       scalar register state separately.

    3. **LDS instead of shared memory**: Functionally similar, but AMD's
       LDS has different banking (32 banks, 4 bytes/bank) and supports
       different access patterns.

    4. **LRR scheduling**: AMD typically uses Loose Round Robin instead
       of NVIDIA's GTO. This gives fairer distribution but potentially
       less cache locality.

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock()
        >>> cu = AMDComputeUnit(AMDCUConfig(max_wavefronts=8, wave_width=4), clock)
        >>> cu.dispatch(WorkItem(
        ...     work_id=0,
        ...     program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
        ...     thread_count=8,
        ... ))
        >>> traces = cu.run()
    """

    def __init__(self, config: AMDCUConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # LDS (Local Data Share) — AMD's shared memory
        self._lds = SharedMemory(size=config.lds_size)
        self._lds_used = 0

        # Wavefront tracking
        self._wavefront_slots: list[WavefrontSlot] = []
        self._next_wave_id = 0

        # VGPR tracking per SIMD unit
        self._vgpr_allocated: list[int] = [0] * config.num_simd_units

        # Simple round-robin index for scheduling
        self._rr_index = 0

    # --- Properties ---

    @property
    def name(self) -> str:
        """Compute unit name."""
        return "CU"

    @property
    def architecture(self) -> Architecture:
        """This is an AMD CU."""
        return Architecture.AMD_CU

    @property
    def idle(self) -> bool:
        """True if no active wavefronts remain."""
        return all(
            w.state == WarpState.COMPLETED for w in self._wavefront_slots
        ) or len(self._wavefront_slots) == 0

    @property
    def occupancy(self) -> float:
        """Current occupancy: active wavefronts / max wavefronts."""
        if self._config.max_wavefronts == 0:
            return 0.0
        active = sum(
            1 for w in self._wavefront_slots
            if w.state != WarpState.COMPLETED
        )
        return active / self._config.max_wavefronts

    @property
    def config(self) -> AMDCUConfig:
        """The CU configuration."""
        return self._config

    @property
    def lds(self) -> SharedMemory:
        """Access to the Local Data Share."""
        return self._lds

    @property
    def wavefront_slots(self) -> list[WavefrontSlot]:
        """All wavefront slots (for inspection)."""
        return self._wavefront_slots

    # --- Dispatch ---

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch a work group to this CU.

        Decomposes the work group into wavefronts and assigns them to
        SIMD units round-robin.

        Args:
            work: The WorkItem to dispatch.

        Raises:
            ResourceError: If not enough resources.
        """
        num_waves = (
            (work.thread_count + self._config.wave_width - 1)
            // self._config.wave_width
        )

        current_active = sum(
            1 for w in self._wavefront_slots
            if w.state != WarpState.COMPLETED
        )

        if current_active + num_waves > self._config.max_wavefronts:
            msg = (
                f"Not enough wavefront slots: need {num_waves}, "
                f"available {self._config.max_wavefronts - current_active}"
            )
            raise ResourceError(msg)

        smem_needed = work.shared_mem_bytes
        if self._lds_used + smem_needed > self._config.lds_size:
            msg = (
                f"Not enough LDS: need {smem_needed}, "
                f"available {self._config.lds_size - self._lds_used}"
            )
            raise ResourceError(msg)

        self._lds_used += smem_needed

        for wave_idx in range(num_waves):
            wave_id = self._next_wave_id
            self._next_wave_id += 1

            thread_start = wave_idx * self._config.wave_width
            thread_end = min(
                thread_start + self._config.wave_width,
                work.thread_count,
            )
            actual_lanes = thread_end - thread_start

            # Assign to a SIMD unit round-robin
            simd_unit = wave_idx % self._config.num_simd_units

            # Create WavefrontEngine
            engine = WavefrontEngine(
                WavefrontConfig(
                    wave_width=actual_lanes,
                    num_vgprs=min(
                        self._config.vgpr_per_simd, 256
                    ),
                    num_sgprs=self._config.sgpr_count,
                    float_format=self._config.float_format,
                    isa=self._config.isa,
                ),
                self._clock,
            )

            if work.program is not None:
                engine.load_program(work.program)

            # Set per-thread (per-lane) data
            for lane_offset in range(actual_lanes):
                global_tid = thread_start + lane_offset
                if global_tid in work.per_thread_data:
                    for reg, val in work.per_thread_data[global_tid].items():
                        engine.set_lane_register(lane_offset, reg, val)

            slot = WavefrontSlot(
                wave_id=wave_id,
                work_id=work.work_id,
                state=WarpState.READY,
                simd_unit=simd_unit,
                engine=engine,
                vgprs_used=min(self._config.vgpr_per_simd, 256),
            )
            self._wavefront_slots.append(slot)

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """One cycle: schedule wavefronts, execute on SIMD units.

        The AMD CU scheduler uses LRR (Loose Round Robin) by default:
        rotate through wavefronts, skip any that are stalled.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            A ComputeUnitTrace for this cycle.
        """
        self._cycle += 1

        # Tick stall counters
        for slot in self._wavefront_slots:
            if slot.stall_counter > 0:
                slot.stall_counter -= 1
                if slot.stall_counter == 0 and slot.state == WarpState.STALLED_MEMORY:
                    slot.state = WarpState.READY
            if slot.state not in (WarpState.COMPLETED, WarpState.RUNNING):
                slot.age += 1

        # Schedule: pick up to num_simd_units wavefronts (one per SIMD unit)
        engine_traces: dict[int, EngineTrace] = {}
        scheduler_actions: list[str] = []

        # For each SIMD unit, find a ready wavefront assigned to it
        for simd_id in range(self._config.num_simd_units):
            ready = [
                w for w in self._wavefront_slots
                if w.state == WarpState.READY and w.simd_unit == simd_id
            ]
            if not ready:
                continue

            # LRR: pick oldest ready wavefront (approximation of LRR)
            picked = max(ready, key=lambda w: w.age)
            picked.state = WarpState.RUNNING

            trace = picked.engine.step(clock_edge)
            engine_traces[picked.wave_id] = trace

            scheduler_actions.append(
                f"SIMD{simd_id}: issued wave {picked.wave_id}"
            )
            picked.age = 0

            # Update state after execution
            if picked.engine.halted:
                picked.state = WarpState.COMPLETED
            elif self._is_memory_instruction(trace):
                picked.state = WarpState.STALLED_MEMORY
                picked.stall_counter = self._config.memory_latency_cycles
            else:
                picked.state = WarpState.READY

        if not scheduler_actions:
            scheduler_actions.append("all wavefronts stalled or completed")

        active_waves = sum(
            1 for w in self._wavefront_slots
            if w.state != WarpState.COMPLETED
        )

        total_vgprs = self._config.vgpr_per_simd * self._config.num_simd_units

        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action="; ".join(scheduler_actions),
            active_warps=active_waves,
            total_warps=self._config.max_wavefronts,
            engine_traces=engine_traces,
            shared_memory_used=self._lds_used,
            shared_memory_total=self._config.lds_size,
            register_file_used=sum(self._vgpr_allocated),
            register_file_total=total_vgprs,
            occupancy=(
                active_waves / self._config.max_wavefronts
                if self._config.max_wavefronts > 0
                else 0.0
            ),
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
        self._wavefront_slots.clear()
        self._lds.reset()
        self._lds_used = 0
        self._vgpr_allocated = [0] * self._config.num_simd_units
        self._next_wave_id = 0
        self._rr_index = 0
        self._cycle = 0

    # --- Private helpers ---

    def _is_memory_instruction(self, trace: EngineTrace) -> bool:
        """Check if the executed instruction was a memory operation."""
        desc = trace.description.upper()
        return "LOAD" in desc or "STORE" in desc

    def __repr__(self) -> str:
        active = sum(
            1 for w in self._wavefront_slots
            if w.state != WarpState.COMPLETED
        )
        return (
            f"AMDComputeUnit(waves={active}/{self._config.max_wavefronts}, "
            f"occupancy={self.occupancy:.1%})"
        )
