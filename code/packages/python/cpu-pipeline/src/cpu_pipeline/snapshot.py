"""PipelineSnapshot and PipelineStats -- observing the pipeline's behavior.

A snapshot captures the full state of the pipeline at a single point in
time (one clock cycle). Think of it as a photograph of the assembly line:
you can see what instruction is at each station.

PipelineStats tracks performance statistics across the pipeline's execution.
These are the same metrics that hardware performance counters measure in
real CPUs.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cpu_pipeline.token import PipelineToken


# =========================================================================
# PipelineSnapshot -- the complete state of the pipeline at one moment
# =========================================================================


@dataclass
class PipelineSnapshot:
    """Captures the full state of the pipeline at a single point in time.

    Example snapshot for a 5-stage pipeline at cycle 7:

        Cycle 7:
          IF:  instr@28  (fetching instruction at PC=28)
          ID:  ADD@24    (decoding an ADD instruction)
          EX:  SUB@20    (executing a SUB)
          MEM: ---       (bubble -- pipeline was stalled here)
          WB:  LDR@12    (writing back a load result)
    """

    cycle: int = 0  # Clock cycle number when this snapshot was taken
    stages: dict[str, PipelineToken] = field(default_factory=dict)
    stalled: bool = False  # True if the pipeline was stalled this cycle
    flushing: bool = False  # True if a flush occurred this cycle
    pc: int = 0  # Current program counter (address of next fetch)

    def __str__(self) -> str:
        """Return a compact representation of the pipeline state."""
        return (
            f"[cycle {self.cycle}] PC={self.pc} "
            f"stalled={self.stalled} flushing={self.flushing}"
        )


# =========================================================================
# PipelineStats -- execution statistics
# =========================================================================


@dataclass
class PipelineStats:
    """Tracks performance statistics across the pipeline's execution.

    Key Metrics:

    IPC (Instructions Per Cycle): The most important pipeline metric.
        IPC = instructions_completed / total_cycles
        Ideal:       IPC = 1.0 (one instruction completes every cycle)
        With stalls: IPC < 1.0 (some cycles are wasted)
        Superscalar: IPC > 1.0 (multiple instructions per cycle)

    CPI (Cycles Per Instruction): The inverse of IPC.
        CPI = total_cycles / instructions_completed
        Ideal:   CPI = 1.0
        Typical: CPI = 1.2-2.0 for real workloads
    """

    total_cycles: int = 0  # Number of clock cycles executed
    instructions_completed: int = 0  # Non-bubble instructions that reached WB
    stall_cycles: int = 0  # Cycles where the pipeline was stalled
    flush_cycles: int = 0  # Cycles where a flush occurred
    bubble_cycles: int = 0  # Total stage-cycles occupied by bubbles

    def ipc(self) -> float:
        """Return instructions per cycle.

        IPC is the primary measure of pipeline efficiency:
          - IPC = 1.0: perfect pipeline utilization (ideal)
          - IPC < 1.0: some cycles are wasted (stalls, flushes)
          - IPC > 1.0: superscalar execution

        Returns 0.0 if no cycles have been executed (avoids division by zero).
        """
        if self.total_cycles == 0:
            return 0.0
        return self.instructions_completed / self.total_cycles

    def cpi(self) -> float:
        """Return cycles per instruction (inverse of IPC).

        CPI tells you how many clock cycles each instruction takes, on average:
          - CPI = 1.0: one cycle per instruction (ideal for scalar pipeline)
          - CPI = 1.5: 50% overhead from stalls and flushes
          - CPI = 0.5: two instructions per cycle (superscalar)

        Returns 0.0 if no instructions have completed (avoids division by zero).
        """
        if self.instructions_completed == 0:
            return 0.0
        return self.total_cycles / self.instructions_completed

    def __str__(self) -> str:
        """Return a formatted summary of pipeline statistics."""
        return (
            f"PipelineStats{{cycles={self.total_cycles}, "
            f"completed={self.instructions_completed}, "
            f"IPC={self.ipc():.3f}, CPI={self.cpi():.3f}, "
            f"stalls={self.stall_cycles}, flushes={self.flush_cycles}, "
            f"bubbles={self.bubble_cycles}}}"
        )
