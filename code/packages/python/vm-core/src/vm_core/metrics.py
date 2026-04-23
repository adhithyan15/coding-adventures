"""VMMetrics — a point-in-time snapshot of vm-core execution statistics.

Consumed by jit-core to decide when to promote a function to the compiled tier.

The metrics object is produced by ``VMCore.metrics()`` and is a plain dataclass
snapshot — it does not update after creation.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class VMMetrics:
    """Execution statistics snapshot.

    Parameters
    ----------
    function_call_counts:
        Maps function name → number of times that function has been called
        through the interpreter.  Functions that were JIT-compiled and called
        via the JIT handler are NOT counted here — only interpreted calls.
    total_instructions_executed:
        Cumulative count of IIRInstr dispatch cycles across all functions.
    total_frames_pushed:
        Cumulative count of call frames pushed (one per CALL instruction
        executed by the interpreter).
    total_jit_hits:
        Cumulative count of calls intercepted by a registered JIT handler
        (i.e. calls that bypassed the interpreter).
    """

    function_call_counts: dict[str, int] = field(default_factory=dict)
    total_instructions_executed: int = 0
    total_frames_pushed: int = 0
    total_jit_hits: int = 0
