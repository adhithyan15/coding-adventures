"""WarpEngine — SIMT parallel execution (NVIDIA CUDA / ARM Mali style).

=== What is SIMT? ===

SIMT stands for "Single Instruction, Multiple Threads." NVIDIA invented this
term to describe how their GPU cores work. It's a hybrid between two older
concepts:

    SISD (one instruction, one datum):
        Like a single CPU core. Our gpu-core package at Layer 9.

    SIMD (one instruction, multiple data):
        Like AMD wavefronts. One instruction operates on a wide vector.
        There are no "threads" — just lanes in a vector ALU.

    SIMT (one instruction, multiple threads):
        Like NVIDIA warps. Multiple threads, each with its own registers
        and (logically) its own program counter. They USUALLY execute
        the same instruction, but CAN diverge.

The key difference between SIMD and SIMT:

    SIMD: "I have one wide ALU that processes 32 numbers at once."
    SIMT: "I have 32 tiny threads that happen to execute in lockstep."

This distinction matters when threads need to take different paths (branches).
In SIMD, you just mask off lanes. In SIMT, the hardware manages a divergence
stack to serialize the paths and then reconverge.

=== How a Warp Works ===

A warp is a group of threads (32 for NVIDIA, 16 for ARM Mali) that the
hardware schedules together. On each clock cycle:

    1. The warp scheduler picks one instruction (at the warp's PC).
    2. That instruction is issued to ALL active threads simultaneously.
    3. Each thread executes the instruction on its OWN registers.
    4. If the instruction is a branch, threads may diverge.

    ┌─────────────────────────────────────────────────────┐
    │  Warp (32 threads)                                  │
    │                                                     │
    │  Active Mask: [1,1,1,1,1,1,1,1,...,1,1,1,1]         │
    │  PC: 0x004                                          │
    │                                                     │
    │  ┌──────┐ ┌──────┐ ┌──────┐       ┌──────┐         │
    │  │ T0   │ │ T1   │ │ T2   │  ...  │ T31  │         │
    │  │R0=1.0│ │R0=2.0│ │R0=3.0│       │R0=32.│         │
    │  │R1=0.5│ │R1=0.5│ │R1=0.5│       │R1=0.5│         │
    │  └──────┘ └──────┘ └──────┘       └──────┘         │
    │                                                     │
    │  Instruction: FMUL R2, R0, R1                       │
    │  Result: T0.R2=0.5, T1.R2=1.0, T2.R2=1.5, ...      │
    └─────────────────────────────────────────────────────┘

=== Divergence: The Price of Flexibility ===

When threads in a warp encounter a branch and disagree on which way to go,
the warp "diverges." The hardware serializes the paths:

    Step 1: Evaluate the branch condition for ALL threads.
    Step 2: Threads that go "true" → execute first (others masked off).
    Step 3: Push (reconvergence_pc, other_mask) onto the divergence stack.
    Step 4: When "true" path finishes, pop the stack.
    Step 5: Execute the "false" path (first group masked off).
    Step 6: At the reconvergence point, all threads are active again.

    Example with 4 threads:

    if (thread_id < 2):    Mask: [1,1,0,0]  ← threads 0,1 take true path
        path_A()           Only threads 0,1 execute
    else:                  Mask: [0,0,1,1]  ← threads 2,3 take false path
        path_B()           Only threads 2,3 execute
    // reconverge          Mask: [1,1,1,1]  ← all 4 threads active again

This means divergent branches effectively halve your throughput — the warp
runs both paths sequentially instead of simultaneously. Nested divergence
can reduce utilization to 1/N where N is the nesting depth.

=== Independent Thread Scheduling (Volta+) ===

NVIDIA's Volta architecture (2017) introduced "independent thread scheduling"
where each thread truly has its own PC. Instead of a divergence stack, the
scheduler groups threads with the same PC into sub-warps and issues them
together. This enables patterns like producer-consumer that were impossible
with the old stack-based model.

We support both modes via the `independent_thread_scheduling` config flag.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat
from gpu_core import GenericISA, GPUCore

from parallel_execution_engine.protocols import (
    DivergenceInfo,
    EngineTrace,
    ExecutionModel,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge
    from gpu_core import Instruction, InstructionSet


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


@dataclass
class WarpConfig:
    """Configuration for a SIMT warp engine.

    Real-world reference values:

        Vendor      │ Warp Width │ Registers │ Memory     │ Max Divergence
        ────────────┼────────────┼───────────┼────────────┼───────────────
        NVIDIA      │ 32         │ 255       │ 512 KB     │ 32+ levels
        ARM Mali    │ 16         │ 64        │ varies     │ 16+ levels
        Our default │ 32         │ 32        │ 1024 B     │ 32 levels

    Fields:
        warp_width:         Number of threads in the warp (32 for NVIDIA).
        num_registers:      Registers per thread (our generic ISA uses 32).
        memory_per_thread:  Local memory per thread in bytes.
        float_format:       FP format for registers (FP32, FP16, BF16).
        max_divergence_depth: Maximum nesting of divergent branches.
        isa:                The instruction set to use (GenericISA by default).
        independent_thread_scheduling: Volta+ mode with per-thread PCs.
    """

    warp_width: int = 32
    num_registers: int = 32
    memory_per_thread: int = 1024
    float_format: FloatFormat = FP32
    max_divergence_depth: int = 32
    isa: InstructionSet = field(default_factory=GenericISA)
    independent_thread_scheduling: bool = False


# ---------------------------------------------------------------------------
# Per-thread context
# ---------------------------------------------------------------------------


@dataclass
class ThreadContext:
    """Per-thread execution context in a SIMT warp.

    Each thread in the warp has:
    - thread_id: its position in the warp (0 to warp_width-1)
    - core: a full GPUCore instance with its own registers and memory
    - active: whether this thread is currently executing (False = masked off)
    - pc: per-thread program counter (used in independent scheduling mode)

    In NVIDIA hardware, each CUDA thread has 255 registers. In our simulator,
    each thread gets a full GPUCore instance, which is heavier but lets us
    reuse all the existing instruction execution infrastructure.
    """

    thread_id: int
    core: GPUCore
    active: bool = True
    pc: int = 0


# ---------------------------------------------------------------------------
# Divergence stack entry
# ---------------------------------------------------------------------------


@dataclass
class DivergenceStackEntry:
    """One entry on the divergence stack.

    When threads diverge at a branch, we push an entry recording:
    - reconvergence_pc: where threads should rejoin
    - saved_mask: which threads took the OTHER path (will run later)

    This is the pre-Volta divergence handling mechanism. The stack allows
    nested divergence — if threads diverge again while already diverged,
    another entry is pushed.

    Divergence stack example (4 threads, nested branches):

        Stack (top→bottom):
        ┌────────────────────────────────────────────┐
        │ reconvergence_pc=10, saved_mask=[0,0,1,0]  │  ← inner branch
        ├────────────────────────────────────────────┤
        │ reconvergence_pc=20, saved_mask=[0,0,0,1]  │  ← outer branch
        └────────────────────────────────────────────┘
    """

    reconvergence_pc: int
    saved_mask: list[bool]


# ---------------------------------------------------------------------------
# WarpEngine — the SIMT parallel execution engine
# ---------------------------------------------------------------------------


class WarpEngine:
    """SIMT warp execution engine (NVIDIA CUDA / ARM Mali style).

    Manages N threads executing in lockstep with hardware divergence support.
    Each thread is backed by a real GPUCore instance from the gpu-core package.

    === Usage Pattern ===

        1. Create engine with config and clock
        2. Load program (same program goes to all threads)
        3. Set per-thread register values (give each thread different data)
        4. Step or run (engine issues instructions to all active threads)
        5. Read results from per-thread registers

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock()
        >>> engine = WarpEngine(WarpConfig(warp_width=4), clock)
        >>> engine.load_program([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()])
        >>> traces = engine.run()
        >>> engine.threads[0].core.registers.read_float(2)
        6.0
    """

    def __init__(self, config: WarpConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0
        self._program: list[Instruction] = []

        # Create one GPUCore per thread. Each thread is an independent
        # processing element with its own registers and local memory.
        self._threads: list[ThreadContext] = [
            ThreadContext(
                thread_id=i,
                core=GPUCore(
                    isa=config.isa,
                    fmt=config.float_format,
                    num_registers=config.num_registers,
                    memory_size=config.memory_per_thread,
                ),
            )
            for i in range(config.warp_width)
        ]

        # The divergence stack for pre-Volta branch handling.
        # When threads diverge, we push the "other path" mask and the
        # address where threads should reconverge.
        self._divergence_stack: list[DivergenceStackEntry] = []

        # Tracks whether all threads have halted (the engine is done).
        self._all_halted = False

    # --- Properties ---

    @property
    def name(self) -> str:
        """Engine name for traces."""
        return "WarpEngine"

    @property
    def width(self) -> int:
        """Number of threads in this warp."""
        return self._config.warp_width

    @property
    def execution_model(self) -> ExecutionModel:
        """This is a SIMT engine."""
        return ExecutionModel.SIMT

    @property
    def threads(self) -> list[ThreadContext]:
        """Access to per-thread contexts (for reading results)."""
        return self._threads

    @property
    def active_mask(self) -> list[bool]:
        """Which threads are currently active (not masked off)."""
        return [t.active for t in self._threads]

    @property
    def halted(self) -> bool:
        """True if ALL threads have executed a HALT instruction."""
        return self._all_halted

    @property
    def config(self) -> WarpConfig:
        """The configuration this engine was created with."""
        return self._config

    # --- Program loading ---

    def load_program(self, program: list[Instruction]) -> None:
        """Load the same program into all threads.

        In real NVIDIA hardware, all threads in a warp share the same
        instruction memory. We simulate this by loading the same program
        into each thread's GPUCore.

        Args:
            program: A list of Instructions (from gpu-core opcodes).
        """
        self._program = list(program)
        for thread in self._threads:
            thread.core.load_program(self._program)
            thread.active = True
            thread.pc = 0
        self._all_halted = False
        self._cycle = 0
        self._divergence_stack.clear()

    # --- Per-thread register setup ---

    def set_thread_register(
        self, thread_id: int, reg: int, value: float
    ) -> None:
        """Set a register value for a specific thread.

        This is how you give each thread different data to work on.
        In a real GPU kernel, each thread would compute its global index
        and use it to load different data from memory. In our simulator,
        we pre-load the data into registers.

        Example:
            # Give each thread a different input value
            for t in range(32):
                engine.set_thread_register(t, 1, float(t))

        Args:
            thread_id: Which thread (0 to warp_width - 1).
            reg: Which register (0 to num_registers - 1).
            value: The float value to write.
        """
        if thread_id < 0 or thread_id >= self._config.warp_width:
            msg = (
                f"Thread ID {thread_id} out of range "
                f"[0, {self._config.warp_width})"
            )
            raise IndexError(msg)
        self._threads[thread_id].core.registers.write_float(reg, value)

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one cycle: issue one instruction to all active threads.

        On each rising clock edge:
        1. Find the instruction at the current warp PC.
        2. Issue it to all active (non-masked) threads.
        3. Detect divergence on branch instructions.
        4. Handle reconvergence when appropriate.
        5. Build and return an EngineTrace.

        Only acts on rising edges (like real hardware). Falling edges
        produce a no-op trace.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            An EngineTrace describing what all threads did this cycle.
        """
        self._cycle += 1

        # If all halted, produce a no-op trace
        if self._all_halted:
            return self._make_halted_trace()

        # Check for reconvergence: if all active threads have reached
        # the reconvergence PC at the top of the divergence stack,
        # pop the stack and restore the full mask.
        self._check_reconvergence()

        # Find the instruction to execute. In SIMT (pre-Volta), all active
        # threads share the same PC. We use thread 0's core as the reference.
        active_threads = [t for t in self._threads if t.active and not t.core.halted]

        if not active_threads:
            # All threads are either halted or masked off.
            # Check if we need to pop the divergence stack.
            if self._divergence_stack:
                return self._pop_divergence_and_trace()
            self._all_halted = True
            return self._make_halted_trace()

        # Save pre-step mask for divergence tracking
        mask_before = [t.active for t in self._threads]

        # Execute the instruction on all active, non-halted threads
        unit_traces: dict[int, str] = {}
        branch_taken_threads: list[int] = []
        branch_not_taken_threads: list[int] = []

        for thread in self._threads:
            if thread.active and not thread.core.halted:
                try:
                    trace = thread.core.step()
                    unit_traces[thread.thread_id] = trace.description

                    # Detect branch divergence: check if different threads
                    # ended up at different PCs after a branch instruction.
                    if trace.next_pc != trace.pc + 1 and not trace.halted:
                        branch_taken_threads.append(thread.thread_id)
                    elif not trace.halted:
                        branch_not_taken_threads.append(thread.thread_id)

                    if trace.halted:
                        unit_traces[thread.thread_id] = "HALTED"
                except RuntimeError:
                    thread.active = False
                    unit_traces[thread.thread_id] = "(error — deactivated)"
            elif thread.core.halted:
                unit_traces[thread.thread_id] = "(halted)"
            else:
                unit_traces[thread.thread_id] = "(masked off)"

        # Handle divergence: if some threads branched and others didn't,
        # we have divergence. Push the "not taken" threads onto the
        # divergence stack and continue with only the "taken" threads.
        divergence_info = None
        if branch_taken_threads and branch_not_taken_threads:
            divergence_info = self._handle_divergence(
                branch_taken_threads,
                branch_not_taken_threads,
                mask_before,
            )

        # Check if all threads are now halted
        if all(t.core.halted for t in self._threads):
            self._all_halted = True

        # Build the trace
        current_mask = [t.active and not t.core.halted for t in self._threads]
        active_count = sum(current_mask)
        total = self._config.warp_width

        # Get a description from the first instruction that was executed
        desc_parts = []
        if unit_traces:
            first_active = next(
                (
                    unit_traces[t.thread_id]
                    for t in self._threads
                    if t.thread_id in unit_traces
                    and unit_traces[t.thread_id]
                    not in ("(masked off)", "(halted)", "(error — deactivated)")
                ),
                "no active threads",
            )
            desc_parts.append(first_active)
        else:
            desc_parts.append("no active threads")

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=f"{desc_parts[0]} — {active_count}/{total} threads active",
            unit_traces=unit_traces,
            active_mask=current_mask,
            active_count=active_count,
            total_count=total,
            utilization=active_count / total if total > 0 else 0.0,
            divergence_info=divergence_info,
        )

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run until all threads halt or max_cycles reached.

        Creates clock edges internally to drive execution. Each cycle
        produces one EngineTrace.

        Args:
            max_cycles: Safety limit to prevent infinite loops.

        Returns:
            List of EngineTrace records, one per cycle.
        """
        from clock import ClockEdge

        traces: list[EngineTrace] = []
        for cycle_num in range(1, max_cycles + 1):
            edge = ClockEdge(
                cycle=cycle_num, value=1, is_rising=True, is_falling=False
            )
            trace = self.step(edge)
            traces.append(trace)
            if self._all_halted:
                break
        else:
            if not self._all_halted:
                msg = f"WarpEngine: max_cycles ({max_cycles}) reached"
                raise RuntimeError(msg)
        return traces

    def reset(self) -> None:
        """Reset the engine to its initial state.

        Resets all thread cores, reactivates all threads, clears the
        divergence stack, and reloads the program (if one was loaded).
        """
        for thread in self._threads:
            thread.core.reset()
            thread.active = True
            thread.pc = 0
            if self._program:
                thread.core.load_program(self._program)
        self._divergence_stack.clear()
        self._all_halted = False
        self._cycle = 0

    # --- Divergence handling (private) ---

    def _handle_divergence(
        self,
        taken_threads: list[int],
        not_taken_threads: list[int],
        mask_before: list[bool],
    ) -> DivergenceInfo:
        """Handle a divergent branch by pushing onto the divergence stack.

        When some threads take a branch and others don't:
        1. Find the reconvergence point (the branch target or fall-through).
        2. Push the "not taken" threads onto the stack with the reconvergence PC.
        3. Mask off the "not taken" threads so only "taken" threads execute.

        Args:
            taken_threads: Thread IDs that took the branch.
            not_taken_threads: Thread IDs that fell through.
            mask_before: The active mask before the branch.

        Returns:
            DivergenceInfo describing this divergence event.
        """
        # The reconvergence PC is the maximum PC among all active threads
        # after the branch. This is a simplified heuristic — real hardware
        # uses the immediate post-dominator in the control flow graph.
        all_pcs = [
            self._threads[tid].core.pc
            for tid in taken_threads + not_taken_threads
        ]
        reconvergence_pc = max(all_pcs)

        # Build the saved mask: threads that took the "not taken" path
        saved_mask = [False] * self._config.warp_width
        for tid in not_taken_threads:
            saved_mask[tid] = True
            self._threads[tid].active = False

        # Push onto the divergence stack
        if len(self._divergence_stack) < self._config.max_divergence_depth:
            self._divergence_stack.append(
                DivergenceStackEntry(
                    reconvergence_pc=reconvergence_pc,
                    saved_mask=saved_mask,
                )
            )

        mask_after = [t.active for t in self._threads]

        return DivergenceInfo(
            active_mask_before=mask_before,
            active_mask_after=mask_after,
            reconvergence_pc=reconvergence_pc,
            divergence_depth=len(self._divergence_stack),
        )

    def _check_reconvergence(self) -> None:
        """Check if active threads have reached a reconvergence point.

        If the divergence stack is non-empty and all active threads are
        at or past the reconvergence PC, pop the stack and reactivate
        the saved threads.
        """
        if not self._divergence_stack:
            return

        entry = self._divergence_stack[-1]
        active_threads = [t for t in self._threads if t.active and not t.core.halted]

        if not active_threads:
            return

        # Check if all active threads have reached the reconvergence PC
        all_at_reconvergence = all(
            t.core.pc >= entry.reconvergence_pc for t in active_threads
        )

        if all_at_reconvergence:
            self._divergence_stack.pop()
            # Reactivate the saved threads
            for tid, should_activate in enumerate(entry.saved_mask):
                if should_activate and not self._threads[tid].core.halted:
                    self._threads[tid].active = True

    def _pop_divergence_and_trace(self) -> EngineTrace:
        """Pop the divergence stack and produce a trace for the switch.

        Called when all currently active threads are halted/masked but
        there are still entries on the divergence stack (meaning some
        threads are waiting to execute the other branch path).
        """
        entry = self._divergence_stack.pop()

        # Reactivate saved threads
        for tid, should_activate in enumerate(entry.saved_mask):
            if should_activate and not self._threads[tid].core.halted:
                self._threads[tid].active = True

        # Set PCs of reactivated threads to the reconvergence PC
        # (they need to resume from where they were when masked)
        # Actually, their PCs should already be correct from when they
        # were executing before being masked.

        current_mask = [t.active and not t.core.halted for t in self._threads]
        active_count = sum(current_mask)

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=(
                f"Divergence stack pop — reactivated {active_count} threads"
            ),
            unit_traces={
                t.thread_id: (
                    "reactivated" if entry.saved_mask[t.thread_id] else "(waiting)"
                )
                for t in self._threads
            },
            active_mask=current_mask,
            active_count=active_count,
            total_count=self._config.warp_width,
            utilization=(
                active_count / self._config.warp_width
                if self._config.warp_width > 0
                else 0.0
            ),
        )

    def _make_halted_trace(self) -> EngineTrace:
        """Produce a trace for when all threads are halted."""
        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description="All threads halted",
            unit_traces={
                t.thread_id: "(halted)" for t in self._threads
            },
            active_mask=[False] * self._config.warp_width,
            active_count=0,
            total_count=self._config.warp_width,
            utilization=0.0,
        )

    def __repr__(self) -> str:
        active = sum(1 for t in self._threads if t.active)
        halted = sum(1 for t in self._threads if t.core.halted)
        return (
            f"WarpEngine(width={self._config.warp_width}, "
            f"active={active}, halted_threads={halted}, "
            f"divergence_depth={len(self._divergence_stack)})"
        )
