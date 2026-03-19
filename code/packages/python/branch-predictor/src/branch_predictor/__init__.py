"""Branch Predictor — teaching CPUs to guess the future.

This package simulates the branch prediction algorithms used in real CPU cores.
Branch prediction is one of the most critical performance features in modern
processors — without it, a deeply pipelined CPU would stall on every branch
instruction, losing 10-15 cycles each time.

The package provides a pluggable architecture:

- **BranchPredictor** protocol — the interface all predictors implement
- **Static predictors** — AlwaysTaken, AlwaysNotTaken, BTFNT
- **Dynamic predictors** — OneBit (1-bit flip-flop), TwoBit (saturating counter)
- **BranchTargetBuffer** — caches WHERE branches go (used alongside any predictor)
- **PredictionStats** — tracks accuracy metrics for benchmarking

All predictors implement the same predict/update interface, so they can be
swapped into any CPU core design without changing the core's code.

Typical usage::

    from branch_predictor import TwoBitPredictor, BranchTargetBuffer

    predictor = TwoBitPredictor(table_size=1024)
    btb = BranchTargetBuffer(size=256)

    # Simulate a branch at PC=0x100
    prediction = predictor.predict(pc=0x100)
    if prediction.taken:
        target = btb.lookup(pc=0x100)

    # After execution, update both structures
    predictor.update(pc=0x100, taken=True, target=0x200)
    btb.update(pc=0x100, target=0x200)

    # Check accuracy
    print(f"Accuracy: {predictor.stats.accuracy:.1f}%")
"""

from branch_predictor.base import BranchPredictor, Prediction
from branch_predictor.btb import BTBEntry, BranchTargetBuffer
from branch_predictor.one_bit import OneBitPredictor
from branch_predictor.static import (
    AlwaysNotTakenPredictor,
    AlwaysTakenPredictor,
    BackwardTakenForwardNotTaken,
)
from branch_predictor.stats import PredictionStats
from branch_predictor.two_bit import TwoBitPredictor, TwoBitState

__all__ = [
    "AlwaysNotTakenPredictor",
    "AlwaysTakenPredictor",
    "BTBEntry",
    "BackwardTakenForwardNotTaken",
    "BranchPredictor",
    "BranchTargetBuffer",
    "OneBitPredictor",
    "Prediction",
    "PredictionStats",
    "TwoBitPredictor",
    "TwoBitState",
]
