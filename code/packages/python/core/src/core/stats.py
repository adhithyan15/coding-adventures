"""CoreStats -- aggregate statistics from all core sub-components.

# Why Aggregate Statistics?

Each sub-component tracks its own statistics independently:
  - Pipeline: stall cycles, flush cycles, completed instructions
  - Branch Predictor: accuracy, misprediction count
  - Hazard Unit: forwarding count, stall count
  - Cache: hit rate, miss rate, evictions

CoreStats pulls all of these together into a single view, like the
dashboard of a car that shows speed (from the speedometer), fuel level
(from the tank sensor), and engine temperature (from the thermostat).

# Key Metrics

IPC (Instructions Per Cycle): the most important performance metric.

    IPC = instructions_completed / total_cycles

    IPC = 1.0: every cycle produces a result (ideal for scalar pipeline)
    IPC < 1.0: stalls and flushes are wasting cycles
    IPC > 1.0: superscalar (not modeled yet)

CPI (Cycles Per Instruction): the inverse of IPC.

    CPI = total_cycles / instructions_completed
"""

from __future__ import annotations

from dataclasses import dataclass, field

from branch_predictor import PredictionStats
from cache import CacheStats
from cpu_pipeline import PipelineStats


@dataclass
class CoreStats:
    """Aggregate statistics from every sub-component of a Core.

    Attributes:
        instructions_completed: Instructions that reached WB.
        total_cycles: Total clock cycles elapsed.
        pipeline_stats: Stats from the cpu-pipeline package.
        predictor_stats: Stats from the branch-predictor package.
        cache_stats: Map of cache level name to its statistics.
        forward_count: Total forwarding operations.
        stall_count: Total stall cycles from hazard detection.
        flush_count: Total pipeline flush events.
    """

    instructions_completed: int = 0
    total_cycles: int = 0
    pipeline_stats: PipelineStats = field(default_factory=PipelineStats)
    predictor_stats: PredictionStats | None = None
    cache_stats: dict[str, CacheStats] = field(default_factory=dict)
    forward_count: int = 0
    stall_count: int = 0
    flush_count: int = 0

    def ipc(self) -> float:
        """Return instructions per cycle.

        This is the primary measure of pipeline efficiency:
          - 1.0 = perfect (every cycle retires an instruction)
          - <1.0 = stalls/flushes wasting cycles
          - 0.0 = no instructions completed or no cycles elapsed
        """
        if self.total_cycles == 0:
            return 0.0
        return self.instructions_completed / self.total_cycles

    def cpi(self) -> float:
        """Return cycles per instruction.

        This is the inverse of IPC:
          - 1.0 = one cycle per instruction (ideal)
          - >1.0 = some cycles wasted
          - 0.0 = no instructions completed
        """
        if self.instructions_completed == 0:
            return 0.0
        return self.total_cycles / self.instructions_completed

    def __str__(self) -> str:
        """Return a formatted summary of all statistics.

        Produces a report similar to what a hardware performance counter
        tool (like Linux perf) would output.
        """
        result = "Core Statistics:\n"
        result += f"  Instructions completed: {self.instructions_completed}\n"
        result += f"  Total cycles:           {self.total_cycles}\n"
        result += f"  IPC: {self.ipc():.3f}   CPI: {self.cpi():.3f}\n"
        result += "\n"

        result += "Pipeline:\n"
        result += f"  Stall cycles:  {self.pipeline_stats.stall_cycles}\n"
        result += f"  Flush cycles:  {self.pipeline_stats.flush_cycles}\n"
        result += f"  Bubble cycles: {self.pipeline_stats.bubble_cycles}\n"
        result += "\n"

        if self.predictor_stats is not None:
            ps = self.predictor_stats
            result += "Branch Prediction:\n"
            result += f"  Total branches:  {ps.predictions}\n"
            result += f"  Correct:         {ps.correct}\n"
            result += f"  Mispredictions:  {ps.incorrect}\n"
            result += f"  Accuracy:        {ps.accuracy:.1f}%\n"
            result += "\n"

        if self.cache_stats:
            result += "Cache Performance:\n"
            for name, stats in self.cache_stats.items():
                result += (
                    f"  {name}: accesses={stats.total_accesses}, "
                    f"hit_rate={stats.hit_rate * 100:.1f}%\n"
                )
            result += "\n"

        result += "Hazards:\n"
        result += f"  Forwards: {self.forward_count}\n"
        result += f"  Stalls:   {self.stall_count}\n"
        result += f"  Flushes:  {self.flush_count}\n"

        return result
