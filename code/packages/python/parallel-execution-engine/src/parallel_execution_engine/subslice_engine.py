"""SubsliceEngine — Intel Xe hybrid SIMD execution engine.

=== What is a Subslice? ===

Intel's GPU architecture uses a hierarchical organization that's different
from both NVIDIA's SIMT warps and AMD's SIMD wavefronts. The basic unit
is the "subslice" (also called "sub-slice" or "dual sub-slice" in newer
architectures).

A subslice contains:
- Multiple Execution Units (EUs), typically 8
- Each EU runs multiple hardware threads, typically 7
- Each thread processes SIMD8 (8-wide vector) instructions

    ┌──────────────────────────────────────────────────────┐
    │  Subslice                                            │
    │                                                      │
    │  ┌──────────────────────┐  ┌──────────────────────┐  │
    │  │  EU 0                │  │  EU 1                │  │
    │  │  ┌────────────────┐  │  │  ┌────────────────┐  │  │
    │  │  │ Thread 0: SIMD8│  │  │  │ Thread 0: SIMD8│  │  │
    │  │  │ Thread 1: SIMD8│  │  │  │ Thread 1: SIMD8│  │  │
    │  │  │ ...            │  │  │  │ ...            │  │  │
    │  │  │ Thread 6: SIMD8│  │  │  │ Thread 6: SIMD8│  │  │
    │  │  └────────────────┘  │  │  └────────────────┘  │  │
    │  │  Thread Arbiter      │  │  Thread Arbiter      │  │
    │  └──────────────────────┘  └──────────────────────┘  │
    │                                                      │
    │  ... (EU 2 through EU 7, same structure) ...         │
    │                                                      │
    │  Shared Local Memory (SLM): 64 KB                    │
    │  Instruction Cache                                   │
    │  Thread Dispatcher                                   │
    └──────────────────────────────────────────────────────┘

=== Why Multiple Threads Per EU? ===

This is Intel's approach to latency hiding. When one thread is stalled
(waiting for memory, for example), the EU's thread arbiter switches to
another ready thread. This keeps the SIMD ALU busy even when individual
threads are blocked.

    EU Thread Arbiter timeline:

    Cycle 1: Thread 0 executes SIMD8 add    ← thread 0 is ready
    Cycle 2: Thread 0 stalls (cache miss)   ← thread 0 blocked
    Cycle 3: Thread 3 executes SIMD8 mul    ← switch to thread 3
    Cycle 4: Thread 3 executes SIMD8 add    ← thread 3 still ready
    Cycle 5: Thread 0 data arrives          ← thread 0 ready again
    Cycle 6: Thread 0 executes SIMD8 store

The arbiter uses round-robin or oldest-first scheduling among ready threads.

=== Total Parallelism ===

One subslice: 8 EUs x 7 threads x 8 SIMD lanes = 448 operations per cycle.
That's a LOT of parallelism from a single subslice.

A full Intel Arc GPU has multiple slices, each with multiple subslices:
    Arc A770: 8 render slices x 4 subslices x 16 EUs = 512 EUs total

=== Simplification for Our Simulator ===

We model each EU thread as a set of SIMD8 GPUCore instances (8 cores
per thread, representing the 8 SIMD lanes). The thread arbiter picks
one ready thread per EU per cycle.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat
from gpu_core import GenericISA, GPUCore

from parallel_execution_engine.protocols import (
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
class SubsliceConfig:
    """Configuration for an Intel Xe-style SIMD subslice.

    Real-world reference values:

        Architecture   │ EUs/subslice │ Threads/EU │ SIMD Width │ GRF
        ───────────────┼──────────────┼────────────┼────────────┼─────
        Intel Xe-LP    │ 16           │ 7          │ 8          │ 128
        Intel Xe-HPG   │ 16           │ 8          │ 8/16       │ 128
        Intel Xe-HPC   │ 16           │ 8          │ 8/16/32    │ 128
        Our default    │ 8            │ 7          │ 8          │ 128

    Fields:
        num_eus:         Number of execution units in the subslice.
        threads_per_eu:  Hardware threads per EU (for latency hiding).
        simd_width:      SIMD vector width (8 for SIMD8, 16 for SIMD16).
        grf_size:        General register file entries per EU.
        slm_size:        Shared local memory size in bytes.
        float_format:    FP format for computations.
        isa:             Instruction set to use.
    """

    num_eus: int = 8
    threads_per_eu: int = 7
    simd_width: int = 8
    grf_size: int = 128
    slm_size: int = 65536
    float_format: FloatFormat = FP32
    isa: InstructionSet = field(default_factory=GenericISA)


# ---------------------------------------------------------------------------
# Execution Unit — manages multiple hardware threads
# ---------------------------------------------------------------------------


class ExecutionUnit:
    """One Execution Unit (EU) in the subslice.

    Each EU has multiple hardware threads and a thread arbiter that picks
    one ready thread to execute per cycle. Each thread runs SIMD8
    instructions, which we simulate with one GPUCore per SIMD lane.

    === Thread Arbitration ===

    The arbiter's job is to keep the SIMD ALU busy. On each cycle, it:
    1. Scans all threads to find which are "ready" (not stalled).
    2. Picks one ready thread (round-robin among ready threads).
    3. Issues that thread's next SIMD8 instruction.

    This is how Intel hides memory latency — while one thread waits for
    data, another thread runs. With 7 threads per EU, the ALU can stay
    busy even with high-latency memory operations.
    """

    def __init__(
        self,
        eu_id: int,
        config: SubsliceConfig,
    ) -> None:
        self.eu_id = eu_id
        self._config = config
        self._current_thread = 0  # round-robin arbiter index

        # Each thread has `simd_width` SIMD lanes, each backed by a GPUCore.
        # _threads[thread_id] = list of GPUCore (one per SIMD lane)
        self._threads: list[list[GPUCore]] = [
            [
                GPUCore(
                    isa=config.isa,
                    fmt=config.float_format,
                    num_registers=min(config.grf_size, 256),
                    memory_size=config.slm_size // max(config.threads_per_eu, 1),
                )
                for _ in range(config.simd_width)
            ]
            for _ in range(config.threads_per_eu)
        ]

        self._thread_active: list[bool] = [
            False
        ] * config.threads_per_eu
        self._program: list[Instruction] = []

    @property
    def threads(self) -> list[list[GPUCore]]:
        """Access to thread SIMD lanes."""
        return self._threads

    def load_program(self, program: list[Instruction]) -> None:
        """Load a program into all threads of this EU."""
        self._program = list(program)
        for thread_id in range(self._config.threads_per_eu):
            for lane_core in self._threads[thread_id]:
                lane_core.load_program(self._program)
            self._thread_active[thread_id] = True
        self._current_thread = 0

    def set_thread_lane_register(
        self,
        thread_id: int,
        lane: int,
        reg: int,
        value: float,
    ) -> None:
        """Set a register value for a specific lane of a specific thread."""
        self._threads[thread_id][lane].registers.write_float(reg, value)

    def step(self) -> dict[int, str]:
        """Execute one cycle using the thread arbiter.

        The arbiter selects one ready thread and executes its SIMD8
        instruction across all lanes.

        Returns:
            Dict mapping thread_id to trace description.
        """
        traces: dict[int, str] = {}

        # Find a ready thread using round-robin
        thread_id = self._find_ready_thread()
        if thread_id is None:
            return traces

        # Execute SIMD8 instruction on all lanes of the selected thread
        lane_descriptions: list[str] = []
        for lane_core in self._threads[thread_id]:
            if not lane_core.halted:
                try:
                    trace = lane_core.step()
                    lane_descriptions.append(trace.description)
                    if trace.halted:
                        pass  # Lane halted
                except RuntimeError:
                    lane_descriptions.append("(error)")

        # Check if all lanes of this thread are halted
        if all(c.halted for c in self._threads[thread_id]):
            self._thread_active[thread_id] = False

        if lane_descriptions:
            traces[thread_id] = (
                f"Thread {thread_id}: SIMD{self._config.simd_width} "
                f"— {lane_descriptions[0]}"
            )

        return traces

    def _find_ready_thread(self) -> int | None:
        """Find the next ready thread using round-robin arbitration.

        Scans threads starting from the last-executed thread + 1,
        wrapping around. Returns the first thread that is active and
        has non-halted lanes.

        Returns:
            Thread ID of the selected thread, or None if all are stalled/done.
        """
        for offset in range(self._config.threads_per_eu):
            tid = (self._current_thread + offset) % self._config.threads_per_eu
            if self._thread_active[tid] and any(
                not c.halted for c in self._threads[tid]
            ):
                self._current_thread = (tid + 1) % self._config.threads_per_eu
                return tid
        return None

    @property
    def all_halted(self) -> bool:
        """True if all threads on this EU are done."""
        return not any(self._thread_active)

    def reset(self) -> None:
        """Reset all threads on this EU."""
        for thread_id in range(self._config.threads_per_eu):
            for lane_core in self._threads[thread_id]:
                lane_core.reset()
                if self._program:
                    lane_core.load_program(self._program)
            self._thread_active[thread_id] = bool(self._program)
        self._current_thread = 0


# ---------------------------------------------------------------------------
# SubsliceEngine — the hybrid SIMD execution engine
# ---------------------------------------------------------------------------


class SubsliceEngine:
    """Intel Xe-style subslice execution engine.

    Manages multiple EUs, each with multiple hardware threads, each
    processing SIMD8 vectors. The thread arbiter in each EU selects
    one ready thread per cycle.

    === Parallelism Hierarchy ===

        Subslice (this engine)
        └── EU 0
        │   ├── Thread 0: SIMD8 [lane0, lane1, ..., lane7]
        │   ├── Thread 1: SIMD8 [lane0, lane1, ..., lane7]
        │   └── ... (threads_per_eu threads)
        └── EU 1
        │   ├── Thread 0: SIMD8
        │   └── ...
        └── ... (num_eus EUs)

    Total parallelism = num_eus * threads_per_eu * simd_width

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock()
        >>> engine = SubsliceEngine(
        ...     SubsliceConfig(num_eus=2, threads_per_eu=2, simd_width=4),
        ...     clock
        ... )
        >>> engine.load_program([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()])
        >>> traces = engine.run()
    """

    def __init__(self, config: SubsliceConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0
        self._program: list[Instruction] = []

        # Create the EUs
        self._eus: list[ExecutionUnit] = [
            ExecutionUnit(eu_id=i, config=config)
            for i in range(config.num_eus)
        ]

        self._all_halted = False

    # --- Properties ---

    @property
    def name(self) -> str:
        """Engine name for traces."""
        return "SubsliceEngine"

    @property
    def width(self) -> int:
        """Total SIMD parallelism across all EUs and threads."""
        return (
            self._config.num_eus
            * self._config.threads_per_eu
            * self._config.simd_width
        )

    @property
    def execution_model(self) -> ExecutionModel:
        """This is a SIMD engine (with multi-threading for latency hiding)."""
        return ExecutionModel.SIMD

    @property
    def halted(self) -> bool:
        """True if all EUs are done."""
        return self._all_halted

    @property
    def config(self) -> SubsliceConfig:
        """The configuration this engine was created with."""
        return self._config

    @property
    def eus(self) -> list[ExecutionUnit]:
        """Access to the execution units."""
        return self._eus

    # --- Program loading ---

    def load_program(self, program: list[Instruction]) -> None:
        """Load a program into all EUs and all threads.

        Every thread on every EU gets the same program. In real hardware,
        threads would be dispatched with different workloads, but for
        our simulator we load the same program everywhere.

        Args:
            program: A list of Instructions.
        """
        self._program = list(program)
        for eu in self._eus:
            eu.load_program(program)
        self._all_halted = False
        self._cycle = 0

    def set_eu_thread_lane_register(
        self,
        eu_id: int,
        thread_id: int,
        lane: int,
        reg: int,
        value: float,
    ) -> None:
        """Set a register for a specific lane of a specific thread on a specific EU.

        Args:
            eu_id: Which EU (0 to num_eus - 1).
            thread_id: Which thread (0 to threads_per_eu - 1).
            lane: Which SIMD lane (0 to simd_width - 1).
            reg: Which register.
            value: The float value.
        """
        self._eus[eu_id].set_thread_lane_register(thread_id, lane, reg, value)

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one cycle: each EU's arbiter picks one thread.

        On each cycle, every EU independently selects one ready thread
        and executes its SIMD instruction. This means up to num_eus
        threads can execute simultaneously (one per EU).

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            An EngineTrace describing what happened across all EUs.
        """
        self._cycle += 1

        if self._all_halted:
            return self._make_halted_trace()

        # Each EU steps independently
        all_traces: dict[int, str] = {}
        active_count = 0

        for eu in self._eus:
            if not eu.all_halted:
                eu_traces = eu.step()
                for thread_id, desc in eu_traces.items():
                    flat_id = eu.eu_id * self._config.threads_per_eu + thread_id
                    all_traces[flat_id] = f"EU{eu.eu_id}/{desc}"
                    active_count += self._config.simd_width

        # Check if all EUs are done
        if all(eu.all_halted for eu in self._eus):
            self._all_halted = True

        total = self.width

        # Build active mask (simplified: active threads * simd_width)
        active_mask = [False] * total
        for i in range(min(active_count, total)):
            active_mask[i] = True

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=(
                f"Subslice step — {active_count}/{total} lanes active "
                f"across {self._config.num_eus} EUs"
            ),
            unit_traces=all_traces,
            active_mask=active_mask,
            active_count=active_count,
            total_count=total,
            utilization=active_count / total if total > 0 else 0.0,
        )

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run until all EUs are done or max_cycles reached.

        Args:
            max_cycles: Safety limit.

        Returns:
            List of EngineTrace records.
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
                msg = f"SubsliceEngine: max_cycles ({max_cycles}) reached"
                raise RuntimeError(msg)
        return traces

    def reset(self) -> None:
        """Reset all EUs to initial state."""
        for eu in self._eus:
            eu.reset()
        self._all_halted = False
        self._cycle = 0

    def _make_halted_trace(self) -> EngineTrace:
        """Produce a trace for when all EUs are halted."""
        total = self.width
        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description="All EUs halted",
            unit_traces={},
            active_mask=[False] * total,
            active_count=0,
            total_count=total,
            utilization=0.0,
        )

    def __repr__(self) -> str:
        active_eus = sum(1 for eu in self._eus if not eu.all_halted)
        return (
            f"SubsliceEngine(eus={self._config.num_eus}, "
            f"active_eus={active_eus}, halted={self._all_halted})"
        )
