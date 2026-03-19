# D02 — Branch Predictor

## Overview

The branch predictor is a hardware unit that guesses the outcome of branch
instructions (if/else, loops, function returns) before the branch condition is
actually evaluated. This might sound absurd — why guess when you can just wait
and know for sure? — but it is one of the most critical performance features in
modern CPUs.

The reason: **pipelines.** In a 5-stage pipeline, by the time the CPU knows
whether a branch is taken (end of stage 3: Execute), it has already fetched
two more instructions. If the branch IS taken, those two instructions are wrong
and must be thrown away — a **pipeline flush** that wastes 2 cycles. In a
13-stage pipeline (like ARM Cortex-A78), a misprediction wastes ~11 cycles.

A good branch predictor is correct 95-99% of the time, turning a potential
11-cycle penalty into a ~0.1-cycle average penalty. This is why modern CPUs
dedicate thousands of transistors to branch prediction.

## Layer Position

```
Core (D05)
├── Pipeline (D04) ← fetches instructions based on prediction
├── Branch Predictor ← YOU ARE HERE
│   predict(pc) → taken/not-taken + target address
│   update(pc, actual_taken, actual_target)
├── Hazard Detection (D03) ← triggers flush on misprediction
└── ...
```

**Depends on:** `clock` (predictions happen on clock edges)
**Used by:** `pipeline` (D04) during IF stage, `core` (D05)

## Key Concepts

### Why Branches Are a Problem

Consider this loop executing on a 5-stage pipeline:

```
Address  Instruction
0x100:   LOAD R1, [R0]        ; load element
0x104:   ADD  R2, R2, R1      ; accumulate
0x108:   ADD  R0, R0, #4      ; next element
0x10C:   CMP  R0, R3          ; reached end?
0x110:   BNE  0x100           ; branch if not equal → loop back

Pipeline without prediction:

Cycle:  1    2    3    4    5    6    7    8    9    10   11
IF:    LOAD  ADD  ADD  CMP  BNE  ???  ???  LOAD  ADD  ADD  CMP
ID:          LOAD  ADD  ADD  CMP  BNE  ---  ---  LOAD  ADD  ADD
EX:                LOAD  ADD  ADD  CMP  BNE  ---  ---  LOAD  ADD
MEM:                     LOAD  ADD  ADD  CMP  BNE  ---  ---  LOAD
WB:                           LOAD  ADD  ADD  CMP  BNE  ---  ---

At cycle 7, BNE reaches EX and we finally know it is taken.
Cycles 6-7 fetched instructions from 0x114, 0x118 — WRONG!
We must flush those and restart from 0x100.
That is 2 wasted cycles per loop iteration.
For 1000 iterations: 2000 wasted cycles!

Pipeline WITH prediction (predict taken):

The predictor says "BNE will be taken → fetch from 0x100 next."
The pipeline fetches LOAD at 0x100 immediately after BNE, no gap.
If the prediction is correct (999 out of 1000 times), zero waste.
Only the last iteration (when the loop exits) is mispredicted.

Total waste: ~2 cycles instead of ~2000. That is a 1000x improvement.
```

### Static Prediction

The simplest predictors use fixed rules — no history, no state:

```
Strategy              Rule                        Accuracy
─────────────────────────────────────────────────────────────
Always Not-Taken      Predict all branches as      ~40-50%
                      not taken (fall through)      (many branches ARE taken)

Always Taken          Predict all branches as      ~60-70%
                      taken                         (loops are usually taken)

BTFN (Backward-Taken  If branch target < PC:       ~75-80%
 Forward-Not-Taken)    predict taken (it is a loop)  (good heuristic:
                      If branch target > PC:         backward = loop,
                       predict not-taken              forward = if/else)
```

BTFN works because backward branches are almost always loop-back edges (taken
N-1 out of N iterations), while forward branches are typically if-else
conditions (often not taken on the "happy path").

### 1-Bit Predictor

A single bit of state per branch, stored in a table indexed by the low bits of
the program counter:

```
Branch History Table (BHT):
┌─────────────┬────────────┐
│ PC[low bits] │ Prediction │
├─────────────┼────────────┤
│ 0x00         │ Taken      │
│ 0x01         │ Not Taken  │
│ 0x02         │ Taken      │
│ ...          │ ...        │
└─────────────┴────────────┘

Rule: predict whatever happened last time.
On misprediction: flip the bit.
```

**Problem with 1-bit:** Consider a loop that runs 10 times. The predictor
mispredicts twice per complete execution: once when the loop starts (if the
previous execution ended with "not taken") and once when the loop ends (the
last iteration is "not taken" but we predicted "taken").

```
Loop iterations:  T T T T T T T T T N  T T T T T T T T T N
1-bit prediction: N T T T T T T T T T  N T T T T T T T T T
                  ^                  ^  ^                  ^
                  miss               miss miss             miss

2 mispredictions per 10 iterations = 80% accuracy for a loop.
With nested loops, the inner loop exit poisons the outer loop entry.
```

### 2-Bit Saturating Counter

The standard improvement: use 2 bits per branch, giving 4 states. The
predictor must see two consecutive mispredictions before changing its mind:

```
State Machine (2-bit saturating counter):

                 Taken                    Taken
          ┌─────────────┐          ┌─────────────┐
          │             ▼          │             ▼
     ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
     │  SNT    │──→│  WNT    │──→│   WT    │──→│   ST    │
     │ Strongly│   │ Weakly  │   │ Weakly  │   │ Strongly│
     │Not Taken│   │Not Taken│   │  Taken  │   │  Taken  │
     └─────────┘   └─────────┘   └─────────┘   └─────────┘
          ▲             │          ▲             │
          └─────────────┘          └─────────────┘
             Not Taken                Not Taken

     Predict NOT TAKEN ◄──┤          ├──► Predict TAKEN
        (SNT or WNT)                    (WT or ST)
```

**How it helps with loops:** When the inner loop exits (not taken), the counter
moves from ST to WT — but still predicts "taken." The next time the loop
starts, the prediction is still correct. Only two consecutive "not taken"
outcomes flip the prediction.

```
Loop iterations:  T T T T T T T T T N  T T T T T T T T T N
2-bit state:      WT ST ST ST ST ST ST ST ST ST WT ST ST ST ST ST ST ST ST ST WT
2-bit prediction: T  T  T  T  T  T  T  T  T  T  T  T  T  T  T  T  T  T  T  T
                                       ^                                    ^
                                      miss                                 miss

1 misprediction per 10 iterations = 90% accuracy.
```

### Branch Target Buffer (BTB)

The predictors above answer "is the branch taken?" but not "where does it
go?" For direct branches (branch to a fixed address), the target is encoded in
the instruction. But the pipeline needs the target address in the IF stage —
before the instruction is even decoded!

The BTB is a cache that maps branch PCs to their last-known target addresses:

```
Branch Target Buffer:
┌─────────┬──────────┬────────────────┐
│   PC    │  Valid   │ Target Address │
├─────────┼──────────┼────────────────┤
│ 0x0110  │   Yes    │    0x0100      │  (the BNE loop-back)
│ 0x0200  │   Yes    │    0x0300      │  (a forward branch)
│ 0x0444  │   No     │    -----      │
│ ...     │   ...    │    ...        │
└─────────┴──────────┴────────────────┘

IF stage logic:
  1. Look up current PC in BTB
  2. If BTB hit AND direction predictor says "taken":
     → next PC = BTB target (speculative fetch from branch target)
  3. If BTB miss OR predictor says "not taken":
     → next PC = PC + 4 (fetch next sequential instruction)
```

The BTB is essential for achieving single-cycle branch resolution in the IF
stage. Without it, even a correct "taken" prediction would require waiting
until ID stage to read the target from the instruction encoding.

### Global History Register (Future Extension)

Many branches are correlated. For example:

```c
if (x > 0) {     // Branch A
    ...
}
if (x > 5) {     // Branch B — if A was not taken, B is also not taken!
    ...
}
```

A **Global History Register (GHR)** is a shift register that records the
outcomes of the last N branches (taken/not-taken as 1/0). This history is
XORed with the PC to index into the prediction table, allowing the predictor
to learn correlations between branches.

```
GHR (8-bit): [T, N, T, T, N, T, T, N] = 0b10110110

Table index = PC[low bits] XOR GHR

This means the same branch at the same PC gets a DIFFERENT prediction
depending on the recent branch history — enabling correlation-aware
prediction.
```

This is the foundation of **gshare** and **tournament** predictors.

### TAGE Predictor (Future Extension)

**TAGE** (TAgged GEometric history length predictor) is the state of the art
in branch prediction, used in most modern high-performance CPUs. It uses
multiple prediction tables with geometrically increasing history lengths:

```
Table 0: index by PC only           (no history, fallback)
Table 1: index by PC + last 4 branches
Table 2: index by PC + last 16 branches
Table 3: index by PC + last 64 branches
Table 4: index by PC + last 256 branches

Each entry is tagged, so the predictor can tell if the entry is relevant.
The prediction comes from the table with the longest matching history.
```

TAGE achieves 95-99% accuracy on real workloads by capturing patterns at
multiple time scales: short-term correlations (Table 1), medium-term
patterns (Table 2-3), and long-term behavior (Table 4).

## Public API

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum

class BranchOutcome(Enum):
    TAKEN = "taken"
    NOT_TAKEN = "not_taken"

@dataclass
class Prediction:
    """A branch prediction."""
    outcome: BranchOutcome       # Predicted direction: taken or not-taken
    target: int | None = None    # Predicted target address (from BTB, if available)
    confidence: float = 0.5      # Confidence level (0.0 to 1.0)

@dataclass
class PredictorStats:
    """Accuracy statistics for the predictor."""
    total_predictions: int = 0
    correct_predictions: int = 0
    direction_mispredictions: int = 0   # Wrong taken/not-taken
    target_mispredictions: int = 0       # Right direction, wrong target

    @property
    def accuracy(self) -> float:
        if self.total_predictions == 0:
            return 0.0
        return self.correct_predictions / self.total_predictions

    @property
    def misprediction_rate(self) -> float:
        return 1.0 - self.accuracy


class BranchPredictor(ABC):
    """
    Abstract interface for branch predictors.

    All predictor implementations must follow this protocol, making them
    pluggable into any core configuration.
    """

    @abstractmethod
    def predict(self, pc: int) -> Prediction:
        """
        Predict the outcome of a branch at the given program counter.

        Called during the IF stage of the pipeline, before the instruction
        is decoded. Must be fast (effectively combinational logic).
        """
        ...

    @abstractmethod
    def update(self, pc: int, actual_outcome: BranchOutcome, actual_target: int) -> None:
        """
        Update the predictor with the actual outcome of a branch.

        Called when the branch is resolved (EX stage). Updates internal
        state to improve future predictions.
        """
        ...

    @property
    @abstractmethod
    def stats(self) -> PredictorStats:
        """Return prediction accuracy statistics."""
        ...

    @abstractmethod
    def reset(self) -> None:
        """Reset all predictor state (clear history, tables, stats)."""
        ...


class StaticPredictor(BranchPredictor):
    """
    Static branch predictor — uses fixed rules, no learning.

    Strategies:
    - ALWAYS_TAKEN: predict all branches as taken
    - ALWAYS_NOT_TAKEN: predict all branches as not taken
    - BTFN: backward-taken, forward-not-taken
    """

    def __init__(self, strategy: str = "btfn") -> None: ...


class OneBitPredictor(BranchPredictor):
    """
    1-bit branch predictor.

    Maintains a table of 1-bit entries indexed by PC.
    Each entry records the last outcome. Predict = last outcome.
    """

    def __init__(self, table_size: int = 1024) -> None: ...


class TwoBitPredictor(BranchPredictor):
    """
    2-bit saturating counter predictor.

    Maintains a table of 2-bit counters indexed by PC.
    Counter states: SNT (00), WNT (01), WT (10), ST (11).
    Predict TAKEN if counter >= 2 (WT or ST).
    """

    def __init__(self, table_size: int = 1024) -> None: ...


class BranchTargetBuffer:
    """
    BTB — caches branch target addresses.

    Used in conjunction with a direction predictor to provide
    both the predicted direction AND target in the IF stage.
    """

    def __init__(self, num_entries: int = 256) -> None: ...

    def lookup(self, pc: int) -> int | None:
        """
        Look up the predicted target for a branch at PC.
        Returns None if the branch is not in the BTB.
        """
        ...

    def update(self, pc: int, target: int) -> None:
        """Record or update the target for a branch at PC."""
        ...


class CombinedPredictor(BranchPredictor):
    """
    Combines a direction predictor with a BTB.

    This is the standard configuration used in real cores:
    the direction predictor says taken/not-taken, and the
    BTB provides the target address.
    """

    def __init__(
        self,
        direction_predictor: BranchPredictor,
        btb: BranchTargetBuffer,
    ) -> None: ...
```

## Data Structures

### Internal State

```python
# 1-bit predictor table
one_bit_table: list[BranchOutcome]  # Indexed by PC % table_size

# 2-bit predictor table
two_bit_table: list[int]  # Values 0-3 (SNT=0, WNT=1, WT=2, ST=3)
                           # Indexed by PC % table_size

# BTB entry
@dataclass
class BTBEntry:
    valid: bool = False
    tag: int = 0           # High bits of PC for matching
    target: int = 0        # Predicted target address
```

## Test Strategy

### Static Predictor Tests

- **Always-taken**: verify every prediction is TAKEN regardless of PC
- **Always-not-taken**: verify every prediction is NOT_TAKEN
- **BTFN**: backward branch (target < PC) predicted taken; forward branch
  (target > PC) predicted not-taken
- **Accuracy tracking**: verify stats update correctly after update() calls

### 1-Bit Predictor Tests

- **Cold start**: first prediction for any PC defaults to NOT_TAKEN (or configurable)
- **Learning**: after update(pc, TAKEN), predict(pc) returns TAKEN
- **Flip on miss**: after update(pc, NOT_TAKEN) when state was TAKEN, prediction flips
- **Table indexing**: two PCs that differ only in high bits map to the same entry
  (aliasing), verify this behavior
- **Loop pattern**: simulate T,T,T,T,N,T,T,T,T,N — count mispredictions (should be 2 per group)

### 2-Bit Predictor Tests

- **State transitions**: verify all 4 states and transitions match the state machine diagram
- **Hysteresis**: one misprediction does not flip the prediction (ST → WT still predicts taken)
- **Two misses to flip**: SNT→WNT (still not-taken), WNT→WT (NOW taken)
- **Loop pattern**: simulate T,T,T,T,N,T,T,T,T,N — count mispredictions (should be 1 per group)
- **Saturation**: many consecutive TAKEN cannot push counter above 3 (ST)

### BTB Tests

- **Cold start**: lookup on empty BTB returns None
- **Store and retrieve**: update(pc, target) then lookup(pc) returns target
- **Capacity**: fill BTB beyond capacity, verify eviction of oldest entry
- **Tag matching**: two PCs mapping to same BTB index but different tags are distinguished

### Combined Predictor Tests

- **Taken + BTB hit**: direction says taken, BTB provides target → correct prediction
- **Taken + BTB miss**: direction says taken but no target → partial misprediction
- **Not-taken**: direction says not-taken → target is PC+4 regardless of BTB
- **Integration**: run a realistic branch trace, verify combined accuracy

### Benchmark Tests

- **Simple loop**: 1000 iterations of a tight loop → measure accuracy (should approach 99.9% for 2-bit)
- **Alternating**: T,N,T,N,T,N... pattern → measure accuracy (pathological for simple predictors)
- **Nested loops**: outer+inner loop pattern → compare 1-bit vs 2-bit accuracy

## Future Extensions

- **Gshare predictor**: XOR global history register with PC for table index
- **Tournament predictor**: meta-predictor chooses between local and global predictors
- **TAGE predictor**: tagged geometric history length predictor (modern standard)
- **Return Address Stack (RAS)**: dedicated stack for predicting function return addresses
- **Indirect branch predictor**: for virtual function calls (target varies)
- **Loop predictor**: specialized predictor that counts loop iterations
- **Trace-driven evaluation**: replay branch traces from real programs to benchmark predictors
