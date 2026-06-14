# CPU Architecture — Fetch, Decode, Execute, and Everything Around It

The CPU (Central Processing Unit) is the brain of a computer. But it's a
very simple brain — it can only do one thing: **read an instruction, figure
out what it means, do what it says, repeat.** The power of a computer comes
not from the complexity of individual operations (they are trivial), but from
doing billions of them per second.

This document covers the fetch-decode-execute cycle, registers, clock signals,
cache hierarchy, branch prediction, pipeline hazards, and their mitigations.

Reference implementations:
- CPU simulator: `code/packages/python/cpu-simulator/`
- Cache: `code/packages/python/cache/`
- Branch predictor: `code/packages/python/branch-predictor/`
- Hazard detection: `code/packages/python/hazard-detection/`
- Clock: `code/packages/python/clock/`

---

## Table of Contents

1. [The Fetch-Decode-Execute Cycle](#1-the-fetch-decode-execute-cycle)
2. [Registers and the Program Counter](#2-registers-and-the-program-counter)
3. [Clock Signals](#3-clock-signals)
4. [Cache Hierarchy](#4-cache-hierarchy)
5. [Branch Prediction](#5-branch-prediction)
6. [Pipeline Hazards](#6-pipeline-hazards)
7. [Forwarding and Stalling](#7-forwarding-and-stalling)
8. [The Python Implementation](#8-the-python-implementation)

---

## 1. The Fetch-Decode-Execute Cycle

Every instruction a CPU executes goes through three stages. This cycle
repeats for every single instruction, billions of times per second.

### The Three Stages

```
    +-------------+      +-------------+      +-------------+
    |   FETCH     | ---> |   DECODE    | ---> |   EXECUTE   |
    |             |      |             |      |             |
    | Read the    |      | What does   |      | Do the      |
    | next        |      | this binary |      | operation.  |
    | instruction |      | pattern     |      | Update      |
    | from memory |      | mean?       |      | registers   |
    | at address  |      |             |      | and memory. |
    | PC          |      |             |      |             |
    +-------------+      +-------------+      +-------------+
          |                                          |
          |              PC = PC + 4                 |
          |  (or branch target if jump)              |
          +<-----------------------------------------+
```

### Stage 1: FETCH

The CPU reads the instruction at the address stored in the **Program
Counter (PC)**. In a 32-bit instruction set (like RISC-V or ARM), this
means reading 4 bytes from memory.

```
    Memory:
    Address:  0x0000   0x0004   0x0008   0x000C
    Content: [instr 0] [instr 1] [instr 2] [instr 3]
                 ^
                 |
                 PC = 0x0000 (points here)
```

The raw instruction is just a 32-bit number. It has no meaning yet — it's
just bits. The fetch stage doesn't understand what it reads.

### Stage 2: DECODE

The decoder breaks the raw instruction into fields: opcode (what operation),
register numbers (which registers to read/write), and immediate values
(literal constants embedded in the instruction).

```
    Raw instruction: 0x00100093

    Decoded fields:
    +--------+-------+-------+--------+--------+--------+
    | funct7 |  rs2  |  rs1  | funct3 |   rd   | opcode |
    +--------+-------+-------+--------+--------+--------+
    | 0000000| 00001 | 00000 |  000   | 00001  |0010011 |
    +--------+-------+-------+--------+--------+--------+

    Meaning: ADDI x1, x0, 1   (add immediate: x1 = x0 + 1)
```

The decoder is ISA-specific (RISC-V, ARM, x86 all have different encodings).
In the Python implementation, the CPU accepts a pluggable decoder — the same
CPU shell can run RISC-V, ARM, or WASM instructions.

### Stage 3: EXECUTE

The executor performs the operation:
1. Reads source registers (e.g., read x0)
2. Sends values to the ALU (e.g., compute 0 + 1)
3. Writes the result to the destination register (e.g., x1 = 1)
4. Updates the PC (usually PC + 4, or branch target if jumping)

```
    Before:  x0 = 0, x1 = 0, PC = 0x0000
    Execute: ADDI x1, x0, 1
    After:   x0 = 0, x1 = 1, PC = 0x0004
```

### The Full Cycle (Worked Example)

```
    Program: Add 1 and 2, store in register 3.

    Memory:
    0x0000: ADDI x1, x0, 1    (x1 = 0 + 1 = 1)
    0x0004: ADDI x2, x0, 2    (x2 = 0 + 2 = 2)
    0x0008: ADD  x3, x1, x2   (x3 = 1 + 2 = 3)
    0x000C: HALT

    Cycle 0:
      FETCH:   Read 4 bytes at PC=0x0000 -> raw bits
      DECODE:  ADDI x1, x0, 1
      EXECUTE: x1 = 0 + 1 = 1, PC = 0x0004

    Cycle 1:
      FETCH:   Read 4 bytes at PC=0x0004 -> raw bits
      DECODE:  ADDI x2, x0, 2
      EXECUTE: x2 = 0 + 2 = 2, PC = 0x0008

    Cycle 2:
      FETCH:   Read 4 bytes at PC=0x0008 -> raw bits
      DECODE:  ADD x3, x1, x2
      EXECUTE: x3 = 1 + 2 = 3, PC = 0x000C

    Cycle 3:
      FETCH:   Read 4 bytes at PC=0x000C -> raw bits
      DECODE:  HALT
      EXECUTE: CPU halted.

    Final state: x1=1, x2=2, x3=3
```

---

## 2. Registers and the Program Counter

### What Are Registers?

Registers are tiny, ultra-fast storage locations inside the CPU. They hold
the values the CPU is currently working with. Accessing a register takes
1 clock cycle. Accessing main memory takes 100+ cycles.

```
    +-------------------------------------------------+
    |                CPU Register File                |
    +-------------------------------------------------+
    | x0  = 0 (hardwired zero)                       |
    | x1  = 1                                         |
    | x2  = 2                                         |
    | x3  = 3                                         |
    | ...                                             |
    | x15 = 0                                         |
    | PC  = 0x000C (program counter)                  |
    +-------------------------------------------------+

    Each register is N flip-flops in parallel (see logic-gates.md).
    A 32-bit register = 32 D flip-flops sharing one clock signal.
```

### The Program Counter (PC)

The PC is a special register that holds the memory address of the **next**
instruction to execute. After each instruction, the PC typically advances
by the instruction size (4 bytes for 32-bit instructions).

```
    Normal flow:    PC = PC + 4
    Branch taken:   PC = branch_target_address
    Jump:           PC = jump_target_address
    Function call:  save PC to link register, PC = function_address
    Return:         PC = saved value from link register
```

### The Instruction Register (IR)

The IR holds the raw instruction just fetched from memory. It's the input
to the decode stage. After decoding, the IR's contents are consumed and
it's free to receive the next instruction.

```
    FETCH:  Memory[PC] -> IR
    DECODE: IR -> {opcode, rd, rs1, rs2, immediate}
```

---

## 3. Clock Signals

Every sequential circuit in a computer is driven by a clock signal — a
square wave that alternates between 0 and 1.

### The Clock Waveform

```
    +--+  +--+  +--+  +--+  +--+
    |  |  |  |  |  |  |  |  |  |
  --+  +--+  +--+  +--+  +--+  +--

    ^     ^     ^     ^     ^
    |     |     |     |     |
    Rising edges: flip-flops capture data here
```

On each **rising edge** (0 to 1 transition), all flip-flops simultaneously
capture their inputs. This is what makes synchronous digital logic work:
everything happens in lockstep.

### Clock Frequency and Period

```
    Frequency:  How many cycles per second (measured in GHz)
    Period:     How long one cycle takes (= 1/frequency)

    Example:
    5 GHz clock = 5 billion cycles per second
    Period = 1 / 5,000,000,000 = 0.2 nanoseconds = 200 picoseconds
```

The clock period must be long enough for the **slowest signal path**
(critical path) to settle. If signals haven't stabilized when the next
rising edge arrives, the captured values will be wrong.

### Clock Generator

A clock generator produces the base clock signal. In real hardware, this is
a crystal oscillator. In our simulation, it's a simple toggle:

```python
    # From code/packages/python/clock/src/clock/clock.py
    # Each tick() call advances one half-cycle
    tick 0: value = 1  (rising edge)
    tick 1: value = 0  (falling edge)
    tick 2: value = 1  (rising edge)
    ...
```

### Clock Divider

A clock divider produces a slower clock from a faster one. A divide-by-2
circuit toggles on every rising edge of the input clock:

```
    Input clock:    +--+  +--+  +--+  +--+
                    |  |  |  |  |  |  |  |
                  --+  +--+  +--+  +--+  +--

    Divided by 2:   +-----+     +-----+
                    |     |     |     |
                  --+     +-----+     +-----

    Divided by 4:   +-----------+
                    |           |
                  --+           +-----------+
```

Clock dividers are used to create multiple clock domains within a chip.
The CPU core might run at 5 GHz while the memory interface runs at 1.6 GHz.

### Multi-Phase Clocks

Some designs use multi-phase clocks with overlapping waveforms:

```
    Phase 0:  +---+       +---+       +---+
              |   |       |   |       |   |
            --+   +-------+   +-------+   +---

    Phase 1:      +---+       +---+       +---
                  |   |       |   |       |   |
            ------+   +-------+   +-------+   +
```

Each phase activates different parts of the pipeline. This was common in
older designs (like the Intel 4004). Modern designs typically use a single
clock with edge-triggered flip-flops instead.

---

## 4. Cache Hierarchy

Main memory (DRAM) is slow — 100+ clock cycles to access. Caches are small,
fast memory buffers (SRAM) that keep copies of frequently accessed data
close to the CPU.

### The Hierarchy

```
    CPU Core
    +-------+
    | Regs  |  <- 1 cycle latency, ~1 KB
    +---+---+
        |
    +---v---+
    |  L1   |  <- 3-4 cycles, 32-64 KB
    +---+---+
        |
    +---v---+
    |  L2   |  <- 10-12 cycles, 256 KB - 1 MB
    +---+---+
        |
    +---v---+
    |  L3   |  <- 30-40 cycles, 8-64 MB (shared between cores)
    +---+---+
        |
    +---v---+
    | DRAM  |  <- 100-200 cycles, 8-128 GB
    +-------+
```

When the CPU needs data:
1. Check L1. If found (**hit**), return data in ~4 cycles.
2. If not in L1 (**miss**), check L2. If hit, copy to L1 and return.
3. If not in L2, check L3. If hit, copy to L2 and L1 and return.
4. If not in L3, fetch from DRAM. Copy to L3, L2, L1, and return.

### Address Decomposition

When the CPU accesses an address, the cache breaks it into three parts:

```
    32-bit address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
                     31..........14   13..............6    5..............0
```

- **Offset** (lowest bits): Which byte within the cache line? For 64-byte
  lines, this is 6 bits (2^6 = 64).

- **Set index** (middle bits): Which set should we look in? For 256 sets,
  this is 8 bits (2^8 = 256).

- **Tag** (highest bits): Which memory block is this? All remaining bits.

### Set Associativity

A cache set can hold multiple lines (called "ways"). The number of ways
determines the **associativity**:

```
    Direct-mapped (1-way):  Each address maps to exactly 1 slot
                            Fast but many conflicts

    4-way set-associative:  Each address maps to 1 set of 4 slots
                            Good balance of speed and hit rate

    Fully associative:      Any address can go in any slot
                            Best hit rate but expensive to search

    Example: 4-way set-associative with 256 sets

    Set 0:  [Line] [Line] [Line] [Line]   <- 4 ways
    Set 1:  [Line] [Line] [Line] [Line]
    Set 2:  [Line] [Line] [Line] [Line]
    ...
    Set 255: [Line] [Line] [Line] [Line]
```

### Cache Lines

Data is moved between cache and memory in fixed-size chunks called **cache
lines** (typically 64 bytes). Even if you only need 1 byte, the entire
64-byte line is fetched. This exploits **spatial locality**: if you access
address X, you'll probably access X+1, X+2, etc. soon.

```
    Cache Line (64 bytes):
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
    |B0|B1|B2|B3|B4|B5|B6|B7| ... (64 bytes total)   |
    +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
     ^                                              ^
     offset=0                                  offset=63
```

### LRU Replacement

When a cache set is full and a new line must be loaded, we must **evict**
one of the existing lines. The most common policy is **LRU** (Least Recently
Used): evict the line that hasn't been accessed for the longest time.

```
    4-way set, all full:

    Way 0: [Line A] last used: 100 cycles ago  <- evict this one (LRU)
    Way 1: [Line B] last used: 5 cycles ago
    Way 2: [Line C] last used: 20 cycles ago
    Way 3: [Line D] last used: 2 cycles ago
```

### Write-Back Policy

When the CPU writes to a cached line, there are two strategies:

- **Write-through:** Write to both cache and main memory immediately.
  Simple but slow (every write goes to DRAM).

- **Write-back:** Write only to cache. Mark the line as "dirty." Write to
  memory only when the line is evicted. Faster because writes are batched.

```
    Write-back flow:

    CPU writes to address X:
      1. Cache line updated, marked DIRTY
      2. (nothing else happens yet)

    Later, line is evicted (LRU):
      3. Because it's DIRTY, write it back to memory
      4. Load new line into the freed slot
```

Most modern caches use write-back because it dramatically reduces memory
bus traffic.

### Implementation Reference

See `code/packages/python/cache/src/cache/cache.py` for the configurable
cache implementation. The same class serves as L1, L2, or L3 — only the
configuration parameters (size, associativity, latency) differ.

---

## 5. Branch Prediction

### The Problem: Pipeline Stalls

In a pipelined CPU, multiple instructions are in-flight simultaneously:

```
    Time ->
    Cycle:     1       2       3       4       5

    Instr 1:  [FETCH] [DECODE] [EXEC]
    Instr 2:          [FETCH]  [DECODE] [EXEC]
    Instr 3:                   [FETCH]  [DECODE] [EXEC]
```

But what happens when instruction 1 is a **conditional branch** (like
"if x > 0, jump to address Y")? We don't know whether the branch is taken
until the EXECUTE stage. But by then, we've already started fetching
instructions 2 and 3.

If the branch IS taken, instructions 2 and 3 are **wrong** — they came from
the wrong path. We must flush them and restart from the correct address.

```
    Without prediction:

    Cycle:     1       2       3       4       5       6
    BEQ:      [FETCH] [DECODE] [EXEC]
    ???:              [STALL]  [STALL]  <- wasted cycles!
    Target:                            [FETCH] [DECODE] [EXEC]
```

Those 2 wasted cycles happen **every branch**. In typical code, ~20% of
instructions are branches. That's a massive performance loss.

### The Solution: Predict and Verify

Instead of waiting, the CPU **guesses** (predicts) whether the branch will
be taken. It starts fetching from the predicted path immediately. If the
guess was right, no time is lost. If wrong, we flush and restart — but
that's no worse than stalling.

### 1-Bit Predictor

The simplest predictor: remember whether the branch was taken last time,
and predict the same.

```
    State: 0 (predict NOT TAKEN) or 1 (predict TAKEN)

    On actual TAKEN:     state = 1
    On actual NOT TAKEN: state = 0
```

**Problem:** Consider a loop that runs 10 times:

```
    Iteration  1: predict NOT TAKEN -> actual TAKEN -> WRONG (state -> 1)
    Iteration  2: predict TAKEN     -> actual TAKEN -> correct
    ...
    Iteration  9: predict TAKEN     -> actual TAKEN -> correct
    Iteration 10: predict TAKEN     -> actual NOT TAKEN -> WRONG (state -> 0)

    Next call:
    Iteration  1: predict NOT TAKEN -> actual TAKEN -> WRONG (state -> 1)
```

Two mispredictions per loop invocation (beginning and end).

### 2-Bit Saturating Counter

Fixes the double-misprediction problem by requiring **two consecutive**
wrong predictions to flip the direction.

```
    Four states:

    00: STRONGLY NOT TAKEN  -> predict NOT TAKEN
    01: WEAKLY NOT TAKEN    -> predict NOT TAKEN
    10: WEAKLY TAKEN        -> predict TAKEN
    11: STRONGLY TAKEN      -> predict TAKEN
```

**State transitions:**

```
    On TAKEN outcome:     state = min(state + 1, 3)  (increment, saturate at 3)
    On NOT TAKEN outcome: state = max(state - 1, 0)  (decrement, saturate at 0)
```

**State transition diagram:**

```
              taken          taken          taken
    (sat) <-------- SNT <-------- WNT <-------- WT <-------- ST (sat)
          -------->     -------->     -------->     -------->
           not taken     not taken     not taken     not taken

    SNT = Strongly Not Taken (00)   predict: NOT TAKEN
    WNT = Weakly Not Taken   (01)   predict: NOT TAKEN
    WT  = Weakly Taken       (10)   predict: TAKEN
    ST  = Strongly Taken     (11)   predict: TAKEN
```

**Loop behavior (starting at WNT):**

```
    Iter  1: WNT -> predict NOT TAKEN -> actual TAKEN -> WRONG, state -> WT
    Iter  2: WT  -> predict TAKEN     -> actual TAKEN -> right, state -> ST
    ...
    Iter 10: ST  -> predict TAKEN     -> actual NOT TAKEN -> WRONG, state -> WT

    Next call:
    Iter  1: WT  -> predict TAKEN     -> actual TAKEN -> right! state -> ST
```

Only 1 misprediction on re-entry (vs 2 for 1-bit). The "weakly taken"
state absorbs the single not-taken at loop exit.

### Branch Target Buffer (BTB)

The prediction table tells us whether a branch is taken, but not WHERE to
jump. The **BTB** stores the target address of recently taken branches.

```
    BTB Table:
    +--------+------------------+
    | PC     | Target Address   |
    +--------+------------------+
    | 0x100  | 0x200            |  (last time PC=0x100 branched to 0x200)
    | 0x300  | 0x050            |
    | ...    | ...              |
    +--------+------------------+
```

When the predictor says "taken," the BTB provides the target address to
start fetching from — before the branch is even decoded.

### Historical Context

```
    Processor       | Predictor Type          | Table Size
    ----------------+-------------------------+-----------
    Alpha 21064     | 2-bit counters          | 2048
    Intel Pentium   | 2-bit + branch history  | 256
    Early ARM7      | 2-bit counters          | 64
    Modern (2020s)  | TAGE / Perceptron       | 64K+
```

Modern predictors achieve 95-99% accuracy by using tournament designs,
pattern history, and even neural-network-inspired approaches.

### Implementation Reference

See `code/packages/python/branch-predictor/` for implementations of:
- `static.py` — always-taken and always-not-taken predictors
- `one_bit.py` — 1-bit predictor
- `two_bit.py` — 2-bit saturating counter predictor
- `btb.py` — branch target buffer

---

## 6. Pipeline Hazards

A pipeline hazard is a situation where the next instruction cannot execute
in the next clock cycle because of a dependency or conflict.

### The Classic 5-Stage Pipeline

```
    IF -> ID -> EX -> MEM -> WB

    IF:  Instruction Fetch   (read instruction from memory)
    ID:  Instruction Decode  (read registers, detect hazards)
    EX:  Execute             (ALU computation)
    MEM: Memory Access       (load/store)
    WB:  Write Back          (write result to register file)
```

In each cycle, all 5 stages are active simultaneously, each working on a
different instruction:

```
    Cycle:    1     2     3     4     5     6     7

    Instr 1: [IF]  [ID]  [EX]  [MEM] [WB]
    Instr 2:       [IF]  [ID]  [EX]  [MEM] [WB]
    Instr 3:             [IF]  [ID]  [EX]  [MEM] [WB]
```

This achieves a throughput of ~1 instruction per cycle (IPC) once the
pipeline is full.

### 6.1 Data Hazards (RAW — Read After Write)

The most common hazard. A later instruction reads a register that an earlier
instruction hasn't finished writing yet.

```
    ADD R1, R2, R3    <- writes R1 in WB stage (cycle 5)
    SUB R4, R1, R5    <- reads R1 in ID stage (cycle 3)
                         ^-- R1 not written yet! Wrong value!
```

**Pipeline view:**

```
    Cycle:    1     2     3     4     5
    ADD R1:  [IF]  [ID]  [EX]  [MEM] [WB]  <- R1 written HERE
    SUB R4:        [IF]  [ID]               <- R1 read HERE (too early!)
                          ^
                          needs R1, but ADD hasn't written it yet
```

### 6.2 Control Hazards (Branch Misprediction)

When a branch is mispredicted, the instructions that were fetched after it
are from the wrong path and must be discarded.

```
    BEQ R1, R2, target    <- branch resolved in EX (cycle 3)
    wrong_instr_1          <- fetched assuming NOT TAKEN
    wrong_instr_2          <- fetched assuming NOT TAKEN

    If branch IS taken:
    -> FLUSH wrong_instr_1 and wrong_instr_2
    -> Restart fetch from target address
    -> 2 cycles wasted (the "branch penalty")
```

**Pipeline view:**

```
    Cycle:    1     2     3     4     5
    BEQ:     [IF]  [ID]  [EX]  <- branch resolved: TAKEN!
    wrong 1:       [IF]  [ID]  <- FLUSH! (replace with bubble)
    wrong 2:             [IF]  <- FLUSH! (replace with bubble)
    target:                    [IF]  [ID]  [EX]  ...
```

### 6.3 Structural Hazards (Resource Contention)

Two instructions need the same hardware resource at the same time.

```
    Example: only 1 memory port

    LOAD R1, [addr]   <- needs memory in MEM stage
    FETCH next instr  <- needs memory in IF stage
                         ^-- conflict! Both need memory at the same time
```

Modern CPUs avoid this with separate instruction and data caches (Harvard
architecture), multiple ALU units, and multi-ported register files. But
with a single floating-point unit, two FP instructions in adjacent pipeline
stages would still conflict.

---

## 7. Forwarding and Stalling

### Forwarding (Bypassing)

Instead of waiting for a value to be written to the register file (WB
stage) and then reading it back (ID stage), we can **forward** the value
directly from where it's produced to where it's needed.

```
    Without forwarding (3-cycle stall):

    Cycle:    1     2     3     4     5     6     7     8
    ADD R1:  [IF]  [ID]  [EX]  [MEM] [WB]
    SUB R4:        [IF]  [--]  [--]  [--]  [ID]  [EX]  ...
                          stall stall stall
                          (waiting for R1)

    With forwarding from EX (0-cycle stall):

    Cycle:    1     2     3     4     5     6
    ADD R1:  [IF]  [ID]  [EX]  [MEM] [WB]
    SUB R4:        [IF]  [ID]  [EX]  [MEM] [WB]
                          ^-----|
                    forwarded R1 value from EX stage
```

Forwarding adds wires (called "bypass paths") that connect the output of
the EX and MEM stages back to the input of the EX stage:

```
    +------+   +------+   +------+   +------+   +------+
    |  IF  |-->|  ID  |-->|  EX  |-->| MEM  |-->|  WB  |
    +------+   +------+   +------+   +------+   +------+
                  ^            |          |
                  |            |          |
                  +--- forward from EX ---+
                  +--- forward from MEM --+
```

### When Forwarding Cannot Help: Load-Use Stall

A load instruction reads data from memory in the MEM stage. If the very
next instruction needs that value, it's not available yet — even forwarding
can't fix this because the value doesn't exist until after MEM.

```
    LOAD R1, [addr]   <- R1 available after MEM (cycle 4)
    ADD R4, R1, R5    <- needs R1 in EX (cycle 3)
                         ^-- value doesn't exist yet!

    Cycle:    1     2     3     4     5     6     7
    LOAD R1: [IF]  [ID]  [EX]  [MEM] [WB]
    ADD R4:        [IF]  [ID]  [--]  [EX]  [MEM] [WB]
                                ^--- 1-cycle stall (bubble)
                          then forward from MEM to EX
```

One cycle of stall is unavoidable. The hardware inserts a "bubble" (NOP)
into the pipeline and freezes the earlier stages for one cycle. After MEM
completes, the value can be forwarded.

### Flushing (Branch Misprediction Recovery)

When a branch is resolved in the EX stage and found to be mispredicted,
the pipeline must:

1. Discard instructions in IF and ID (replace with bubbles)
2. Set PC to the correct target address
3. Resume fetching from the correct path

```
    Before flush:
    Cycle:    1     2     3
    BEQ:     [IF]  [ID]  [EX]  <- discovered: branch TAKEN
    wrong1:        [IF]  [ID]  <- about to execute wrongly
    wrong2:              [IF]  <- about to decode wrongly

    After flush:
    Cycle:    1     2     3     4     5
    BEQ:     [IF]  [ID]  [EX]
    wrong1:        [IF]  [--]  <- flushed (bubble)
    wrong2:              [--]  <- flushed (bubble)
    target:                    [IF]  [ID]  [EX]
```

The cost of a misprediction is the **branch penalty** — typically 2-3
cycles for a simple pipeline, 15-20 cycles for deep pipelines (like modern
x86 with 15+ stages).

### Hazard Detection Summary

```
    Hazard Type     | Detection                      | Resolution
    ----------------+--------------------------------+---------------------------
    RAW (data)      | ID reads reg that EX/MEM       | Forward from EX or MEM
                    | is writing                     |
    Load-use (data) | ID reads reg that EX is        | Stall 1 cycle, then
                    | loading from memory            | forward from MEM
    Control         | Branch mispredicted in EX      | Flush IF and ID, redirect
    Structural      | Two stages need same resource   | Stall or duplicate resource
```

### Implementation Reference

See `code/packages/python/hazard-detection/` for:
- `types.py` — `PipelineSlot`, `HazardAction`, `HazardResult` data types
- `data_hazard.py` — RAW hazard detection with forwarding
- `control_hazard.py` — Branch misprediction detection
- `structural_hazard.py` — Resource contention detection
- `hazard_unit.py` — Unified hazard detection unit

---

## 8. The Python Implementation

### CPU Simulator (`code/packages/python/cpu-simulator/`)

```
    cpu-simulator/
    |-- src/cpu_simulator/
    |   |-- cpu.py         # CPU class with fetch-decode-execute cycle
    |   |-- registers.py   # RegisterFile (N registers of B bits)
    |   |-- memory.py      # Byte-addressable memory
    |   |-- pipeline.py    # PipelineTrace, FetchResult, DecodeResult, ExecuteResult
```

**Key design:** The CPU is ISA-agnostic. It accepts pluggable
`InstructionDecoder` and `InstructionExecutor` protocols. The same CPU class
can run RISC-V, ARM, WASM, or Intel 4004 programs by swapping the decoder
and executor.

```python
    cpu = CPU(
        decoder=my_riscv_decoder,
        executor=my_riscv_executor,
        num_registers=32,
    )
    cpu.load_program(machine_code_bytes)
    traces = cpu.run()
```

Each `step()` call returns a `PipelineTrace` showing what happened at each
stage — making the pipeline visible for learning and debugging.

### Cache (`code/packages/python/cache/`)

```
    cache/
    |-- src/cache/
    |   |-- cache.py       # Main cache logic (configurable for L1/L2/L3)
    |   |-- cache_line.py  # Single cache line (valid, dirty, tag, data)
    |   |-- cache_set.py   # Set of N-way associative lines with LRU
    |   |-- hierarchy.py   # Multi-level cache hierarchy
    |   |-- stats.py       # Hit/miss counters and rates
```

### Branch Predictor (`code/packages/python/branch-predictor/`)

```
    branch-predictor/
    |-- src/branch_predictor/
    |   |-- base.py        # Prediction dataclass
    |   |-- static.py      # Always-taken, always-not-taken
    |   |-- one_bit.py     # 1-bit predictor
    |   |-- two_bit.py     # 2-bit saturating counter
    |   |-- btb.py         # Branch target buffer
    |   |-- stats.py       # Accuracy tracking
```

### Clock (`code/packages/python/clock/`)

```
    clock/
    |-- src/clock/
    |   |-- clock.py       # ClockGenerator, ClockDivider, MultiPhaseClock
```
