"""Protocols — the unified interface for all parallel execution engines.

=== What is a Parallel Execution Engine? ===

At Layer 9 (gpu-core), we built a single processing element — one tiny
compute unit that executes one instruction at a time. Useful for learning,
but real accelerators never run just ONE core. They run THOUSANDS in parallel.

Layer 8 is where parallelism happens. It takes many Layer 9 cores (or
simpler processing elements) and orchestrates them to execute together.
But HOW they're orchestrated differs fundamentally across architectures:

    NVIDIA GPU:   32 threads in a "warp" — each has its own registers,
                  but they execute the same instruction (SIMT).

    AMD GPU:      32/64 "lanes" in a "wavefront" — one instruction stream,
                  one wide vector ALU, explicit execution mask (SIMD).

    Google TPU:   NxN grid of multiply-accumulate units — data FLOWS
                  through the array, no instructions at all (Systolic).

    Apple NPU:    Array of MACs driven by a compiler-generated schedule —
                  no runtime scheduler, just a fixed plan (Scheduled MAC).

    Intel GPU:    SIMD8 execution units with multiple hardware threads —
                  a hybrid of SIMD and multi-threading (Subslice).

Despite these radical differences, ALL of them share a common interface:
"advance one clock cycle, tell me what happened, report utilization."
That common interface is the ParallelExecutionEngine protocol.

=== Flynn's Taxonomy — A Quick Refresher ===

In 1966, Michael Flynn classified computer architectures:

    ┌───────────────────┬─────────────────┬─────────────────────┐
    │                   │ Single Data     │ Multiple Data        │
    ├───────────────────┼─────────────────┼─────────────────────┤
    │ Single Instr.     │ SISD (old CPU)  │ SIMD (vector proc.) │
    │ Multiple Instr.   │ MISD (rare)     │ MIMD (multi-core)   │
    └───────────────────┴─────────────────┴─────────────────────┘

Modern accelerators don't fit neatly into these boxes:
- NVIDIA coined "SIMT" because warps are neither pure SIMD nor pure MIMD.
- Systolic arrays don't have "instructions" at all.
- NPU scheduled arrays are driven by static compiler schedules.

Our ExecutionModel enum captures these real-world execution models.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from clock import ClockEdge


# ---------------------------------------------------------------------------
# ExecutionModel — the five parallel execution paradigms
# ---------------------------------------------------------------------------


class ExecutionModel(Enum):
    """The five parallel execution models supported by this package.

    Each model represents a fundamentally different way to organize parallel
    computation. They are NOT interchangeable — each has different properties
    around divergence, synchronization, and data movement.

    Think of these as "architectural philosophies":

        SIMT:          "Give every thread its own identity, execute together"
        SIMD:          "One instruction, wide ALU, explicit masking"
        SYSTOLIC:      "Data flows through a grid — no instructions needed"
        SCHEDULED_MAC: "Compiler decides everything — hardware just executes"
        VLIW:          "Pack multiple ops into one wide instruction word"

    Comparison table:

        Model          │ Has PC? │ Has threads? │ Divergence?     │ Used by
        ───────────────┼─────────┼──────────────┼─────────────────┼──────────
        SIMT           │ Yes*    │ Yes          │ HW-managed      │ NVIDIA
        SIMD           │ Yes     │ No (lanes)   │ Explicit mask   │ AMD
        SYSTOLIC       │ No      │ No           │ N/A             │ Google TPU
        SCHEDULED_MAC  │ No      │ No           │ Compile-time    │ Apple NPU
        VLIW           │ Yes     │ No           │ Predicated      │ Qualcomm

        * SIMT: each thread logically has its own PC, but they usually share one.
    """

    SIMT = "simt"
    SIMD = "simd"
    SYSTOLIC = "systolic"
    SCHEDULED_MAC = "scheduled_mac"
    VLIW = "vliw"


# ---------------------------------------------------------------------------
# DivergenceInfo — tracking branch divergence (SIMT/SIMD only)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DivergenceInfo:
    """Information about branch divergence during one execution step.

    === What is Divergence? ===

    When a group of threads/lanes encounters a branch (if/else), some may
    take the "true" path and others the "false" path. This is called
    "divergence" — the threads are no longer executing in lockstep.

        Before branch:    All 8 threads active: [1, 1, 1, 1, 1, 1, 1, 1]
        Branch condition:  thread_id < 4?
        After branch:     Only 4 active:        [1, 1, 1, 1, 0, 0, 0, 0]
                          The other 4 will run later.

    Divergence is the enemy of GPU performance. When half the threads are
    masked off, you're wasting half your hardware. Real GPU code tries to
    minimize divergence by ensuring threads in the same warp/wavefront
    take the same path.

    Fields:
        active_mask_before: Which units were active BEFORE the branch.
        active_mask_after:  Which units are active AFTER the branch.
        reconvergence_pc:   The instruction address where all units rejoin.
                            -1 if not applicable (e.g., SIMD explicit mask).
        divergence_depth:   How many nested divergent branches we're inside.
                            0 means no divergence. Higher = more serialization.
    """

    active_mask_before: list[bool]
    active_mask_after: list[bool]
    reconvergence_pc: int = -1
    divergence_depth: int = 0


# ---------------------------------------------------------------------------
# DataflowInfo — tracking data movement (Systolic only)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DataflowInfo:
    """Information about data flow in a systolic array.

    === What is Dataflow Execution? ===

    In a systolic array, there are no instructions. Instead, data "flows"
    through a grid of processing elements, like water flowing through pipes.
    Each PE does a multiply-accumulate and passes data to its neighbor.

    This dataclass tracks the state of every PE in the grid so we can
    visualize how data pulses through the array cycle by cycle.

    Fields:
        pe_states:      2D grid of PE state descriptions.
                        pe_states[row][col] = "acc=3.14, in=2.0"
        data_positions: Where each input value currently is in the array.
                        Maps input_id to (row, col) position.
    """

    pe_states: list[list[str]]
    data_positions: dict[str, tuple[int, int]] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# EngineTrace — the unified trace record for all engines
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class EngineTrace:
    """Record of one parallel execution step across ALL parallel units.

    === Why a Unified Trace? ===

    Every engine — warp, wavefront, systolic, MAC array — produces one
    EngineTrace per clock cycle. This lets higher layers (and tests, and
    visualization tools) treat all engines uniformly.

    The trace captures:
    1. WHAT happened (description, per-unit details)
    2. WHO was active (active_mask, utilization)
    3. HOW efficient it was (active_count / total_count)
    4. Engine-specific details (divergence for SIMT, dataflow for systolic)

    Example trace from a 4-thread warp:

        EngineTrace(
            cycle=3,
            engine_name="WarpEngine",
            execution_model=ExecutionModel.SIMT,
            description="FADD R2, R0, R1 — 3/4 threads active",
            unit_traces={
                0: "R2 = 1.0 + 2.0 = 3.0",
                1: "R2 = 3.0 + 4.0 = 7.0",
                2: "(masked — diverged)",
                3: "R2 = 5.0 + 6.0 = 11.0",
            },
            active_mask=[True, True, False, True],
            active_count=3,
            total_count=4,
            utilization=0.75,
        )

    Fields:
        cycle:           Clock cycle number.
        engine_name:     Which engine produced this trace.
        execution_model: The parallel execution model (SIMT, SIMD, etc.).
        description:     Human-readable summary of what happened.
        unit_traces:     Per-unit descriptions (thread/lane/PE/MAC index -> str).
        active_mask:     Which units were active this cycle.
        active_count:    How many units did useful work.
        total_count:     Total units available.
        utilization:     active_count / total_count (0.0 to 1.0).
        divergence_info: Branch divergence details (SIMT/SIMD only).
        dataflow_info:   Data flow state (systolic only).
    """

    cycle: int
    engine_name: str
    execution_model: ExecutionModel
    description: str
    unit_traces: dict[int, str]
    active_mask: list[bool]
    active_count: int
    total_count: int
    utilization: float
    divergence_info: DivergenceInfo | None = None
    dataflow_info: DataflowInfo | None = None

    def format(self) -> str:
        """Pretty-print the trace for educational display.

        Returns a multi-line string showing the cycle, engine, utilization,
        and per-unit details. Example output:

            [Cycle 3] WarpEngine (SIMT) — 75.0% utilization (3/4 active)
              FADD R2, R0, R1 — 3/4 threads active
              Thread 0: R2 = 1.0 + 2.0 = 3.0
              Thread 1: R2 = 3.0 + 4.0 = 7.0
              Thread 2: (masked — diverged)
              Thread 3: R2 = 5.0 + 6.0 = 11.0
        """
        pct = f"{self.utilization * 100:.1f}%"
        lines = [
            f"[Cycle {self.cycle}] {self.engine_name} "
            f"({self.execution_model.value.upper()}) "
            f"— {pct} utilization ({self.active_count}/{self.total_count} active)"
        ]
        lines.append(f"  {self.description}")

        for unit_id in sorted(self.unit_traces):
            lines.append(f"  Unit {unit_id}: {self.unit_traces[unit_id]}")

        if self.divergence_info is not None:
            di = self.divergence_info
            lines.append(
                f"  Divergence: depth={di.divergence_depth}, "
                f"reconvergence_pc={di.reconvergence_pc}"
            )

        return "\n".join(lines)


# ---------------------------------------------------------------------------
# ParallelExecutionEngine — the protocol all engines implement
# ---------------------------------------------------------------------------


@runtime_checkable
class ParallelExecutionEngine(Protocol):
    """The common interface for all parallel execution engines.

    === Protocol Design ===

    This protocol captures the minimal shared behavior of ALL parallel
    execution engines, regardless of execution model:

    1. name  — identify which engine this is
    2. width — how many parallel units (threads, lanes, PEs, MACs)
    3. execution_model — which paradigm (SIMT, SIMD, systolic, etc.)
    4. step(clock_edge) — advance one clock cycle
    5. halted — is all work complete?
    6. reset() — return to initial state

    Any class that has these methods and properties satisfies this protocol,
    even without explicitly inheriting from it. This is Python's "structural
    subtyping" — if it looks like an engine and steps like an engine, it IS
    an engine.

    === Why so minimal? ===

    Different engines have radically different APIs:
    - WarpEngine has load_program(), set_thread_register()
    - SystolicArray has load_weights(), feed_input()
    - MACArrayEngine has load_schedule(), load_inputs()

    Those are engine-specific. The protocol only captures what they ALL share,
    so that Layer 7 (the compute unit) can drive any engine uniformly.
    """

    @property
    def name(self) -> str:
        """Engine name: 'WarpEngine', 'WavefrontEngine', etc."""
        ...

    @property
    def width(self) -> int:
        """Parallelism width (threads, lanes, PEs, MACs)."""
        ...

    @property
    def execution_model(self) -> ExecutionModel:
        """Which parallel execution model this engine uses."""
        ...

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Advance one clock cycle. Returns a trace of what happened."""
        ...

    @property
    def halted(self) -> bool:
        """True if all work is complete."""
        ...

    def reset(self) -> None:
        """Reset to initial state."""
        ...
