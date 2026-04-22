"""Metrics data structures for the Tetrad VM (spec TET04).

The metrics layer is the distinguishing contribution of this VM.  Rather than
bolting instrumentation on afterwards, every meaningful runtime event is
captured in well-typed structures that the JIT (spec TET05) and external
analysis tools can inspect through a stable API.

Conceptual model
----------------
                 ┌───────────────────────────────────┐
  VM execution   │   instruction N fires              │
  ─────────────► │   slot k records observation "u8"  │
                 │   branch at ip 7: taken            │
                 └────────────────┬──────────────────┘
                                  │
                       ┌──────────▼──────────┐
                       │     VMMetrics        │
                       │  instruction_counts  │
                       │  function_call_counts│
                       │  loop_back_edge_counts│
                       │  branch_stats        │
                       │  total_instructions  │
                       │  immediate_jit_queue │
                       └─────────────────────┘

Feedback slot state machine
---------------------------

Every binary operation that involves an unknown-type operand allocates one
feedback slot.  The slot tracks a compact type profile:

   UNINITIALIZED  ──(first observation)──►  MONOMORPHIC
   MONOMORPHIC    ──(same type again)──────► MONOMORPHIC
   MONOMORPHIC    ──(new type)────────────►  POLYMORPHIC   (2–4 distinct types)
   POLYMORPHIC    ──(5th distinct type)───►  MEGAMORPHIC
   MEGAMORPHIC    ──(any observation)──────► MEGAMORPHIC   (never downgrades)

For Tetrad v1 all values are u8, so every slot stays MONOMORPHIC with
observations=["u8"].  The data is still written so the JIT can verify it reads
feedback correctly — it just never encounters a polymorphic slot.

When a Lisp front-end is added, a slot for `(+ x y)` might see ["u8", "pair"],
triggering the polymorphic path and inhibiting fast JIT specialisation.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from tetrad_compiler.bytecode import Instruction

__all__ = [
    "BranchStats",
    "SlotKind",
    "SlotState",
    "VMMetrics",
    "VMTrace",
]


# ---------------------------------------------------------------------------
# Slot-level type-feedback
# ---------------------------------------------------------------------------


class SlotKind(enum.Enum):
    """The four states of a feedback slot's type-profile.

    Analogous to V8 Ignition's IC (inline cache) states.  The progression is
    strictly monotonic: a slot can only move forward, never back.

    +───────────────+──────────────────────────────────────────────────────+
    | State         | Meaning                                              |
    +───────────────+──────────────────────────────────────────────────────+
    | UNINITIALIZED | Never reached.  Feedback vector just allocated.      |
    | MONOMORPHIC   | Exactly one type seen.  Fast JIT specialisation OK.  |
    | POLYMORPHIC   | 2–4 distinct types seen.  JIT emits type guards.     |
    | MEGAMORPHIC   | ≥5 types.  JIT skips specialisation entirely.        |
    +───────────────+──────────────────────────────────────────────────────+
    """

    UNINITIALIZED = "uninitialized"
    MONOMORPHIC = "monomorphic"
    POLYMORPHIC = "polymorphic"
    MEGAMORPHIC = "megamorphic"


@dataclass
class SlotState:
    """Runtime type profile for one feedback slot.

    ``kind``         — current IC state (see SlotKind).
    ``observations`` — ordered list of distinct type strings seen so far.
                       For Tetrad v1 this is always at most ["u8"].
    ``count``        — total times this slot was reached (including revisits
                       of the same type).  Used by the JIT to decide whether
                       the profile is warm enough to trust.
    """

    kind: SlotKind = field(default_factory=lambda: SlotKind.UNINITIALIZED)
    observations: list[str] = field(default_factory=list)
    count: int = 0


# ---------------------------------------------------------------------------
# Branch statistics
# ---------------------------------------------------------------------------


@dataclass
class BranchStats:
    """Taken / not-taken counters for one conditional branch instruction.

    ``taken_count``     — number of times the branch was taken (acc == 0 for JZ).
    ``not_taken_count`` — number of times the branch fell through.
    ``taken_ratio``     — fraction in [0.0, 1.0]; 0.0 if the branch was never reached.

    The JIT uses taken_ratio to decide branch layout:

      • taken_ratio > 0.9  → the branch body is the rare case; put it out-of-line.
      • taken_ratio < 0.1  → the fall-through is rare; put it out-of-line.
      • otherwise          → emit both paths inline.
    """

    taken_count: int = 0
    not_taken_count: int = 0

    @property
    def taken_ratio(self) -> float:
        """Fraction of executions where the branch was taken."""
        total = self.taken_count + self.not_taken_count
        return self.taken_count / total if total > 0 else 0.0


# ---------------------------------------------------------------------------
# Aggregate metrics
# ---------------------------------------------------------------------------


@dataclass
class VMMetrics:
    """All runtime observations gathered during a single VM execution.

    Fields
    ------
    instruction_counts
        opcode (int) → execution count.  Lets the JIT identify which opcodes
        dominate execution time.

    function_call_counts
        function name → invocation count.  Hot functions (called many times)
        are candidates for JIT compilation.

    loop_back_edge_counts
        function name → (ip-of-JMP_LOOP → iteration count).
        High iteration counts inform loop unrolling and OSR decisions.

    branch_stats
        function name → (ip-of-JZ-or-JNZ → BranchStats).
        Used for branch layout optimisation.

    total_instructions
        Aggregate executed instruction count across all functions.

    immediate_jit_queue
        Names of FULLY_TYPED functions, in declaration order.
        The JIT drains this queue and compiles these *before* the first
        instruction of main executes — giving zero-warmup for typed functions.
    """

    instruction_counts: dict[int, int] = field(default_factory=dict)
    function_call_counts: dict[str, int] = field(default_factory=dict)
    loop_back_edge_counts: dict[str, dict[int, int]] = field(default_factory=dict)
    branch_stats: dict[str, dict[int, BranchStats]] = field(default_factory=dict)
    total_instructions: int = 0
    immediate_jit_queue: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Execution trace (debug / test path)
# ---------------------------------------------------------------------------


@dataclass
class VMTrace:
    """A snapshot of VM state immediately before and after one instruction.

    Produced only by ``TetradVM.execute_traced`` — the overhead is one object
    allocation per instruction, so this path is intentionally opt-in.

    ``frame_depth``      — 0 = top-level (main); 1–3 = nested call depth.
    ``fn_name``          — name of the function being executed.
    ``ip``               — instruction pointer *before* the instruction fired.
    ``instruction``      — the Instruction object that was dispatched.
    ``acc_before``       — accumulator value *before* the instruction.
    ``acc_after``        — accumulator value *after* the instruction.
    ``registers_before`` — copy of the 8-register file before the instruction.
    ``registers_after``  — copy of the 8-register file after the instruction.
    ``feedback_delta``   — list of (slot_index, new_SlotState) pairs for every
                           feedback slot that changed during this instruction.
                           Empty for instructions that don't touch feedback.
    """

    frame_depth: int
    fn_name: str
    ip: int
    instruction: Instruction
    acc_before: int
    acc_after: int
    registers_before: list[int]
    registers_after: list[int]
    feedback_delta: list[tuple[int, SlotState]]
