# D03 — Hazard Detection and Forwarding

## Overview

Pipelining overlaps instruction execution — while one instruction is being
executed, the next is being decoded, and the one after that is being fetched.
This overlap creates **hazards**: situations where the pipeline cannot proceed
correctly because one instruction depends on the result of another that has not
finished yet.

The hazard detection unit and forwarding unit work together to keep the
pipeline running at full speed despite these dependencies. The hazard detection
unit identifies conflicts; the forwarding unit resolves them by short-circuiting
data paths; and when forwarding cannot help, the pipeline stalls (inserts
bubbles) or flushes (discards speculative instructions).

These units are pure combinational logic — they examine the current pipeline
state each cycle and produce control signals. They have no memory of their own
and do not need a clock.

## Layer Position

```
Core (D05)
├── Pipeline (D04) ← hazard/forwarding units control the pipeline
│   ├── IF → ID → EX → MEM → WB
│   │         ↑    │     │
│   │         └────┴─────┘  ← forwarding paths
│   │
│   ├── Hazard Detection Unit ← YOU ARE HERE
│   │   (examines pipeline registers, produces stall/flush signals)
│   │
│   └── Forwarding Unit ← YOU ARE HERE
│       (muxes forwarded values into ALU inputs)
│
├── Branch Predictor (D02) ← flush on misprediction
└── ...
```

**Depends on:** nothing (pure combinational logic)
**Used by:** `pipeline` (D04), `core` (D05)

## Key Concepts

### The Three Types of Hazards

#### 1. Data Hazards (RAW — Read After Write)

The most common hazard. Instruction B reads a register that instruction A
has not yet written back:

```
Instruction A:  ADD R1, R2, R3    ; writes R1 in WB stage (cycle 5)
Instruction B:  SUB R4, R1, R5    ; reads R1 in ID stage (cycle 3)

Pipeline without forwarding:

Cycle:    1     2     3     4     5
A:       IF    ID    EX   MEM   WB ← R1 written here
B:              IF    ID   EX   MEM
                      ↑
                      B reads R1 here, but A has not written it yet!
                      B gets the OLD value of R1 — WRONG RESULT.
```

This is called **RAW** (Read After Write) because B reads a value that A
writes, and the read happens before the write completes.

Other data hazard types (relevant for out-of-order execution, future work):
- **WAR** (Write After Read): B writes a register that A reads — only a problem
  with out-of-order execution
- **WAW** (Write After Write): B writes a register that A also writes — only a
  problem with out-of-order execution

#### 2. Control Hazards

A branch instruction changes the program counter, but the pipeline has already
fetched instructions from the "wrong" path:

```
Instruction A:  BEQ R1, R2, target   ; branch if R1 == R2
Instruction B:  ADD R3, R4, R5       ; fetched from PC+4 (sequential)
Instruction C:  SUB R6, R7, R8       ; fetched from PC+8

Pipeline:
Cycle:    1     2     3     4     5
A(BEQ):  IF    ID    EX   MEM   WB
B(ADD):         IF    ID    ←── if branch is TAKEN, B and C are wrong!
C(SUB):               IF   ←── these must be flushed (replaced with NOPs)

BEQ is resolved in EX (cycle 3). If taken, B and C are in the pipeline
but should not execute. The hazard unit must FLUSH them.
```

#### 3. Structural Hazards

Two instructions need the same hardware resource at the same time:

```
Cycle:    1     2     3     4     5
A:       IF    ID    EX   MEM   WB
                           ↑     ↑
                           │     └── A writes to register file
                           │
B:              IF    ID    EX   MEM
                      ↑
                      └── B reads from register file

If the register file has only one read port and one write port,
A writing and D reading in the same cycle is fine (different ports).
But if two instructions both need the memory port in MEM stage,
that is a structural hazard.
```

Modern CPUs avoid most structural hazards through hardware duplication:
- Split L1 cache (L1I + L1D) eliminates IF/MEM memory conflicts
- Multiple register file ports allow simultaneous read and write
- Multiple execution units allow parallel computation

Our implementation detects structural hazards when the pipeline configuration
has limited resources.

### Solution 1: Data Forwarding (Bypassing)

The key insight: the result of instruction A is **computed** in the EX stage
(cycle 3), but not **written back** to the register file until WB (cycle 5).
Why make B wait until cycle 5 when the value is available in cycle 3?

**Forwarding** adds hardware paths that bypass the register file, routing the
result directly from where it is produced to where it is needed:

```
Without forwarding (stall for 2 cycles):

Cycle:    1     2     3     4     5     6     7
A(ADD):  IF    ID    EX   MEM   WB
B(SUB):         IF    ID  stall stall  ID    EX   ← delayed by 2 cycles
                       ↑                ↑
                       needs R1         R1 now available from register file

With forwarding (no stalls):

Cycle:    1     2     3     4     5
A(ADD):  IF    ID    EX   MEM   WB
B(SUB):         IF    ID    EX   MEM
                       ↑    ↑
                       │    └── forward from EX/MEM register
                       └── R1 needed here

The forwarding unit detects that:
  - A (in EX stage) writes to R1
  - B (in ID stage) reads R1
  → Route A's EX output directly to B's ALU input
```

There are multiple forwarding paths, each named by the source and destination
pipeline stages:

```
Forwarding Paths:

                    ┌───────────────────────────────┐
                    │        Forwarding MUXes        │
                    └───────┬───────────────┬───────┘
                            │               │
        ┌──────┐   ┌──────┐│  ┌──────┐    │┌──────┐   ┌──────┐
        │  IF  │──→│  ID  │├─→│  EX  │────┤│ MEM  │──→│  WB  │
        └──────┘   └──────┘│  └──────┘    │└──────┘   └──────┘
                           │       │       │    │
                           │       └───────┘    │
                           │    EX-to-EX fwd    │
                           │                    │
                           └────────────────────┘
                              MEM-to-EX fwd

EX-to-EX forwarding:  Forward result from EX/MEM pipeline register to EX input
                       (1-cycle-old result, most common path)

MEM-to-EX forwarding:  Forward result from MEM/WB pipeline register to EX input
                        (2-cycle-old result, needed when EX-to-EX is not possible)
```

### Solution 2: Stalling (Pipeline Bubble)

Some hazards cannot be solved by forwarding. The classic case: **load-use
hazard.** A LOAD instruction reads data from memory in the MEM stage, but
the dependent instruction needs the value in the EX stage — one cycle earlier
than forwarding can deliver:

```
Instruction A:  LDR R1, [R2]      ; R1 available after MEM stage (cycle 4)
Instruction B:  ADD R3, R1, R4    ; needs R1 in EX stage (cycle 3)

Cycle:    1     2     3     4     5     6
A(LDR):  IF    ID    EX   MEM   WB
                           ↑
                           R1 is loaded from memory HERE

B(ADD):         IF    ID  stall  EX   MEM
                      ↑          ↑
                      needs R1   NOW we can forward from MEM/WB register

The hazard detection unit inserts a BUBBLE (NOP) into the pipeline:
- Freeze IF and ID stages (do not advance)
- Insert NOP into EX stage
- After one stall cycle, forwarding from MEM stage can deliver R1
```

This costs exactly 1 cycle. The pipeline inserts a "bubble" — an empty
instruction that occupies the EX stage for one cycle while the LOAD completes:

```
Cycle:    1     2     3     4     5     6
A(LDR):  IF    ID    EX   MEM   WB
Bubble:                    NOP   ---   ---
B(ADD):         IF    ID    ID    EX   MEM    ← ID repeated (stalled)
C(SUB):               IF    IF    ID    EX    ← IF repeated (stalled)
```

### Solution 3: Flushing

When a branch is mispredicted, all instructions fetched after the branch are
wrong and must be discarded:

```
Cycle:    1     2     3     4     5
BEQ:     IF    ID    EX   MEM   WB
                      ↑
                      Branch resolved: TAKEN (but we predicted not-taken)

Inst X:         IF    ID ← WRONG instruction, must be flushed
Inst Y:               IF ← WRONG instruction, must be flushed

After flush:
Cycle:    1     2     3     4     5     6     7
BEQ:     IF    ID    EX   MEM   WB
Inst X:         IF   NOP  NOP   NOP   NOP    ← replaced with bubble
Inst Y:               NOP NOP   NOP   NOP    ← replaced with bubble
Target:                    IF    ID    EX     ← correct instruction fetched
```

The flush signal:
1. Replaces pipeline register contents in IF/ID and ID/EX with NOPs
2. Redirects the PC to the correct target address
3. The pipeline resumes from the correct path

The cost of a flush is the **misprediction penalty** — equal to the number of
pipeline stages between IF and the stage where the branch is resolved. For a
5-stage pipeline resolving branches in EX, that is 2 cycles. For a 13-stage
pipeline, it can be 10+ cycles.

## Public API

```python
from dataclasses import dataclass
from enum import Enum
from typing import Optional

class HazardType(Enum):
    NONE = "none"
    DATA_RAW = "data_raw"          # Read After Write
    LOAD_USE = "load_use"          # Load followed by use (subset of RAW)
    CONTROL = "control"            # Branch misprediction
    STRUCTURAL = "structural"      # Resource conflict

class HazardAction(Enum):
    NONE = "none"                  # No hazard — proceed normally
    FORWARD_EX_EX = "forward_ex"   # Forward from EX/MEM register to EX input
    FORWARD_MEM_EX = "forward_mem" # Forward from MEM/WB register to EX input
    STALL = "stall"                # Insert bubble, freeze earlier stages
    FLUSH = "flush"                # Discard speculative instructions

@dataclass
class HazardEvent:
    """A detected hazard and the action to resolve it."""
    hazard_type: HazardType
    action: HazardAction
    source_stage: str              # Pipeline stage producing the value
    dest_stage: str                # Pipeline stage consuming the value
    register: int                  # Register causing the hazard
    description: str               # Human-readable explanation

@dataclass
class ForwardingPath:
    """A data forwarding path that resolves a RAW hazard."""
    source_stage: str              # "EX" or "MEM"
    source_register: int           # Which register is being forwarded
    value: int                     # The forwarded value
    dest_input: str                # "alu_input_a" or "alu_input_b"

@dataclass
class PipelineRegisters:
    """
    Snapshot of all pipeline register contents.

    The hazard detection unit examines these each cycle to find conflicts.
    This is ISA-independent — it only looks at register numbers, not
    instruction semantics.
    """
    # IF/ID register
    if_id_pc: int = 0
    if_id_instruction: int = 0     # Raw instruction bits

    # ID/EX register
    id_ex_rs1: int = -1            # Source register 1 (-1 = none)
    id_ex_rs2: int = -1            # Source register 2 (-1 = none)
    id_ex_rd: int = -1             # Destination register (-1 = none)
    id_ex_reg_write: bool = False  # Does this instruction write a register?
    id_ex_mem_read: bool = False   # Is this a load instruction?
    id_ex_is_branch: bool = False  # Is this a branch instruction?

    # EX/MEM register
    ex_mem_rd: int = -1
    ex_mem_reg_write: bool = False
    ex_mem_mem_read: bool = False
    ex_mem_alu_result: int = 0     # Result computed in EX stage
    ex_mem_branch_taken: bool = False
    ex_mem_branch_target: int = 0

    # MEM/WB register
    mem_wb_rd: int = -1
    mem_wb_reg_write: bool = False
    mem_wb_read_data: int = 0      # Data loaded from memory
    mem_wb_alu_result: int = 0     # ALU result (passed through)

@dataclass
class HazardStats:
    """Statistics about pipeline hazards."""
    total_cycles: int = 0
    stall_cycles: int = 0
    flush_cycles: int = 0
    forwards_ex_ex: int = 0
    forwards_mem_ex: int = 0
    data_hazards: int = 0
    control_hazards: int = 0
    structural_hazards: int = 0

    @property
    def stall_rate(self) -> float:
        if self.total_cycles == 0:
            return 0.0
        return self.stall_cycles / self.total_cycles


class HazardDetectionUnit:
    """
    Detects pipeline hazards by examining pipeline register contents.

    This is pure combinational logic — no internal state between cycles.
    Each call to detect() examines the current pipeline state and returns
    the appropriate action.
    """

    def detect(self, pipeline_regs: PipelineRegisters) -> list[HazardEvent]:
        """
        Examine pipeline registers and detect all hazards.

        Returns a list of HazardEvents, each describing a hazard and
        the recommended action (forward, stall, or flush).

        Detection rules (checked in priority order):

        1. LOAD-USE hazard:
           if ID/EX.mem_read AND
              ID/EX.rd == IF/ID.rs1 OR ID/EX.rd == IF/ID.rs2
           → STALL (cannot forward from MEM before it completes)

        2. EX forwarding:
           if EX/MEM.reg_write AND EX/MEM.rd != 0 AND
              EX/MEM.rd == ID/EX.rs1 OR EX/MEM.rd == ID/EX.rs2
           → FORWARD from EX/MEM

        3. MEM forwarding:
           if MEM/WB.reg_write AND MEM/WB.rd != 0 AND
              NOT (EX/MEM forwarding already covers this) AND
              MEM/WB.rd == ID/EX.rs1 OR MEM/WB.rd == ID/EX.rs2
           → FORWARD from MEM/WB

        4. Control hazard:
           if EX/MEM.branch_taken != predicted
           → FLUSH IF/ID and ID/EX stages
        """
        ...

    @property
    def stats(self) -> HazardStats:
        """Return cumulative hazard statistics."""
        ...

    def reset(self) -> None:
        """Reset statistics."""
        ...


class ForwardingUnit:
    """
    Determines forwarding paths for the current cycle.

    Given the pipeline registers, returns which values should be forwarded
    and to which ALU inputs. The pipeline uses these to MUX the correct
    values into the execution stage.
    """

    def resolve(self, pipeline_regs: PipelineRegisters) -> list[ForwardingPath]:
        """
        Determine all active forwarding paths.

        Returns a list of ForwardingPaths. Each path specifies:
        - Where the value comes from (EX/MEM or MEM/WB register)
        - Which ALU input it should be routed to
        - The value itself

        The pipeline MUXes use these paths to override register file reads.
        """
        ...
```

## Data Structures

### Pipeline Control Signals

The hazard detection unit produces control signals that the pipeline consumes:

```python
@dataclass
class PipelineControl:
    """Control signals produced by hazard detection for the pipeline."""
    pc_write: bool = True          # Allow PC to advance (False = stall IF)
    if_id_write: bool = True       # Allow IF/ID register to update (False = stall ID)
    id_ex_flush: bool = False      # Replace ID/EX contents with NOP (bubble)
    if_id_flush: bool = False      # Replace IF/ID contents with NOP (flush)
    pc_source: str = "normal"      # "normal" (PC+4), "branch" (branch target),
                                    # "predictor" (predicted target)
```

### Hazard Detection Truth Table

The complete decision logic, expressed as a truth table:

```
Condition                                          Action
───────────────────────────────────────────────────────────────────
ID/EX.mem_read AND                                 STALL
  (ID/EX.rd == IF/ID.rs1 OR ID/EX.rd == IF/ID.rs2)  (load-use)

EX/MEM.reg_write AND EX/MEM.rd != 0 AND           FORWARD
  (EX/MEM.rd == ID/EX.rs1 OR EX/MEM.rd == ID/EX.rs2)  (EX→EX)

MEM/WB.reg_write AND MEM/WB.rd != 0 AND           FORWARD
  NOT already forwarded from EX/MEM AND              (MEM→EX)
  (MEM/WB.rd == ID/EX.rs1 OR MEM/WB.rd == ID/EX.rs2)

EX/MEM.branch_taken != prediction                  FLUSH
                                                     (misprediction)

Two instructions both need MEM port                STALL
  (structural hazard, if not split cache)            (structural)

None of the above                                  NONE
                                                     (proceed)
```

## Test Strategy

### Data Hazard Tests (RAW)

- **No hazard**: two independent instructions (no shared registers) → no action
- **EX-to-EX forward**: `ADD R1, R2, R3` then `SUB R4, R1, R5` → forward R1
  from EX/MEM to ALU input A
- **MEM-to-EX forward**: `ADD R1, R2, R3` then NOP then `SUB R4, R1, R5` →
  forward R1 from MEM/WB
- **Double forward**: `ADD R1, ...` then `ADD R1, ...` then `SUB R4, R1, R5` →
  should forward from EX/MEM (most recent), not MEM/WB
- **Both inputs**: `ADD R1, R2, R3` then `ADD R4, R1, R1` → forward R1 to
  both ALU inputs
- **Register 0**: writes to R0 should never trigger forwarding (R0 is hardwired
  to zero in RISC-V; configurable)

### Load-Use Hazard Tests

- **Load-use stall**: `LDR R1, [R2]` then `ADD R3, R1, R4` → stall for 1 cycle
- **Load + 1 gap**: `LDR R1, [R2]` then `NOP` then `ADD R3, R1, R4` →
  forward from MEM/WB, no stall
- **Load + 2 gap**: `LDR R1, [R2]` then `NOP` then `NOP` then `ADD R3, R1, R4` →
  no hazard (value in register file)

### Control Hazard Tests

- **Branch taken (predicted not-taken)**: BEQ resolved as taken → flush IF/ID and ID/EX
- **Branch not-taken (predicted taken)**: BEQ resolved as not-taken → flush
- **Branch correctly predicted**: no flush needed
- **Flush produces bubbles**: verify flushed stages contain NOP-equivalent

### Structural Hazard Tests

- **No structural hazard**: with split L1 cache, IF and MEM can access memory
  simultaneously
- **Structural hazard**: with unified cache, IF and MEM in same cycle → stall

### Forwarding Unit Tests

- **No forwarding**: independent instructions → empty forwarding path list
- **Single forward path**: one dependency → one ForwardingPath returned
- **Multiple forward paths**: dependencies on both ALU inputs → two paths
- **Forward value correctness**: verify the forwarded value matches the ALU result
  or memory read data
- **Priority**: EX/MEM forwarding takes precedence over MEM/WB for same register

### Statistics Tests

- **Stall counting**: verify stall_cycles increments on each stall
- **Forward counting**: verify forwards_ex_ex and forwards_mem_ex counts
- **Flush counting**: verify flush_cycles reflects misprediction penalties
- **Stall rate calculation**: verify stall_rate = stall_cycles / total_cycles

### Integration with Pipeline

- **Sequence of dependent instructions**: chain of ADD R1, ... ; ADD R2, R1, ... ;
  ADD R3, R2, ... — verify all forwarding paths activate correctly
- **Mixed hazards**: load-use followed by branch → stall then possible flush
- **Real program trace**: bubble sort inner loop — verify correct execution with
  forwarding and stalls

## Future Extensions

- **WAR and WAW hazards**: needed for out-of-order execution (register renaming)
- **Scoreboard**: track which registers are "in flight" for multi-cycle operations
- **Reservation stations**: Tomasulo's algorithm for out-of-order execution
- **Memory disambiguation**: detect when loads and stores to different addresses
  can be reordered
- **Speculative forwarding**: forward values speculatively, roll back on misspeculation
- **Multi-issue hazard detection**: detect hazards across multiple instructions
  issued in the same cycle (superscalar)
