"""One-bit branch predictor — one flip-flop per branch.

The one-bit predictor is the simplest dynamic predictor. Unlike static
predictors (AlwaysTaken, BTFNT), it actually learns from the branch's
history. Each branch address maps to a single bit of state that records
the last outcome:

    bit = 0 → predict NOT TAKEN
    bit = 1 → predict TAKEN

After each branch resolves, the bit is updated to match the actual outcome.
This means the predictor always predicts "whatever happened last time."

Hardware implementation:
    A small SRAM table indexed by the lower bits of the PC.
    Each entry is a single flip-flop (1 bit of storage).
    Total storage: table_size × 1 bit.
    For a 1024-entry table: 1024 bits = 128 bytes.

The aliasing problem:
    Since the table is indexed by (pc % table_size), two different branches
    can map to the same entry. This is called "aliasing" or "interference."
    When branches alias, they corrupt each other's predictions.

    Example with table_size=4:
        Branch at 0x100 → index 0 (0x100 % 4 = 0)
        Branch at 0x104 → index 0 (0x104 % 4 = 0)   ← COLLISION!

    With larger tables (1024+), aliasing is rare for most programs.

The double-misprediction problem:
    Consider a loop that runs N times then exits:

        for i in range(10):
            body()      # branch at end: taken 9 times, not-taken once

    Iteration 1: bit=0 (cold) → predict NOT TAKEN → actual TAKEN → WRONG, set bit=1
    Iteration 2: bit=1 → predict TAKEN → actual TAKEN → correct
    ...
    Iteration 9: bit=1 → predict TAKEN → actual TAKEN → correct
    Iteration 10: bit=1 → predict TAKEN → actual NOT TAKEN → WRONG, set bit=0

    Next time the loop runs:
    Iteration 1: bit=0 → predict NOT TAKEN → actual TAKEN → WRONG, set bit=1

    Result: 2 mispredictions per loop invocation (first and last iterations).
    For a loop running 10 times, that's 2/10 = 20% misprediction rate.
    The two-bit predictor solves this — see two_bit.py.
"""

from __future__ import annotations

from branch_predictor.base import Prediction
from branch_predictor.stats import PredictionStats


class OneBitPredictor:
    """1-bit predictor — one flip-flop per branch address.

    Maintains a table of 1-bit entries indexed by (pc % table_size).
    Each entry remembers the LAST outcome of that branch.

    The fundamental state diagram::

        ┌─────────────────┐     taken      ┌─────────────────┐
        │ Predict NOT TAKEN│ ──────────────→ │  Predict TAKEN   │
        │    (bit = 0)     │ ←────────────── │    (bit = 1)     │
        └─────────────────┘   not taken     └─────────────────┘

    Every misprediction flips the bit. This is too aggressive — a single
    anomalous outcome changes the prediction. The 2-bit predictor adds
    hysteresis to fix this.

    Args:
        table_size: Number of entries in the prediction table. Must be a
            power of 2 for efficient hardware implementation (though this
            simulator doesn't enforce that). Larger tables reduce aliasing
            but cost more silicon. Default: 1024 entries = 128 bytes.

    Example::

        predictor = OneBitPredictor(table_size=1024)

        # First encounter — cold start, defaults to NOT TAKEN
        pred = predictor.predict(pc=0x100)
        assert pred.taken is False

        # Update with actual outcome: branch was taken
        predictor.update(pc=0x100, taken=True)

        # Now predicts TAKEN (remembers last outcome)
        pred = predictor.predict(pc=0x100)
        assert pred.taken is True
    """

    def __init__(self, table_size: int = 1024) -> None:
        # ── Table size ────────────────────────────────────────────────────
        # In hardware, this would be the number of rows in a small SRAM.
        # Common sizes: 256, 512, 1024, 2048, 4096.
        self._table_size = table_size

        # ── Prediction table ──────────────────────────────────────────────
        # Maps (index) → last_outcome. We use a dict rather than a list
        # to avoid pre-allocating memory for entries that are never accessed.
        # In hardware, all entries exist physically but start at 0 (not-taken).
        self._table: dict[int, bool] = {}

        # ── Statistics tracker ────────────────────────────────────────────
        self._stats = PredictionStats()

    def _index(self, pc: int) -> int:
        """Compute the table index for a given PC.

        In hardware, this is just the lower log2(table_size) bits of the PC.
        Using modulo achieves the same result in software.

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            An integer in [0, table_size) used to index the prediction table.
        """
        return pc % self._table_size

    def predict(self, pc: int) -> Prediction:
        """Predict based on the last outcome of this branch.

        On a cold start (branch not yet seen), defaults to NOT TAKEN.
        This is a common design choice — the bit starts at 0.

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            Prediction with taken matching the stored bit for this branch.
        """
        index = self._index(pc)
        taken = self._table.get(index, False)  # default: not taken
        # Confidence: 0.5 because we only have 1 bit of history.
        # We know the last outcome, but that's weak evidence.
        return Prediction(taken=taken, confidence=0.5)

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:  # noqa: ARG002
        """Update the prediction table with the actual outcome.

        Simply sets the bit to match the actual outcome. This is the "flip"
        that gives the 1-bit predictor its characteristic behavior.

        Args:
            pc: The program counter of the branch instruction.
            taken: Whether the branch was actually taken.
            target: The actual target address (unused by this predictor).
        """
        index = self._index(pc)
        # Record accuracy BEFORE updating the table, so we compare against
        # what the predictor would have predicted.
        predicted = self._table.get(index, False)
        self._stats.record(correct=(predicted == taken))
        # Now update the table to remember this outcome for next time.
        self._table[index] = taken

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        return self._stats

    def reset(self) -> None:
        """Reset the prediction table and statistics."""
        self._table.clear()
        self._stats.reset()
