"""Pipeline — the fetch-decode-execute cycle that drives every CPU.

=== What is a pipeline? ===

Every CPU operates by repeating three steps over and over:

    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │  FETCH  │ ──→ │ DECODE  │ ──→ │ EXECUTE │ ──→ (repeat)
    └─────────┘     └─────────┘     └─────────┘

1. FETCH:   Read the next instruction from memory at the address stored
            in the Program Counter (PC). The instruction is just a number
            — a pattern of bits that encodes what operation to perform.

2. DECODE:  Figure out what those bits mean. Which operation is it? (ADD?
            LOAD? BRANCH?) Which registers are involved? Is there an
            immediate value encoded in the instruction?

3. EXECUTE: Perform the operation. This might mean sending values through
            the ALU (for arithmetic), reading/writing memory (for loads
            and stores), or changing the PC (for branches/jumps).

After execution, the PC is updated (usually PC += 4 for 32-bit instruction
sets) and the cycle repeats.

=== Why is it called a "pipeline"? ===

In simple CPUs (like ours), these three stages happen one after another
for each instruction. But in modern CPUs, they overlap — while one
instruction is being executed, the next one is being decoded, and the one
after that is being fetched. This is called "pipelining" and it's how
CPUs achieve high throughput.

Think of it like a laundry pipeline:
  - Simple: wash shirt 1, dry shirt 1, fold shirt 1, THEN wash shirt 2...
  - Pipelined: while shirt 1 is drying, start washing shirt 2.
               While shirt 2 is drying and shirt 1 is being folded,
               start washing shirt 3.

Our simulator starts with a simple non-pipelined design (one instruction
fully completes before the next begins) but exposes the pipeline stages
visibly so you can see what happens at each step.

=== Pipeline hazards (future) ===

Pipelining introduces problems called "hazards":
  - Data hazard: instruction 2 needs the result of instruction 1, but
    instruction 1 hasn't finished yet
  - Control hazard: a branch instruction changes the PC, so the
    instructions we already fetched are wrong (pipeline "flush")
  - Structural hazard: two instructions need the same hardware unit
    at the same time

These are fascinating problems that we'll explore as we add pipelining.
"""

from dataclasses import dataclass, field
from enum import Enum


class PipelineStage(Enum):
    """The three stages of the fetch-decode-execute cycle.

    Each instruction passes through these stages in order:

        FETCH → DECODE → EXECUTE

    In our simple (non-pipelined) CPU, only one stage is active at a time.
    In a pipelined CPU, up to three instructions can be in different stages
    simultaneously.
    """

    FETCH = "fetch"
    DECODE = "decode"
    EXECUTE = "execute"


@dataclass
class FetchResult:
    """What the FETCH stage produces.

    The fetch stage reads raw bytes from memory at the current PC address.
    It doesn't know what the bytes mean — that's the decode stage's job.

    Example:
        PC = 0x00000004
        The 4 bytes at that address are: 0x00 0x20 0x81 0xB3
        raw_instruction = 0x002081B3

    In the pipeline diagram:
        ┌──────────────────────────────────────┐
        │ FETCH                                │
        │ PC: 0x00000004                       │
        │ Read 4 bytes → 0x002081B3            │
        └──────────────────────────────────────┘
    """

    pc: int  # Program Counter value when the fetch occurred
    raw_instruction: int  # The raw 32-bit instruction word


@dataclass
class DecodeResult:
    """What the DECODE stage produces.

    The decode stage takes the raw instruction bits and extracts the
    meaningful fields: what operation, which registers, what immediate value.

    This is ISA-specific — RISC-V, ARM, WASM, and 4004 all decode
    differently. The CPU simulator provides this as a generic container;
    the ISA simulator fills in the details.

    Example (RISC-V 'add x3, x1, x2'):
        mnemonic = "add"
        fields = {"rd": 3, "rs1": 1, "rs2": 2, "funct3": 0, "funct7": 0}

    In the pipeline diagram:
        ┌──────────────────────────────────────┐
        │ DECODE                               │
        │ 0x002081B3 → add x3, x1, x2         │
        │ rd=3, rs1=1, rs2=2                   │
        └──────────────────────────────────────┘
    """

    mnemonic: str  # Human-readable instruction name
    fields: dict[str, int]  # Decoded fields (ISA-specific)
    raw_instruction: int  # The raw instruction (for display)


@dataclass
class ExecuteResult:
    """What the EXECUTE stage produces.

    The execute stage performs the actual operation and records what changed.

    Example (add x3, x1, x2 where x1=1, x2=2):
        description = "x3 = x1 + x2 = 1 + 2 = 3"
        registers_changed = {"x3": 3}
        memory_changed = {}
        next_pc = 12  (PC + 4, normal sequential execution)

    In the pipeline diagram:
        ┌──────────────────────────────────────┐
        │ EXECUTE                              │
        │ add x3, x1, x2                      │
        │ ALU: 1 + 2 = 3                      │
        │ Write x3 = 3                         │
        │ PC → 12                              │
        └──────────────────────────────────────┘
    """

    description: str  # Human-readable description of what happened
    registers_changed: dict[str, int]  # Which registers changed and to what
    memory_changed: dict[int, int]  # Which memory addresses changed
    next_pc: int  # The new program counter value
    halted: bool = False  # Did this instruction halt the CPU?


@dataclass
class PipelineTrace:
    """A complete record of one instruction's journey through the pipeline.

    This is the main data structure for visualization. It captures what
    happened at each stage, allowing you to see the full pipeline:

        ┌──────────────────────────────────────────────────────────┐
        │ Instruction #0                                          │
        ├──────────────┬──────────────────┬───────────────────────┤
        │ FETCH        │ DECODE           │ EXECUTE               │
        │ PC: 0x0000   │ addi x1, x0, 1  │ x1 = 0 + 1 = 1       │
        │ → 0x00100093 │ rd=1, rs1=0,     │ Write x1 = 1         │
        │              │ imm=1            │ PC → 4                │
        └──────────────┴──────────────────┴───────────────────────┘

    Example:
        >>> trace = PipelineTrace(
        ...     cycle=0,
        ...     fetch=FetchResult(pc=0, raw_instruction=0x00100093),
        ...     decode=DecodeResult(mnemonic="addi", fields={"rd": 1, "rs1": 0, "imm": 1}, raw_instruction=0x00100093),
        ...     execute=ExecuteResult(description="x1 = 0 + 1 = 1", registers_changed={"x1": 1}, memory_changed={}, next_pc=4),
        ... )
    """

    cycle: int  # Which instruction number this is (0, 1, 2, ...)
    fetch: FetchResult
    decode: DecodeResult
    execute: ExecuteResult
    register_snapshot: dict[str, int] = field(default_factory=dict)

    def format_pipeline(self) -> str:
        """Format this trace as a visual pipeline diagram.

        Returns a multi-line string showing all three stages side by side.

        Example output:
            ┌──────────────────────────────────────────────────┐
            │ Cycle 0                                          │
            ├───────────────┬─────────────────┬────────────────┤
            │ FETCH         │ DECODE          │ EXECUTE        │
            │ PC: 0x0000    │ addi x1, x0, 1  │ x1 = 1        │
            │ → 0x00100093  │ rd=1 rs1=0 i=1  │ PC → 4        │
            └───────────────┴─────────────────┴────────────────┘
        """
        fetch_lines = [
            "FETCH",
            f"PC: 0x{self.fetch.pc:04X}",
            f"-> 0x{self.fetch.raw_instruction:08X}",
        ]
        decode_lines = [
            "DECODE",
            self.decode.mnemonic,
            " ".join(f"{k}={v}" for k, v in self.decode.fields.items()),
        ]
        execute_lines = [
            "EXECUTE",
            self.execute.description,
            f"PC -> {self.execute.next_pc}",
        ]

        # Pad all columns to same number of lines
        max_lines = max(len(fetch_lines), len(decode_lines), len(execute_lines))
        for lines in (fetch_lines, decode_lines, execute_lines):
            while len(lines) < max_lines:
                lines.append("")

        # Format as columns
        col_width = 20
        result = [f"--- Cycle {self.cycle} ---"]
        for i in range(max_lines):
            f = fetch_lines[i].ljust(col_width)
            d = decode_lines[i].ljust(col_width)
            e = execute_lines[i].ljust(col_width)
            result.append(f"  {f} | {d} | {e}")

        return "\n".join(result)
