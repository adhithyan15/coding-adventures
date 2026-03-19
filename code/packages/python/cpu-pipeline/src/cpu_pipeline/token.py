"""PipelineToken, PipelineStage, and PipelineConfig -- the building blocks.

A CPU pipeline is an assembly line for instructions. Instead of completing
one instruction fully before starting the next (like a single-cycle CPU),
a pipelined CPU overlaps instruction execution:

    Single-cycle (no pipeline):
    Instr 1: [IF][ID][EX][MEM][WB]
    Instr 2:                       [IF][ID][EX][MEM][WB]
    Throughput: 1 instruction every 5 cycles

    Pipelined:
    Instr 1: [IF][ID][EX][MEM][WB]
    Instr 2:     [IF][ID][EX][MEM][WB]
    Instr 3:         [IF][ID][EX][MEM][WB]
    Throughput: 1 instruction every 1 cycle (after filling)

This module defines:
  - StageCategory: what kind of work a stage does (fetch, decode, etc.)
  - PipelineStage: a named stage with a category
  - PipelineToken: a unit of work (instruction) flowing through the pipeline
  - PipelineConfig: the configuration describing how many stages and their order
"""

from __future__ import annotations

import copy
from dataclasses import dataclass, field
from enum import IntEnum

# =========================================================================
# StageCategory -- classifies pipeline stages by their function
# =========================================================================


class StageCategory(IntEnum):
    """Classifies pipeline stages by their function.

    Every stage in a pipeline does one of these five jobs, regardless of
    how many stages the pipeline has. A 5-stage pipeline has one stage per
    category. A 13-stage pipeline might have 2 fetch stages, 2 decode
    stages, 3 execute stages, etc.

    This classification is used for:
      - Determining which callback to invoke for each stage
      - Knowing where to insert stall bubbles
      - Knowing which stages to flush on a misprediction
    """

    FETCH = 0  # Reads instructions from the instruction cache
    DECODE = 1  # Decodes the instruction and reads registers
    EXECUTE = 2  # Performs computation (ALU, branch resolution)
    MEMORY = 3  # Accesses data memory (loads and stores)
    WRITEBACK = 4  # Writes results back to the register file

    def __str__(self) -> str:
        """Return a human-readable name for the stage category."""
        names = {
            StageCategory.FETCH: "fetch",
            StageCategory.DECODE: "decode",
            StageCategory.EXECUTE: "execute",
            StageCategory.MEMORY: "memory",
            StageCategory.WRITEBACK: "writeback",
        }
        return names.get(self, "unknown")


# =========================================================================
# PipelineStage -- definition of a single stage in the pipeline
# =========================================================================


@dataclass
class PipelineStage:
    """Defines a single stage in the pipeline.

    A stage has a short name (used in diagrams), a description (for humans),
    and a category (for the pipeline to know what callback to invoke).

    Example stages::

        PipelineStage(name="IF",  description="Instruction Fetch",
                      category=StageCategory.FETCH)
        PipelineStage(name="EX1", description="Execute - ALU",
                      category=StageCategory.EXECUTE)
    """

    name: str  # Short name like "IF", "ID", "EX1"
    description: str  # Human-readable description
    category: StageCategory  # What kind of work this stage does

    def __str__(self) -> str:
        """Return the stage name for display in diagrams."""
        return self.name


# =========================================================================
# PipelineToken -- a unit of work flowing through the pipeline
# =========================================================================


@dataclass
class PipelineToken:
    """Represents one instruction moving through the pipeline.

    Think of it as a tray on an assembly line. The tray starts empty at the
    IF stage, gets filled with decoded information at ID, gets computed
    results at EX, gets memory data at MEM, and delivers results at WB.

    Token Lifecycle:
        IF stage:  FetchFunc fills in PC and raw_instruction
        ID stage:  DecodeFunc fills in opcode, registers, control signals
        EX stage:  ExecuteFunc fills in alu_result, branch_taken, branch_target
        MEM stage: MemoryFunc fills in mem_data (for loads)
        WB stage:  WritebackFunc uses write_data to update register file

    Bubbles:
        A "bubble" is a special token that represents NO instruction. Bubbles
        are inserted when the pipeline stalls or flushes. A bubble flows
        through the pipeline like a normal token but does nothing at each
        stage. In hardware, a bubble is a NOP (no-operation) instruction.
    """

    # --- Instruction identity ---
    pc: int = 0  # Program counter -- memory address of this instruction
    raw_instruction: int = 0  # Raw instruction bits as fetched from memory
    opcode: str = ""  # Decoded instruction name (e.g., "ADD", "LDR")

    # --- Decoded operands (set by ID stage callback) ---
    rs1: int = -1  # First source register number (-1 means unused)
    rs2: int = -1  # Second source register number (-1 means unused)
    rd: int = -1  # Destination register number (-1 means unused)
    immediate: int = 0  # Sign-extended immediate value

    # --- Control signals (set by ID stage callback) ---
    reg_write: bool = False  # True if this instruction writes a register
    mem_read: bool = False  # True if this instruction reads from data memory
    mem_write: bool = False  # True if this instruction writes to data memory
    is_branch: bool = False  # True if this is a branch instruction
    is_halt: bool = False  # True if this is a halt/stop instruction

    # --- Computed values (filled during execution) ---
    alu_result: int = 0  # Output of the ALU in the EX stage
    mem_data: int = 0  # Data read from memory in the MEM stage
    write_data: int = 0  # Final value to write to the destination register
    branch_taken: bool = False  # True if the branch was actually taken
    branch_target: int = 0  # Actual branch target address

    # --- Pipeline metadata ---
    is_bubble: bool = False  # True if this is a NOP/bubble
    stage_entered: dict[str, int] = field(default_factory=dict)
    forwarded_from: str = ""  # Stage that provided a forwarded value

    def __str__(self) -> str:
        """Return a human-readable representation.

        Bubbles display as "---" (like empty slots on the assembly line).
        Normal tokens display their opcode and PC.
        """
        if self.is_bubble:
            return "---"
        if self.opcode:
            return f"{self.opcode}@{self.pc}"
        return f"instr@{self.pc}"

    def clone(self) -> PipelineToken:
        """Return a deep copy of the token.

        This is necessary because tokens are passed between pipeline stages
        via pipeline registers. Each register holds its own copy so that
        modifying a token in one stage does not affect the copy in the
        pipeline register.
        """
        cloned = copy.copy(self)
        # Deep copy the stage_entered dict
        cloned.stage_entered = dict(self.stage_entered)
        return cloned


def new_bubble() -> PipelineToken:
    """Create a new bubble token.

    A bubble is a "do nothing" instruction that occupies a pipeline stage
    without performing any useful work. It is the pipeline equivalent of
    a "no-op" on an assembly line.
    """
    return PipelineToken(is_bubble=True)


def new_token() -> PipelineToken:
    """Create a new empty token with default register values.

    The token starts with all register fields set to -1 (unused) and
    all control signals set to false. The fetch callback will fill in
    the PC and raw instruction; the decode callback fills in everything else.
    """
    return PipelineToken()


# =========================================================================
# PipelineConfig -- configuration for the pipeline
# =========================================================================


@dataclass
class PipelineConfig:
    """Holds the configuration for a pipeline.

    The key insight: a pipeline's behavior is determined entirely by its
    stage configuration and execution width. Everything else (instruction
    semantics, hazard handling) is injected via callbacks.
    """

    stages: list[PipelineStage] = field(default_factory=list)
    execution_width: int = 1  # Width 1 = scalar, Width > 1 = superscalar

    def num_stages(self) -> int:
        """Return the number of stages in the pipeline."""
        return len(self.stages)

    def validate(self) -> None:
        """Check that the configuration is well-formed.

        Rules:
          - Must have at least 2 stages (a 1-stage "pipeline" is not a pipeline)
          - Execution width must be at least 1
          - All stage names must be unique
          - There must be at least one fetch stage and one writeback stage

        Raises:
            ValueError: If any validation rule is violated.
        """
        if len(self.stages) < 2:
            raise ValueError(
                f"pipeline must have at least 2 stages, got {len(self.stages)}"
            )
        if self.execution_width < 1:
            raise ValueError(
                f"execution width must be at least 1, got {self.execution_width}"
            )

        # Check for unique stage names
        seen: set[str] = set()
        for s in self.stages:
            if s.name in seen:
                raise ValueError(f"duplicate stage name: {s.name!r}")
            seen.add(s.name)

        # Check for required categories
        has_fetch = any(s.category == StageCategory.FETCH for s in self.stages)
        has_writeback = any(
            s.category == StageCategory.WRITEBACK for s in self.stages
        )
        if not has_fetch:
            raise ValueError("pipeline must have at least one fetch stage")
        if not has_writeback:
            raise ValueError("pipeline must have at least one writeback stage")


def classic_5_stage() -> PipelineConfig:
    """Return the standard 5-stage RISC pipeline configuration.

    This is the pipeline described in every computer architecture textbook:

        IF -> ID -> EX -> MEM -> WB

    It matches the MIPS R2000 (1985) and is the foundation for understanding
    all modern CPU pipelines.
    """
    return PipelineConfig(
        stages=[
            PipelineStage("IF", "Instruction Fetch", StageCategory.FETCH),
            PipelineStage("ID", "Instruction Decode", StageCategory.DECODE),
            PipelineStage("EX", "Execute", StageCategory.EXECUTE),
            PipelineStage("MEM", "Memory Access", StageCategory.MEMORY),
            PipelineStage("WB", "Write Back", StageCategory.WRITEBACK),
        ],
        execution_width=1,
    )


def deep_13_stage() -> PipelineConfig:
    """Return a 13-stage pipeline inspired by ARM Cortex-A78.

    Modern high-performance CPUs split the classic 5 stages into many
    sub-stages to enable higher clock frequencies. Each sub-stage does
    less work, so it completes faster, allowing a faster clock.

    The tradeoff: a branch misprediction now costs 10+ cycles instead of 2.
    """
    return PipelineConfig(
        stages=[
            PipelineStage("IF1", "Fetch 1 - TLB lookup", StageCategory.FETCH),
            PipelineStage("IF2", "Fetch 2 - cache read", StageCategory.FETCH),
            PipelineStage("IF3", "Fetch 3 - align/buffer", StageCategory.FETCH),
            PipelineStage("ID1", "Decode 1 - pre-decode", StageCategory.DECODE),
            PipelineStage("ID2", "Decode 2 - full decode", StageCategory.DECODE),
            PipelineStage("ID3", "Decode 3 - register read", StageCategory.DECODE),
            PipelineStage("EX1", "Execute 1 - ALU", StageCategory.EXECUTE),
            PipelineStage("EX2", "Execute 2 - shift/multiply", StageCategory.EXECUTE),
            PipelineStage("EX3", "Execute 3 - result select", StageCategory.EXECUTE),
            PipelineStage("MEM1", "Memory 1 - address calc", StageCategory.MEMORY),
            PipelineStage("MEM2", "Memory 2 - cache access", StageCategory.MEMORY),
            PipelineStage("MEM3", "Memory 3 - data align", StageCategory.MEMORY),
            PipelineStage("WB", "Write Back", StageCategory.WRITEBACK),
        ],
        execution_width=1,
    )
