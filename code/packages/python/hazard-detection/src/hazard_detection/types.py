"""Shared data types for pipeline hazard detection.

=== Why These Types Exist ===

A CPU pipeline is like an assembly line: each stage works on a different
instruction simultaneously. But sometimes instructions interfere with each
other — one instruction needs a result that another hasn't produced yet,
or two instructions fight over the same hardware resource.

The hazard detection unit needs to know what each pipeline stage is doing
WITHOUT knowing the specifics of the instruction set. It doesn't care
whether you're running ARM, RISC-V, or x86 — it only needs to know:

  1. Which registers does this instruction READ?
  2. Which register does it WRITE?
  3. Is it a branch? Was it predicted correctly?
  4. What hardware resources does it need (ALU, FP unit, memory)?

These types capture exactly that information, nothing more.

=== The Pipeline Stages (5-Stage Classic) ===

    IF → ID → EX → MEM → WB
    │    │    │     │     │
    │    │    │     │     └─ Write Back: write result to register file
    │    │    │     └─ Memory: load/store data from/to memory
    │    │    └─ Execute: ALU computes result
    │    └─ Instruction Decode: read registers, detect hazards
    └─ Instruction Fetch: grab instruction from memory

The hazard unit sits between ID and EX. It peeks at what's in each stage
and decides: "Can ID proceed, or do we need to stall/forward/flush?"
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


# ---------------------------------------------------------------------------
# PipelineSlot — what the hazard unit sees in each pipeline stage
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class PipelineSlot:
    """Information about an instruction occupying a pipeline stage.

    This is ISA-independent — whatever decoder is plugged in extracts this
    info from raw instruction bits. The hazard unit only cares about register
    numbers and resource usage, not opcodes.

    === Fields Explained ===

    valid:
        Is there actually an instruction here? After a flush or at startup,
        stages contain "bubbles" (empty slots) — valid=False.

    pc:
        Program counter. Useful for debugging ("which instruction caused
        the hazard?"), not used for hazard logic itself.

    source_regs:
        Tuple of register numbers this instruction READS. For example,
        ADD R1, R2, R3 reads R2 and R3, so source_regs = (2, 3).
        A tuple (not list) because it's frozen/immutable.

    dest_reg:
        The register this instruction WRITES. ADD R1, R2, R3 writes R1,
        so dest_reg = 1. Instructions that don't write (like a store or
        a branch) have dest_reg = None.

    dest_value:
        The computed result, if available. After the EX stage, the ALU
        result is known. After MEM, the loaded value is known. This is
        what gets forwarded to avoid stalls.

    is_branch / branch_taken / branch_predicted_taken:
        Branch instructions change the flow of execution. The predictor
        guesses the outcome during IF; the actual outcome is known in EX.
        If the guess was wrong, we must flush the pipeline.

    mem_read / mem_write:
        Load (mem_read=True) and store (mem_write=True) instructions.
        Loads are special because the value isn't available until after
        MEM — so a load followed immediately by a use MUST stall.

    uses_alu / uses_fp:
        Which execution unit does this instruction need? Most instructions
        use the ALU. Floating-point ops use the FP unit. If two instructions
        in the pipeline need the same unit at the same time, that's a
        structural hazard.

    === Example: Encoding "ADD R1, R2, R3" ===

        PipelineSlot(
            valid=True,
            pc=0x1000,
            source_regs=(2, 3),   # reads R2 and R3
            dest_reg=1,           # writes R1
            dest_value=None,      # not computed yet (still in ID)
            is_branch=False,
            branch_taken=False,
            branch_predicted_taken=False,
            mem_read=False,
            mem_write=False,
            uses_alu=True,
            uses_fp=False,
        )
    """

    valid: bool = False
    pc: int = 0
    source_regs: tuple[int, ...] = ()
    dest_reg: int | None = None
    dest_value: int | None = None
    is_branch: bool = False
    branch_taken: bool = False
    branch_predicted_taken: bool = False
    mem_read: bool = False
    mem_write: bool = False
    uses_alu: bool = True
    uses_fp: bool = False


# ---------------------------------------------------------------------------
# HazardAction — what the hazard unit tells the pipeline to do
# ---------------------------------------------------------------------------


class HazardAction(Enum):
    """The action the hazard unit instructs the pipeline to take.

    Think of these as traffic signals for the pipeline:

    === NONE (Green Light) ===
    Everything is fine. The pipeline flows normally.

    === FORWARD_FROM_EX (Yellow Shortcut from EX) ===
    "The value you need is right HERE in the EX stage — grab it!"
    Instead of waiting for the instruction to reach WB, we wire the
    EX output directly back to the ID input. No time lost.

        Without forwarding:         With forwarding:
        ADD R1, R2, R3  [EX]       ADD R1, R2, R3  [EX] ──┐
        SUB R4, R1, R5  [ID] STALL SUB R4, R1, R5  [ID] ←─┘ OK!

    === FORWARD_FROM_MEM (Yellow Shortcut from MEM) ===
    Same idea, but the value comes from the MEM stage. This happens
    when there's a 2-instruction gap, or after a load completes.

    === STALL (Red Light) ===
    "STOP! You can't proceed yet." The pipeline freezes the IF and ID
    stages and inserts a bubble (NOP) into EX. This happens when
    forwarding can't help — typically a load-use hazard:

        LW R1, [addr]   [EX]  ← value won't be ready until after MEM
        ADD R4, R1, R5  [ID]  ← needs R1 NOW — must wait 1 cycle

    === FLUSH (Emergency Stop) ===
    "WRONG WAY! Throw out everything!" A branch was mispredicted, so
    the instructions that were fetched after it are WRONG. We must
    discard them (replace with bubbles) and restart from the correct PC.

        BEQ R1, R2, target  [EX]  ← discovers branch IS taken
        wrong_instr_1        [ID]  ← FLUSH (replace with bubble)
        wrong_instr_2        [IF]  ← FLUSH (replace with bubble)
    """

    NONE = "none"
    FORWARD_FROM_EX = "forward_ex"
    FORWARD_FROM_MEM = "forward_mem"
    STALL = "stall"
    FLUSH = "flush"


# ---------------------------------------------------------------------------
# HazardResult — the complete verdict from hazard detection
# ---------------------------------------------------------------------------


@dataclass
class HazardResult:
    """Complete result from hazard detection — may include multiple details.

    === Why a Structured Result? ===

    A simple "stall or not" boolean isn't enough. The pipeline needs to know:
    - WHAT action to take (forward? stall? flush?)
    - The forwarded VALUE (if forwarding)
    - WHERE it came from (for debugging: "forwarded from EX" vs "from MEM")
    - HOW MANY cycles to stall (usually 1, but could be more)
    - HOW MANY stages to flush (branch misprediction flushes IF and ID)
    - WHY (human-readable explanation for debugging and learning)

    === Examples ===

    No hazard:
        HazardResult(action=HazardAction.NONE, reason="no dependencies")

    RAW hazard resolved by forwarding from EX:
        HazardResult(
            action=HazardAction.FORWARD_FROM_EX,
            forwarded_value=42,
            forwarded_from="EX",
            reason="R1 produced by ADD in EX, forwarded to SUB in ID",
        )

    Load-use stall:
        HazardResult(
            action=HazardAction.STALL,
            stall_cycles=1,
            reason="R1 loaded by LW in EX, needed by ADD in ID — must wait",
        )

    Branch misprediction flush:
        HazardResult(
            action=HazardAction.FLUSH,
            flush_count=2,
            reason="BEQ mispredicted: predicted not-taken, actually taken",
        )
    """

    action: HazardAction = HazardAction.NONE
    forwarded_value: int | None = None
    forwarded_from: str = ""
    stall_cycles: int = 0
    flush_count: int = 0
    reason: str = ""
