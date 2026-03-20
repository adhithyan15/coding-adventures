"""StreamingMultiprocessor — NVIDIA SM simulator.

=== What is a Streaming Multiprocessor? ===

The SM is the heart of NVIDIA's GPU architecture. Every NVIDIA GPU — from
the GeForce in your laptop to the H100 in a data center — is built from
SMs. Each SM is a self-contained compute unit that can independently
execute work without coordination with other SMs.

An SM contains:
- **Warp schedulers** (4 on modern GPUs) that pick ready warps to execute
- **WarpEngines** (one per scheduler) that execute 32-thread warps
- **Register file** (256 KB, 65536 registers) partitioned among warps
- **Shared memory** (up to 228 KB) for inter-thread communication
- **L1 cache** (often shares capacity with shared memory)

=== The Key Innovation: Latency Hiding ===

CPUs hide latency with deep pipelines, out-of-order execution, and branch
prediction — complex hardware that's expensive in transistors and power.

GPUs take the opposite approach: have MANY warps, and when one stalls,
switch to another. A single SM can have 48-64 warps resident. When warp 0
stalls on a memory access (~400 cycles), the scheduler instantly switches
to warp 1. By the time it has cycled through enough warps, warp 0's data
has arrived.

    CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
    GPU strategy:  Have MANY threads, switch instantly to hide latency

This is why GPUs have massive register files (256 KB vs 1-2 KB for a CPU
core) — each of those 64 warps needs its own registers, and switching
between warps must be FREE (zero-cycle context switch). The registers are
partitioned at dispatch time, so switching is just changing which register
addresses the ALU reads from.

=== Architecture Diagram ===

    StreamingMultiprocessor
    +---------------------------------------------------------------+
    |                                                               |
    |  Warp Scheduler 0        Warp Scheduler 1                     |
    |  +------------------+   +------------------+                  |
    |  | w0: READY        |   | w1: STALLED      |                  |
    |  | w4: READY        |   | w5: READY        |                  |
    |  | w8: COMPLETED    |   | w9: RUNNING      |                  |
    |  +--------+---------+   +--------+---------+                  |
    |           |                      |                            |
    |           v                      v                            |
    |  +------------------+   +------------------+                  |
    |  | WarpEngine 0     |   | WarpEngine 1     |                  |
    |  | (32 threads)     |   | (32 threads)     |                  |
    |  +------------------+   +------------------+                  |
    |                                                               |
    |  Shared Resources:                                            |
    |  +-----------------------------------------------------------+|
    |  | Register File: 256 KB (65,536 x 32-bit registers)         ||
    |  | Shared Memory: 96 KB (configurable split with L1 cache)   ||
    |  +-----------------------------------------------------------+|
    +---------------------------------------------------------------+
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat
from gpu_core import GenericISA
from parallel_execution_engine import EngineTrace, WarpConfig, WarpEngine

from compute_unit.protocols import (
    Architecture,
    ComputeUnitTrace,
    SchedulingPolicy,
    SharedMemory,
    WarpState,
    WorkItem,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge
    from gpu_core import InstructionSet


# ---------------------------------------------------------------------------
# SMConfig — all tunable parameters for an NVIDIA-style SM
# ---------------------------------------------------------------------------


@dataclass
class SMConfig:
    """Configuration for an NVIDIA-style Streaming Multiprocessor.

    Real-world SM configurations (for reference):

        Parameter             | Volta (V100) | Ampere (A100) | Hopper (H100)
        ──────────────────────┼──────────────┼───────────────┼──────────────
        Warp schedulers       | 4            | 4             | 4
        Max warps per SM      | 64           | 64            | 64
        Max threads per SM    | 2048         | 2048          | 2048
        CUDA cores (FP32)    | 64           | 64            | 128
        Register file         | 256 KB       | 256 KB        | 256 KB
        Shared memory         | 96 KB        | 164 KB        | 228 KB
        L1 cache              | combined w/ shared mem

    Our default configuration models a Volta-class SM with reduced sizes
    for faster simulation. The scheduling logic and occupancy calculations
    match real hardware behavior.

    Fields:
        num_schedulers:         Number of warp schedulers (typically 4).
        warp_width:             Threads per warp (always 32 for NVIDIA).
        max_warps:              Maximum resident warps on this SM.
        max_threads:            max_warps x warp_width.
        max_blocks:             Maximum resident thread blocks.
        scheduling_policy:      How the scheduler picks warps (GTO, etc.).
        register_file_size:     Total 32-bit registers available.
        max_registers_per_thread: Max registers a single thread can use.
        shared_memory_size:     Shared memory in bytes.
        l1_cache_size:          L1 cache in bytes.
        instruction_cache_size: Instruction cache in bytes.
        float_format:           FP format for computation.
        isa:                    Instruction set architecture.
        memory_latency_cycles:  Cycles for a global memory access (stall duration).
        barrier_enabled:        Whether __syncthreads() is supported.
    """

    num_schedulers: int = 4
    warp_width: int = 32
    max_warps: int = 48
    max_threads: int = 1536
    max_blocks: int = 16
    scheduling_policy: SchedulingPolicy = SchedulingPolicy.GTO

    # Register file
    register_file_size: int = 65536
    max_registers_per_thread: int = 255

    # Memory
    shared_memory_size: int = 98304
    l1_cache_size: int = 32768
    instruction_cache_size: int = 131072

    # Float format
    float_format: FloatFormat = FP32

    # ISA
    isa: InstructionSet = field(default_factory=GenericISA)

    # Stall simulation
    memory_latency_cycles: int = 200
    barrier_enabled: bool = True


# ---------------------------------------------------------------------------
# WarpSlot — tracks one warp's state in the scheduler
# ---------------------------------------------------------------------------


@dataclass
class WarpSlot:
    """One slot in the warp scheduler's table.

    Each WarpSlot tracks the state of one warp — whether it's ready to
    execute, stalled waiting for memory, completed, etc. The scheduler
    scans these slots to find ready warps.

    === Warp Lifecycle ===

        1. dispatch() creates a WarpSlot in READY state
        2. Scheduler picks it -> RUNNING
        3. After execution:
           - If LOAD/STORE: transition to STALLED_MEMORY for N cycles
           - If HALT: transition to COMPLETED
           - Otherwise: back to READY
        4. After stall countdown expires: back to READY

    Fields:
        warp_id:        Unique identifier for this warp.
        work_id:        Which WorkItem this warp belongs to.
        state:          Current state (READY, STALLED_MEMORY, etc.).
        engine:         The WarpEngine executing this warp's threads.
        stall_counter:  Cycles remaining until stall resolves (0 = not stalled).
        age:            How many cycles since this warp was last issued.
                        Used by OLDEST_FIRST and GTO scheduling policies.
        registers_used: How many registers this warp occupies.
    """

    warp_id: int
    work_id: int
    state: WarpState
    engine: WarpEngine
    stall_counter: int = 0
    age: int = 0
    registers_used: int = 0


# ---------------------------------------------------------------------------
# WarpScheduler — picks which warp to issue each cycle
# ---------------------------------------------------------------------------


class WarpScheduler:
    """Warp scheduler that implements multiple scheduling policies.

    === How Warp Scheduling Works ===

    On each clock cycle, the scheduler:
    1. Scans all warp slots assigned to it
    2. Decrements stall counters for stalled warps
    3. Transitions warps whose stalls have resolved to READY
    4. Picks one READY warp according to the scheduling policy
    5. Returns that warp for execution

    === Scheduling Policies ===

    ROUND_ROBIN:
        Simply rotates through warps: 0, 1, 2, ..., wrap around.
        Skips non-READY warps. Fair but doesn't optimize for locality.

    GTO (Greedy-Then-Oldest):
        Keeps issuing from the same warp until it stalls, then picks
        the oldest ready warp. This improves cache locality because
        the same warp's instructions tend to access nearby memory.

        Timeline example:
            Cycle 1: Issue warp 3 (GTO stays with warp 3)
            Cycle 2: Issue warp 3 (still ready, keep going)
            Cycle 3: Warp 3 stalls (memory access)
            Cycle 4: Switch to warp 7 (oldest ready warp)
            Cycle 5: Issue warp 7 ...

    The scheduler maintains state (like last_issued for GTO) to
    implement these policies efficiently.
    """

    def __init__(
        self,
        scheduler_id: int,
        policy: SchedulingPolicy,
    ) -> None:
        self.scheduler_id = scheduler_id
        self.policy = policy
        self._warps: list[WarpSlot] = []
        self._rr_index = 0
        self._last_issued: int | None = None

    @property
    def warps(self) -> list[WarpSlot]:
        """The warp slots managed by this scheduler."""
        return self._warps

    def add_warp(self, slot: WarpSlot) -> None:
        """Add a warp to this scheduler's management."""
        self._warps.append(slot)

    def tick_stalls(self) -> None:
        """Decrement stall counters and transition stalled warps to READY.

        Called once per cycle before scheduling. Any warp whose stall
        counter reaches 0 transitions back to READY.
        """
        for warp in self._warps:
            if warp.stall_counter > 0:
                warp.stall_counter -= 1
                if warp.stall_counter == 0 and warp.state in (
                    WarpState.STALLED_MEMORY,
                    WarpState.STALLED_DEPENDENCY,
                ):
                    warp.state = WarpState.READY

            # Age all non-completed warps (for OLDEST_FIRST / GTO)
            if warp.state not in (WarpState.COMPLETED, WarpState.RUNNING):
                warp.age += 1

    def pick_warp(self) -> WarpSlot | None:
        """Select a ready warp according to the scheduling policy.

        Returns:
            The selected WarpSlot, or None if no warps are ready.
        """
        ready = [w for w in self._warps if w.state == WarpState.READY]
        if not ready:
            return None

        match self.policy:
            case SchedulingPolicy.ROUND_ROBIN:
                return self._pick_round_robin(ready)
            case SchedulingPolicy.GTO:
                return self._pick_gto(ready)
            case SchedulingPolicy.LRR:
                return self._pick_lrr(ready)
            case SchedulingPolicy.OLDEST_FIRST:
                return self._pick_oldest_first(ready)
            case SchedulingPolicy.GREEDY:
                return self._pick_oldest_first(ready)
            case _:
                return ready[0]

    def _pick_round_robin(self, ready: list[WarpSlot]) -> WarpSlot:
        """Round-robin: rotate through warps in order."""
        # Find the next ready warp starting from the current index
        all_ids = [w.warp_id for w in self._warps]
        for i in range(len(all_ids)):
            idx = (self._rr_index + i) % len(all_ids)
            target_id = all_ids[idx]
            for w in ready:
                if w.warp_id == target_id:
                    self._rr_index = (idx + 1) % len(all_ids)
                    return w
        # Fallback
        return ready[0]

    def _pick_gto(self, ready: list[WarpSlot]) -> WarpSlot:
        """GTO: keep issuing same warp until it stalls, then oldest."""
        # Try to continue with the last issued warp
        if self._last_issued is not None:
            for w in ready:
                if w.warp_id == self._last_issued:
                    return w
        # Last issued warp is not ready — pick the oldest ready warp
        return self._pick_oldest_first(ready)

    def _pick_lrr(self, ready: list[WarpSlot]) -> WarpSlot:
        """LRR (Loose Round Robin): round-robin but skip stalled warps."""
        return self._pick_round_robin(ready)

    def _pick_oldest_first(self, ready: list[WarpSlot]) -> WarpSlot:
        """Oldest first: pick the warp that has been waiting longest."""
        return max(ready, key=lambda w: w.age)

    def mark_issued(self, warp_id: int) -> None:
        """Record that a warp was just issued (for GTO policy)."""
        self._last_issued = warp_id
        for w in self._warps:
            if w.warp_id == warp_id:
                w.age = 0
                break

    def reset(self) -> None:
        """Clear all warps from this scheduler."""
        self._warps.clear()
        self._rr_index = 0
        self._last_issued = None


# ---------------------------------------------------------------------------
# ResourceError — raised when dispatch fails due to resource limits
# ---------------------------------------------------------------------------


class ResourceError(Exception):
    """Raised when a compute unit cannot accommodate a work item.

    This happens when the SM doesn't have enough registers, shared memory,
    or warp slots to fit the requested thread block. In real CUDA, this
    would manifest as a launch failure or reduced occupancy.
    """


# ---------------------------------------------------------------------------
# StreamingMultiprocessor — the main SM simulator
# ---------------------------------------------------------------------------


class StreamingMultiprocessor:
    """NVIDIA Streaming Multiprocessor simulator.

    Manages multiple warps executing thread blocks, with a configurable
    warp scheduler, shared memory, and register file partitioning.

    === Usage Pattern ===

        1. Create SM with config and clock
        2. Dispatch one or more WorkItems (thread blocks)
        3. Call step() or run() to simulate execution
        4. Read traces to understand what happened

    === How dispatch() Works ===

    When a thread block is dispatched to the SM:

        1. Check resources: enough registers? shared memory? warp slots?
        2. Decompose the block into warps (every 32 threads = 1 warp)
        3. Allocate registers for each warp
        4. Reserve shared memory for the block
        5. Create WarpEngine instances for each warp
        6. Add warp slots to the schedulers (round-robin distribution)

    === How step() Works ===

    On each clock cycle:

        1. Tick stall counters (memory latency countdown)
        2. Each scheduler picks one ready warp (using scheduling policy)
        3. Execute picked warps on their WarpEngines
        4. Check for memory instructions -> stall the warp
        5. Check for HALT -> mark warp as completed
        6. Build and return a ComputeUnitTrace

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock(frequency_hz=1_500_000_000)
        >>> sm = StreamingMultiprocessor(SMConfig(max_warps=8), clock)
        >>> sm.dispatch(WorkItem(
        ...     work_id=0,
        ...     program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
        ...     thread_count=64,  # 2 warps
        ... ))
        >>> traces = sm.run()
        >>> print(f"Completed in {len(traces)} cycles")
    """

    def __init__(self, config: SMConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0

        # Shared memory for the SM
        self._shared_memory = SharedMemory(size=config.shared_memory_size)
        self._shared_memory_used = 0

        # Register file tracking (total registers available, allocated)
        self._registers_allocated = 0

        # Warp schedulers — one per scheduler slot
        self._schedulers = [
            WarpScheduler(i, config.scheduling_policy)
            for i in range(config.num_schedulers)
        ]

        # Track all active warp slots for occupancy and idle checks
        self._all_warp_slots: list[WarpSlot] = []
        self._next_warp_id = 0

        # Track dispatched work items
        self._active_blocks: list[int] = []

    # --- Properties ---

    @property
    def name(self) -> str:
        """Compute unit name."""
        return "SM"

    @property
    def architecture(self) -> Architecture:
        """This is an NVIDIA SM."""
        return Architecture.NVIDIA_SM

    @property
    def idle(self) -> bool:
        """True if no active warps remain."""
        return all(
            w.state == WarpState.COMPLETED for w in self._all_warp_slots
        ) or len(self._all_warp_slots) == 0

    @property
    def occupancy(self) -> float:
        """Current occupancy: active (non-completed) warps / max warps.

        Occupancy is the key performance metric for GPU kernels. Low
        occupancy means the SM can't hide memory latency because there
        aren't enough warps to switch between when one stalls.

        What limits occupancy:
        1. Register pressure: too many registers per thread
        2. Shared memory: too much shared memory per block
        3. Block size: too many warps per block
        """
        if self._config.max_warps == 0:
            return 0.0
        active = sum(
            1
            for w in self._all_warp_slots
            if w.state != WarpState.COMPLETED
        )
        return active / self._config.max_warps

    @property
    def config(self) -> SMConfig:
        """The SM configuration."""
        return self._config

    @property
    def shared_memory(self) -> SharedMemory:
        """Access to the shared memory instance."""
        return self._shared_memory

    @property
    def warp_slots(self) -> list[WarpSlot]:
        """All warp slots (for inspection)."""
        return self._all_warp_slots

    # --- Occupancy calculation ---

    def compute_occupancy(
        self,
        registers_per_thread: int,
        shared_mem_per_block: int,
        threads_per_block: int,
    ) -> float:
        """Calculate theoretical occupancy for a kernel launch configuration.

        This is the STATIC occupancy calculation — how full the SM could
        theoretically be, given the resource requirements of a kernel.
        It's separate from the DYNAMIC occupancy (self.occupancy) which
        tracks how many warps are actually resident right now.

        === How Occupancy is Limited ===

        Occupancy is limited by the tightest constraint among:

        1. Register pressure:
           Each warp needs registers_per_thread * 32 registers.
           Total warps = register_file_size / regs_per_warp.

        2. Shared memory:
           Each block needs shared_mem_per_block bytes.
           Max blocks = shared_memory_size / shared_mem_per_block.
           Max warps = max_blocks * warps_per_block.

        3. Hardware limit:
           The SM simply can't hold more than max_warps warps.

        Example:
            64 registers/thread, 48 KB shared memory, 256 threads/block:
            - Regs: 64 * 32 = 2048 regs/warp. 65536/2048 = 32 warps max.
            - Smem: 98304/49152 = 2 blocks. 2 * 8 warps = 16 warps max.
            - HW: 48 warps max.
            - Occupancy = min(32, 16, 48) / 48 = 33.3%

        Args:
            registers_per_thread: Registers used by each thread.
            shared_mem_per_block: Shared memory bytes per block.
            threads_per_block:    Threads per block.

        Returns:
            Float from 0.0 to 1.0 representing theoretical occupancy.
        """
        warp_w = self._config.warp_width
        warps_per_block = (threads_per_block + warp_w - 1) // warp_w

        # Limit 1: register file
        regs_per_warp = registers_per_thread * self._config.warp_width
        if regs_per_warp > 0:
            max_warps_by_regs = self._config.register_file_size // regs_per_warp
        else:
            max_warps_by_regs = self._config.max_warps

        # Limit 2: shared memory
        if shared_mem_per_block > 0:
            max_blocks_by_smem = self._config.shared_memory_size // shared_mem_per_block
            max_warps_by_smem = max_blocks_by_smem * warps_per_block
        else:
            max_warps_by_smem = self._config.max_warps

        # Limit 3: hardware limit
        max_warps_by_hw = self._config.max_warps

        # Actual occupancy is limited by the tightest constraint
        active_warps = min(max_warps_by_regs, max_warps_by_smem, max_warps_by_hw)
        return min(active_warps / self._config.max_warps, 1.0)

    # --- Dispatch ---

    def dispatch(self, work: WorkItem) -> None:
        """Dispatch a thread block to this SM.

        Decomposes the thread block into warps, allocates registers and
        shared memory, creates WarpEngine instances, and adds warp slots
        to the schedulers.

        Args:
            work: The WorkItem to dispatch.

        Raises:
            ResourceError: If not enough resources for this work item.
        """
        # Calculate resource requirements
        num_warps = (
            (work.thread_count + self._config.warp_width - 1)
            // self._config.warp_width
        )
        regs_needed = (
            work.registers_per_thread * self._config.warp_width * num_warps
        )
        smem_needed = work.shared_mem_bytes

        # Check resource availability
        current_active = sum(
            1 for w in self._all_warp_slots
            if w.state != WarpState.COMPLETED
        )

        if current_active + num_warps > self._config.max_warps:
            msg = (
                f"Not enough warp slots: need {num_warps}, "
                f"available {self._config.max_warps - current_active}"
            )
            raise ResourceError(msg)

        if self._registers_allocated + regs_needed > self._config.register_file_size:
            avail_regs = (
                self._config.register_file_size
                - self._registers_allocated
            )
            msg = (
                f"Not enough registers: need {regs_needed}, "
                f"available {avail_regs}"
            )
            raise ResourceError(msg)

        if self._shared_memory_used + smem_needed > self._config.shared_memory_size:
            avail_smem = (
                self._config.shared_memory_size
                - self._shared_memory_used
            )
            msg = (
                f"Not enough shared memory: need {smem_needed}, "
                f"available {avail_smem}"
            )
            raise ResourceError(msg)

        # Allocate resources
        self._registers_allocated += regs_needed
        self._shared_memory_used += smem_needed
        self._active_blocks.append(work.work_id)

        # Create warps and distribute across schedulers
        for warp_idx in range(num_warps):
            warp_id = self._next_warp_id
            self._next_warp_id += 1

            # Determine thread range for this warp
            thread_start = warp_idx * self._config.warp_width
            thread_end = min(
                thread_start + self._config.warp_width,
                work.thread_count,
            )
            actual_threads = thread_end - thread_start

            # Create a WarpEngine for this warp
            engine = WarpEngine(
                WarpConfig(
                    warp_width=actual_threads,
                    num_registers=work.registers_per_thread,
                    float_format=self._config.float_format,
                    isa=self._config.isa,
                ),
                self._clock,
            )

            # Load program if provided
            if work.program is not None:
                engine.load_program(work.program)

            # Set per-thread data if provided
            for t_offset in range(actual_threads):
                global_tid = thread_start + t_offset
                if global_tid in work.per_thread_data:
                    for reg, val in work.per_thread_data[global_tid].items():
                        engine.set_thread_register(t_offset, reg, val)

            # Create the warp slot
            slot = WarpSlot(
                warp_id=warp_id,
                work_id=work.work_id,
                state=WarpState.READY,
                engine=engine,
                registers_used=(
                    work.registers_per_thread * actual_threads
                ),
            )
            self._all_warp_slots.append(slot)

            # Distribute to schedulers round-robin
            sched_idx = warp_idx % self._config.num_schedulers
            self._schedulers[sched_idx].add_warp(slot)

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> ComputeUnitTrace:
        """One cycle: schedulers pick warps, engines execute, stalls update.

        === Step-by-Step ===

        1. Tick stall counters on all schedulers. Warps whose memory
           latency countdown has expired transition back to READY.

        2. Each scheduler independently picks one ready warp using the
           configured scheduling policy (GTO, ROUND_ROBIN, etc.).

        3. Execute the picked warps on their WarpEngines. Each engine
           advances one instruction across all 32 threads.

        4. Check execution results:
           - If the instruction was LOAD or STORE, stall the warp for
             memory_latency_cycles.
           - If the instruction was HALT, mark the warp as COMPLETED.
           - Otherwise, transition back to READY.

        5. Build a ComputeUnitTrace capturing the full state of the SM.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            A ComputeUnitTrace for this cycle.
        """
        self._cycle += 1

        # Phase 1: Tick stall counters
        for sched in self._schedulers:
            sched.tick_stalls()

        # Phase 2: Each scheduler picks a warp and executes it
        engine_traces: dict[int, EngineTrace] = {}
        scheduler_actions: list[str] = []

        for sched in self._schedulers:
            picked = sched.pick_warp()
            if picked is None:
                scheduler_actions.append(
                    f"S{sched.scheduler_id}: no ready warp"
                )
                continue

            # Mark as running
            picked.state = WarpState.RUNNING

            # Execute one cycle on the warp's engine
            trace = picked.engine.step(clock_edge)
            engine_traces[picked.warp_id] = trace

            # Record the scheduling decision
            sched.mark_issued(picked.warp_id)
            scheduler_actions.append(
                f"S{sched.scheduler_id}: issued warp {picked.warp_id}"
            )

            # Phase 3: Check execution results and update warp state
            if picked.engine.halted:
                picked.state = WarpState.COMPLETED
            elif self._is_memory_instruction(trace):
                # Stall for memory latency
                picked.state = WarpState.STALLED_MEMORY
                picked.stall_counter = self._config.memory_latency_cycles
            else:
                picked.state = WarpState.READY

        # Build the trace
        active_warps = sum(
            1 for w in self._all_warp_slots
            if w.state != WarpState.COMPLETED
        )
        total_warps = self._config.max_warps

        return ComputeUnitTrace(
            cycle=self._cycle,
            unit_name=self.name,
            architecture=self.architecture,
            scheduler_action="; ".join(scheduler_actions),
            active_warps=active_warps,
            total_warps=total_warps,
            engine_traces=engine_traces,
            shared_memory_used=self._shared_memory_used,
            shared_memory_total=self._config.shared_memory_size,
            register_file_used=self._registers_allocated,
            register_file_total=self._config.register_file_size,
            occupancy=active_warps / total_warps if total_warps > 0 else 0.0,
        )

    def run(self, max_cycles: int = 100000) -> list[ComputeUnitTrace]:
        """Run until all work completes or max_cycles.

        Creates clock edges internally to drive execution.

        Args:
            max_cycles: Safety limit to prevent infinite loops.

        Returns:
            List of ComputeUnitTrace records, one per cycle.
        """
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
        """Reset all state: engines, schedulers, shared memory."""
        for sched in self._schedulers:
            sched.reset()
        self._all_warp_slots.clear()
        self._shared_memory.reset()
        self._shared_memory_used = 0
        self._registers_allocated = 0
        self._active_blocks.clear()
        self._next_warp_id = 0
        self._cycle = 0

    # --- Private helpers ---

    def _is_memory_instruction(self, trace: EngineTrace) -> bool:
        """Check if the executed instruction was a memory operation.

        Memory operations (LOAD/STORE) stall the warp for
        memory_latency_cycles to simulate global memory latency.

        We detect this by checking the trace description for keywords
        that indicate a memory operation.
        """
        desc = trace.description.upper()
        return "LOAD" in desc or "STORE" in desc

    def __repr__(self) -> str:
        active = sum(
            1 for w in self._all_warp_slots
            if w.state != WarpState.COMPLETED
        )
        return (
            f"StreamingMultiprocessor(warps={active}/{self._config.max_warps}, "
            f"occupancy={self.occupancy:.1%}, "
            f"policy={self._config.scheduling_policy.value})"
        )
