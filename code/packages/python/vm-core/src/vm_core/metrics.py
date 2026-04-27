"""VMMetrics — a point-in-time snapshot of vm-core execution statistics.

Consumed by jit-core to decide when to promote a function to the
compiled tier, and by frontends (e.g. `tetrad-runtime`) to expose
language-level metric surfaces.

``VMMetrics`` is produced by ``VMCore.metrics()`` and is a plain
dataclass snapshot — it does not update after creation.  The live state
lives on ``VMCore`` itself; the snapshot is a deep copy so callers can
mutate it without affecting the running VM.

Aggregate counters
------------------

- ``function_call_counts`` — per-function interpreted-call counts.  The
  JIT promotes a function when its count crosses a tier threshold.
- ``total_instructions_executed`` — cumulative dispatch count.
- ``total_frames_pushed`` — cumulative frame-push count.
- ``total_jit_hits`` — count of calls that bypassed the interpreter via
  a registered JIT handler.

Branch / loop observations (LANG17)
-----------------------------------

- ``branch_stats`` — for every conditional branch
  (``jmp_if_true`` / ``jmp_if_false``), the taken / not-taken counts at
  that instruction site.  Keyed by ``(fn_name, source_ip)`` where
  ``source_ip`` is the index of the branch in the function's IIR
  instruction list.  A JIT reads these to decide branch layout (hot
  body inline, cold body out-of-line).
- ``loop_back_edge_counts`` — for every back-edge (any jump whose
  target index is strictly less than the source index), the number of
  times it was taken.  Keyed by ``(fn_name, source_ip)``.  Used by the
  JIT to identify hot loops for on-stack replacement or unrolling.

Keying by IIR index
-------------------

``source_ip`` is the **IIR instruction index** in the function's
``instructions`` list, *not* the byte-code index of the source
language.  Frontends that want to key by original source-language IP
(Tetrad does this for backwards compatibility) project through
``IIRFunction.source_map`` on the read side — see the tetrad-runtime
re-projection layer (PR4 of LANG17).

Zero-cost philosophy
--------------------

Branch and loop counters add one dict lookup and one integer increment
per conditional or back-taken jump.  That overhead is always-on today
because every metric surface we care about needs it; if a future
frontend wants to disable it, a ``VMConfig.collect_branch_stats`` flag
is a trivial extension (out of scope for now).
"""

from __future__ import annotations

from dataclasses import dataclass, field

__all__ = ["BranchStats", "VMMetrics"]


@dataclass
class BranchStats:
    """Taken / not-taken counters for one conditional-branch instruction.

    Attributes
    ----------
    taken_count:
        Number of times the branch was taken (condition satisfied).
    not_taken_count:
        Number of times the branch fell through (condition failed).

    The derived ``taken_ratio`` property is the fraction in ``[0.0, 1.0]``
    (or ``0.0`` if the branch has not been reached).  JITs use it for
    branch layout heuristics:

    - ``taken_ratio > 0.9`` → the fall-through is the rare case.
    - ``taken_ratio < 0.1`` → the branch body is the rare case; put it
      out-of-line.
    - otherwise → emit both paths inline.
    """

    taken_count: int = 0
    not_taken_count: int = 0

    @property
    def taken_ratio(self) -> float:
        """Fraction of reaches where the branch was taken."""
        total = self.taken_count + self.not_taken_count
        return self.taken_count / total if total > 0 else 0.0

    @property
    def total(self) -> int:
        """Total number of times this branch was reached."""
        return self.taken_count + self.not_taken_count


@dataclass
class VMMetrics:
    """Immutable snapshot of execution statistics at the moment
    ``VMCore.metrics()`` was called.

    Callers that want a live view should re-call ``metrics()`` — do not
    hold on to a ``VMMetrics`` across executions expecting it to update.
    """

    function_call_counts: dict[str, int] = field(default_factory=dict)
    total_instructions_executed: int = 0
    total_frames_pushed: int = 0
    total_jit_hits: int = 0
    branch_stats: dict[str, dict[int, BranchStats]] = field(default_factory=dict)
    """Per-function conditional-branch taken/not-taken counters.

    ``fn_name → source_ip → BranchStats``.  ``source_ip`` is the IIR
    instruction index of the ``jmp_if_true`` or ``jmp_if_false``.
    Only conditional branches appear here; unconditional ``jmp`` is
    not counted.
    """

    loop_back_edge_counts: dict[str, dict[int, int]] = field(default_factory=dict)
    """Per-function loop-back-edge hit counts.

    ``fn_name → source_ip → hit_count``.  A back-edge is any jump
    whose *target* index is strictly less than the *source* index —
    i.e., a jump that goes backward in the instruction list.  Every
    execution of such a jump (whether unconditional or a taken
    conditional) increments the counter.
    """
