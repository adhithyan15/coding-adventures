# D04 — Configurable N-Stage Pipeline

## Overview

The pipeline is the central execution engine of the CPU core. Instead of
completing one instruction before starting the next (like a single-cycle CPU),
a pipelined CPU overlaps instruction execution — while one instruction is being
executed, the next is being decoded, and the one after that is being fetched.
This is the same principle as an assembly line: each worker (stage) performs one
task, then passes the work to the next worker.

This package implements a **configurable** pipeline — the number of stages, the
function of each stage, and the execution width are all parameters. This lets
you model anything from a simple 5-stage teaching pipeline to a deep 20-stage
design reminiscent of Intel Prescott.

## Layer Position

```
Core (D05)
├── Pipeline ← YOU ARE HERE
│   ├── IF → ID → EX → MEM → WB  (classic 5-stage)
│   │
│   ├── Branch Predictor (D02) ← provides predictions to IF stage
│   ├── Hazard Detection (D03) ← stall/flush signals for pipeline control
│   ├── Forwarding Unit (D03)  ← bypass paths between stages
│   └── Cache (D01)            ← IF reads from L1I, MEM reads/writes L1D
│
└── Register File              ← ID reads, WB writes
```

**Depends on:** `clock`, `branch-predictor` (D02), `hazard-detection` (D03), `cache` (D01)
**Used by:** `core` (D05)

## Key Concepts

### The Assembly Line Analogy

Imagine washing dishes. The single-cycle approach: you wash, rinse, dry, and
put away one dish completely before starting the next. If each step takes 1
minute, one dish takes 4 minutes, and 10 dishes take 40 minutes.

The pipelined approach: while person A washes dish 2, person B rinses dish 1.
While person A washes dish 3, person B rinses dish 2, and person C dries dish 1.
Each dish still takes 4 minutes (latency), but a new dish completes every 1
minute (throughput). 10 dishes take 4 + 9 = 13 minutes instead of 40.

```
Single-cycle (no pipeline):
Dish 1: [Wash][Rinse][Dry ][Store]
Dish 2:                             [Wash][Rinse][Dry ][Store]
Dish 3:                                                        [Wash]...
Time:    1     2     3     4     5     6     7     8     9    ...
Throughput: 1 dish every 4 minutes

Pipelined:
Dish 1: [Wash][Rinse][Dry ][Store]
Dish 2:       [Wash][Rinse][Dry ][Store]
Dish 3:             [Wash][Rinse][Dry ][Store]
Dish 4:                   [Wash][Rinse][Dry ][Store]
Time:    1     2     3     4     5     6     7
Throughput: 1 dish every 1 minute (after pipeline fills)
```

### The Classic 5-Stage Pipeline

The simplest practical pipeline divides the fetch-decode-execute cycle into 5
stages, each taking 1 clock cycle:

```
Stage 1: IF (Instruction Fetch)
├── Read instruction from L1I cache at address PC
├── Ask branch predictor for next-PC prediction
└── Pass instruction + PC to IF/ID pipeline register

Stage 2: ID (Instruction Decode)
├── Decode instruction: extract opcode, registers, immediate
├── Read source register values from register file
├── Extend immediate values to full width (sign-extend)
└── Pass decoded fields to ID/EX pipeline register

Stage 3: EX (Execute)
├── Perform ALU operation (add, sub, compare, shift, etc.)
├── Compute branch target address (PC + offset)
├── Resolve branch condition (compare register values)
├── Select ALU inputs (register value or forwarded value)
└── Pass result to EX/MEM pipeline register

Stage 4: MEM (Memory Access)
├── Load: read data from L1D cache
├── Store: write data to L1D cache
├── Non-memory instructions: pass ALU result through
└── Pass result to MEM/WB pipeline register

Stage 5: WB (Write Back)
├── Write result to destination register in register file
├── Select between ALU result and memory load data
└── Instruction complete
```

### Pipeline Registers

Between each pair of stages sits a **pipeline register** — a bank of D
flip-flops that captures the output of one stage and feeds it as input to the
next stage. Pipeline registers are clocked: they update on the rising edge of
the clock signal, ensuring that each stage sees a stable input for the entire
cycle.

```
┌──────┐  ┌───────┐  ┌──────┐  ┌────────┐  ┌──────┐  ┌────────┐  ┌──────┐
│  IF  │─→│ IF/ID │─→│  ID  │─→│ ID/EX  │─→│  EX  │─→│ EX/MEM │─→│ MEM  │─→...
└──────┘  │ reg   │  └──────┘  │ reg    │  └──────┘  │ reg    │  └──────┘
          └───────┘            └────────┘            └────────┘
               ↑                    ↑                     ↑
          Clock edge           Clock edge            Clock edge

On each rising clock edge:
  - IF/ID register captures IF stage output
  - ID/EX register captures ID stage output
  - EX/MEM register captures EX stage output
  - MEM/WB register captures MEM stage output

All registers update simultaneously — this is what makes pipelining work.
Stage N's output becomes stage N+1's input in the SAME clock tick.
```

### Throughput vs Latency

Pipelining improves **throughput** (instructions completed per unit time) but
does not improve **latency** (time for one instruction to complete):

```
                    Single-cycle     5-stage pipeline
Latency:            1 long cycle     5 short cycles
                    (= 5 ns)        (5 × 1 ns = 5 ns)

Throughput:         1 instr / 5 ns   1 instr / 1 ns
                                     (5x improvement!)

CPI (ideal):        1                1
Clock frequency:    200 MHz          1 GHz
                    (limited by      (limited by
                     longest path)    longest STAGE)
```

The clock period is determined by the **slowest stage** in the pipeline, not
the total path length. By breaking a long combinational path into shorter
stages, pipelining enables higher clock frequencies.

### Deeper Pipelines: The Tradeoff

More stages = shorter per-stage delay = higher clock frequency. This is why
Intel pushed to 20+ stages in the Pentium 4 era (2000-2006) and why ARM uses
13 stages in the Cortex-A78.

But deeper pipelines have costs:

```
Pipeline Depth vs Performance:

Stages  Clock    Misprediction  Branch      Net effect
        speed    penalty        frequency   (approximate)
──────────────────────────────────────────────────────────
5       1 GHz    2 cycles       15%         baseline
10      1.8 GHz  7 cycles       15%         ~1.4x faster
13      2.2 GHz  10 cycles      15%         ~1.5x faster
20      3.0 GHz  17 cycles      15%         ~1.3x faster (!)
31      3.8 GHz  28 cycles      15%         ~0.9x — SLOWER

At some point, the misprediction penalty eats all the clock speed gains.
This is why Intel backed off from deep pipelines after Prescott (31 stages).
Modern designs settle around 10-15 stages as the sweet spot.
```

### Real-World Pipeline Configurations

```
Processor                Year  Stages  Width    Notes
─────────────────────────────────────────────────────────────────
MIPS R2000               1985  5       1-wide   Classic textbook pipeline
ARM7TDMI                 1994  3       1-wide   Minimal pipeline (IF/ID+EX/WB)
ARM Cortex-A53           2012  8       2-wide   In-order, efficiency core
ARM Cortex-A78           2020  13      4-wide   Out-of-order, performance core
Apple M4 "Everest"       2024  ~14     8-wide   Very wide, aggressive OoO
Intel Pentium 4          2000  20      3-wide   Deep pipeline for high clock
Intel Prescott           2004  31      3-wide   Deepest x86 pipeline ever
AMD Zen 4                2022  ~19     6-wide   Modern high-performance
```

### Pipeline Stages in Our Model

Our pipeline is configurable. The default 5-stage configuration is:

```python
DEFAULT_STAGES = [
    PipelineStage("IF",  "Instruction Fetch"),
    PipelineStage("ID",  "Instruction Decode"),
    PipelineStage("EX",  "Execute"),
    PipelineStage("MEM", "Memory Access"),
    PipelineStage("WB",  "Write Back"),
]
```

But you can define custom stage configurations. For example, a deeper pipeline
might split EX into multiple stages:

```python
DEEP_STAGES = [
    PipelineStage("IF1", "Instruction Fetch 1"),
    PipelineStage("IF2", "Instruction Fetch 2"),
    PipelineStage("ID1", "Instruction Decode 1"),
    PipelineStage("ID2", "Instruction Decode 2"),
    PipelineStage("EX1", "Execute 1 — ALU"),
    PipelineStage("EX2", "Execute 2 — shift/multiply"),
    PipelineStage("EX3", "Execute 3 — result select"),
    PipelineStage("MEM1", "Memory Access 1"),
    PipelineStage("MEM2", "Memory Access 2"),
    PipelineStage("WB",  "Write Back"),
]
```

The pipeline does not interpret instructions — that is the ISA decoder's job.
The pipeline manages the flow of **pipeline tokens** through stages, handling
stalls, flushes, and forwarding. The actual semantics of each instruction are
provided by callback functions injected from the core.

## Public API

```python
from dataclasses import dataclass, field
from typing import Callable, Optional
from enum import Enum

@dataclass
class PipelineStage:
    """Definition of a single pipeline stage."""
    name: str                      # Short name (e.g., "IF", "EX1")
    description: str               # Human-readable description
    category: str = "execute"      # "fetch", "decode", "execute", "memory", "writeback"


@dataclass
class PipelineToken:
    """
    A unit of work flowing through the pipeline.

    Each token represents one instruction moving through the stages.
    It carries all decoded information and intermediate results.
    This is ISA-independent — the ISA decoder fills in the fields.
    """
    pc: int = 0                    # Program counter of this instruction
    raw_instruction: int = 0       # Raw instruction bits
    opcode: str = ""               # Decoded opcode name (for debugging)

    # Decoded operands (filled by ID stage callback)
    rs1: int = -1                  # Source register 1 (-1 = unused)
    rs2: int = -1                  # Source register 2 (-1 = unused)
    rd: int = -1                   # Destination register (-1 = unused)
    immediate: int = 0             # Immediate value (sign-extended)

    # Control signals (filled by ID stage callback)
    reg_write: bool = False        # Does this instruction write a register?
    mem_read: bool = False         # Does this instruction read memory?
    mem_write: bool = False        # Does this instruction write memory?
    is_branch: bool = False        # Is this a branch instruction?
    is_halt: bool = False          # Is this a halt instruction?

    # Computed values (filled during execution)
    alu_result: int = 0            # ALU output
    mem_data: int = 0              # Data read from memory
    write_data: int = 0            # Data to write to register file
    branch_taken: bool = False     # Was the branch actually taken?
    branch_target: int = 0         # Actual branch target address

    # Pipeline metadata
    is_bubble: bool = False        # True if this is a NOP/bubble
    stage_entered: dict = field(default_factory=dict)  # stage_name → cycle number
    forwarded_from: str = ""       # If forwarded, which stage provided the value


@dataclass
class PipelineConfig:
    """Configuration for the pipeline."""
    stages: list[PipelineStage]    # The stages in order
    execution_width: int = 1       # Instructions per cycle (1 = scalar, >1 = superscalar)

    @staticmethod
    def classic_5_stage() -> 'PipelineConfig':
        """Standard 5-stage RISC pipeline (IF, ID, EX, MEM, WB)."""
        ...

    @staticmethod
    def deep_13_stage() -> 'PipelineConfig':
        """13-stage pipeline inspired by ARM Cortex-A78."""
        ...


@dataclass
class PipelineSnapshot:
    """The complete state of the pipeline at one point in time."""
    cycle: int                                     # Current cycle number
    stages: dict[str, Optional[PipelineToken]]     # stage_name → token (or None)
    stalled: bool                                  # Is the pipeline stalled?
    flushing: bool                                 # Is a flush in progress?
    pc: int                                        # Current program counter


@dataclass
class PipelineStats:
    """Execution statistics."""
    total_cycles: int = 0
    instructions_completed: int = 0
    stall_cycles: int = 0
    flush_cycles: int = 0
    bubble_cycles: int = 0         # Cycles where a stage held a bubble

    @property
    def ipc(self) -> float:
        """Instructions per cycle."""
        if self.total_cycles == 0:
            return 0.0
        return self.instructions_completed / self.total_cycles

    @property
    def cpi(self) -> float:
        """Cycles per instruction (inverse of IPC)."""
        if self.instructions_completed == 0:
            return 0.0
        return self.total_cycles / self.instructions_completed


class Pipeline:
    """
    A configurable N-stage instruction pipeline.

    The pipeline manages the flow of PipelineTokens through stages.
    It does NOT interpret instructions — interpretation is delegated
    to callback functions provided by the core / ISA decoder.

    The pipeline is clock-driven: each call to step() advances all
    stages by one cycle, respecting stall and flush signals from
    the hazard detection unit.
    """

    def __init__(
        self,
        config: PipelineConfig,
        fetch_callback: Callable[[int], int],
        decode_callback: Callable[[int, PipelineToken], PipelineToken],
        execute_callback: Callable[[PipelineToken], PipelineToken],
        memory_callback: Callable[[PipelineToken], PipelineToken],
        writeback_callback: Callable[[PipelineToken], None],
        hazard_unit: Optional['HazardDetectionUnit'] = None,
        forwarding_unit: Optional['ForwardingUnit'] = None,
        branch_predictor: Optional['BranchPredictor'] = None,
    ) -> None:
        """
        Create a pipeline.

        Args:
            config: Pipeline configuration (stages, width)
            fetch_callback: (pc) → raw instruction bits
            decode_callback: (raw_instr, token) → decoded token
            execute_callback: (token) → token with ALU result
            memory_callback: (token) → token with memory data
            writeback_callback: (token) → None (writes to register file)
            hazard_unit: Optional hazard detection (stall/flush control)
            forwarding_unit: Optional forwarding (bypass paths)
            branch_predictor: Optional branch predictor (speculative fetch)
        """
        ...

    def step(self) -> PipelineSnapshot:
        """
        Advance the pipeline by one clock cycle.

        1. Check for hazards (stall/flush signals)
        2. If stalled: freeze earlier stages, insert bubble
        3. If flushing: replace speculative stages with bubbles
        4. Otherwise: advance each token to the next stage
        5. Fetch new instruction into IF stage
        6. Return snapshot of pipeline state

        All stage transitions happen simultaneously (on the clock edge).
        """
        ...

    def run(self, max_cycles: int = 10000) -> PipelineStats:
        """
        Run the pipeline until halt or max_cycles.

        Returns execution statistics.
        """
        ...

    @property
    def snapshot(self) -> PipelineSnapshot:
        """Return current pipeline state without advancing."""
        ...

    @property
    def stats(self) -> PipelineStats:
        """Return current execution statistics."""
        ...

    @property
    def is_halted(self) -> bool:
        """True if a halt instruction has reached WB stage."""
        ...

    def trace(self) -> list[PipelineSnapshot]:
        """Return the complete history of pipeline snapshots (for visualization)."""
        ...
```

## Data Structures

### Pipeline State Diagram

At any given cycle, the pipeline looks like this (5-stage example):

```
Cycle 7:

 ┌──────┐  ┌───────┐  ┌──────┐  ┌────────┐  ┌──────┐  ┌────────┐  ┌──────┐  ┌────────┐  ┌──────┐
 │  IF  │  │ IF/ID │  │  ID  │  │ ID/EX  │  │  EX  │  │ EX/MEM │  │ MEM  │  │ MEM/WB │  │  WB  │
 │      │  │       │  │      │  │        │  │      │  │        │  │      │  │        │  │      │
 │Inst 7│─→│Inst 6│─→│Inst 6│─→│Inst 5 │─→│Inst 5│─→│Inst 4 │─→│Inst 4│─→│Inst 3 │─→│Inst 3│
 └──────┘  └───────┘  └──────┘  └────────┘  └──────┘  └────────┘  └──────┘  └────────┘  └──────┘

 5 instructions in flight simultaneously:
   Inst 3: completing (WB stage)
   Inst 4: accessing memory (MEM stage)
   Inst 5: executing (EX stage)
   Inst 6: decoding (ID stage)
   Inst 7: being fetched (IF stage)
```

### Stall Visualization

When a load-use hazard is detected:

```
Cycle 7 (stall detected):

 ┌──────┐  ┌───────┐  ┌──────┐  ┌────────┐  ┌──────┐  ┌────────┐  ┌──────┐
 │  IF  │  │ IF/ID │  │  ID  │  │ ID/EX  │  │  EX  │  │ EX/MEM │  │ MEM  │
 │      │  │       │  │      │  │        │  │      │  │        │  │      │
 │Inst 6│  │Inst 5│  │Inst 5│  │BUBBLE  │  │LDR   │  │Inst 3 │  │Inst 3│
 │FROZEN│  │FROZEN│  │FROZEN│  │inserted│  │(load)│  │       │  │      │
 └──────┘  └───────┘  └──────┘  └────────┘  └──────┘  └────────┘  └──────┘
   │          │          │          ↑
   └──────────┴──────────┘          └── Bubble inserted here
   These stages are frozen              (hazard detection unit)
   (PC does not advance)

Cycle 8 (stall resolved, forwarding now possible):

 ┌──────┐  ┌───────┐  ┌──────┐  ┌────────┐  ┌──────┐  ┌────────┐  ┌──────┐
 │  IF  │  │ IF/ID │  │  ID  │  │ ID/EX  │  │  EX  │  │ EX/MEM │  │ MEM  │
 │      │  │       │  │      │  │        │  │      │  │        │  │      │
 │Inst 6│─→│Inst 5│─→│Inst 5│─→│Inst 5 │  │BUBBLE│  │LDR    │  │Inst 3│
 │      │  │      │  │      │  │       │  │      │  │(value │  │      │
 └──────┘  └───────┘  └──────┘  └────────┘  └──────┘  │ ready)│  └──────┘
                                    ↑                   │       │
                                    └───────────────────┘
                                     MEM-to-EX forwarding now works
```

## Test Strategy

### Basic Pipeline Tests

- **Single instruction**: one instruction flows through all 5 stages in 5 cycles
- **Steady state**: after pipeline fills, one instruction completes per cycle
- **Pipeline fill**: first instruction completes at cycle 5, second at cycle 6, etc.
- **Halt propagation**: halt instruction in IF eventually reaches WB and stops pipeline
- **Empty pipeline**: step() on empty pipeline with no program does not crash

### Throughput Tests

- **IPC = 1.0**: sequence of independent instructions → 1 instruction per cycle
- **IPC < 1.0**: dependent instructions with stalls → IPC below 1.0
- **CPI calculation**: verify CPI = total_cycles / instructions_completed

### Stall Tests

- **Load-use stall**: LDR then dependent ADD → pipeline stalls for 1 cycle
- **Stall freezes earlier stages**: verify IF and ID contents unchanged during stall
- **Bubble insertion**: verify EX stage receives a bubble during stall
- **Stall + forward**: after stall, forwarding delivers the value

### Flush Tests

- **Branch misprediction flush**: mispredicted branch → 2 stages flushed (5-stage)
- **Flush replaces with bubbles**: verify flushed stages contain bubble tokens
- **PC redirect**: after flush, IF fetches from correct target address
- **Flush penalty**: verify flush costs N-2 cycles (where N = stage of branch resolution)

### Forwarding Integration Tests

- **EX-to-EX forwarding**: dependent instructions with no stall
- **MEM-to-EX forwarding**: dependency 2 instructions apart
- **Forwarding record**: verify token's `forwarded_from` field is set correctly

### Configuration Tests

- **3-stage pipeline**: IF → ID+EX → MEM+WB (simplified, like ARM7)
- **5-stage pipeline**: classic RISC pipeline
- **10-stage pipeline**: deeper pipeline, verify longer misprediction penalty
- **Custom stages**: user-defined stage names and categories

### Trace and Snapshot Tests

- **Snapshot accuracy**: snapshot reflects actual pipeline contents
- **Trace completeness**: trace records every cycle's state
- **Cycle numbering**: snapshots have correct, incrementing cycle numbers

### Statistics Tests

- **Instruction count**: matches number of non-bubble completions
- **Stall cycle count**: matches number of stall events
- **Flush cycle count**: matches misprediction penalties
- **IPC calculation**: independent instructions → IPC near 1.0

## Future Extensions

- **Superscalar execution**: multiple instructions per cycle (execution_width > 1)
  - Requires multi-port register file, multiple ALUs, wider issue logic
  - Hazard detection across multiple in-flight instructions per stage
- **Superpipelining**: subdivide stages further for higher clock frequency
- **Out-of-order pipeline**: fetch in-order, execute out-of-order, commit in-order
  - Requires reorder buffer, reservation stations, register renaming
- **Multi-threaded pipeline**: simultaneous multi-threading (SMT / Hyper-Threading)
  - Multiple thread contexts share the same pipeline
  - Each thread has its own register file and PC
- **Pipeline visualization**: generate ASCII or HTML diagrams showing instruction flow
- **Variable-latency stages**: some instructions take multiple cycles in EX (e.g., multiply, divide)
  - Pipelined multiplier: 3-cycle latency but 1-cycle throughput
  - Non-pipelined divider: 30+ cycle latency, blocks the stage
