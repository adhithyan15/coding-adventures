# Branch Predictor

Simulates the branch prediction algorithms used in real CPU cores. Branch prediction is one of the most critical performance features in modern processors — without it, a deeply pipelined CPU would stall on every branch instruction, losing 10-15 cycles each time.

## What's Inside

| Module | Description |
|--------|-------------|
| `base.py` | `BranchPredictor` protocol and `Prediction` dataclass |
| `static.py` | `AlwaysTakenPredictor`, `AlwaysNotTakenPredictor`, `BackwardTakenForwardNotTaken` |
| `one_bit.py` | `OneBitPredictor` — 1-bit flip-flop per branch |
| `two_bit.py` | `TwoBitPredictor` — 2-bit saturating counter (the classic) |
| `btb.py` | `BranchTargetBuffer` — caches branch target addresses |
| `stats.py` | `PredictionStats` — accuracy tracking |

## How It Fits in the Stack

This package is **completely standalone** — no dependencies on other packages. It provides the branch prediction unit that plugs into a CPU core's fetch stage. Any predictor can be swapped in because they all implement the same `BranchPredictor` protocol.

## Design: Pluggable Predictors

All predictors implement the same interface:

```python
class BranchPredictor(Protocol):
    def predict(self, pc: int) -> Prediction: ...
    def update(self, pc: int, taken: bool, target: int | None = None) -> None: ...
    @property
    def stats(self) -> PredictionStats: ...
    def reset(self) -> None: ...
```

This means you can swap `AlwaysTakenPredictor` for `TwoBitPredictor` without changing any other code.

## Usage Examples

### Basic prediction loop

```python
from branch_predictor import TwoBitPredictor

predictor = TwoBitPredictor(table_size=1024)

# Simulate a branch at PC=0x100
prediction = predictor.predict(pc=0x100)
print(f"Predicted: {'taken' if prediction.taken else 'not taken'}")

# After execution, update with actual outcome
predictor.update(pc=0x100, taken=True)

# Check accuracy
print(f"Accuracy: {predictor.stats.accuracy:.1f}%")
```

### Comparing predictors

```python
from branch_predictor import (
    AlwaysTakenPredictor,
    OneBitPredictor,
    TwoBitPredictor,
)

predictors = {
    "Always Taken": AlwaysTakenPredictor(),
    "1-bit": OneBitPredictor(),
    "2-bit": TwoBitPredictor(),
}

# Simulate a loop: 9 taken + 1 not-taken, repeated 5 times
for name, pred in predictors.items():
    for _ in range(5):
        for i in range(10):
            pred.update(pc=0x100, taken=(i < 9))
    print(f"{name}: {pred.stats.accuracy:.1f}%")
```

### Using the BTB alongside a predictor

```python
from branch_predictor import TwoBitPredictor, BranchTargetBuffer

predictor = TwoBitPredictor()
btb = BranchTargetBuffer(size=256)

pc = 0x100
prediction = predictor.predict(pc)
if prediction.taken:
    target = btb.lookup(pc)  # WHERE does it go?

# After execution
predictor.update(pc, taken=True, target=0x200)
btb.update(pc, target=0x200, branch_type="conditional")
```

## Installation

```bash
pip install coding-adventures-branch-predictor
```

Or for development:

```bash
uv venv && uv pip install -e ".[dev]"
```

## Running Tests

```bash
python -m pytest tests/ -v --cov=branch_predictor --cov-report=term-missing
```
