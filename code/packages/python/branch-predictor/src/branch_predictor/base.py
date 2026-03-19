"""Base protocol for all branch predictors.

In CPU design, the branch predictor sits at the very front of the pipeline —
in the fetch stage. Before the CPU even knows what instruction it's looking at,
the predictor guesses whether the current PC points to a branch and, if so,
whether that branch will be taken.

Why is this necessary? Consider a 15-stage pipeline (like Intel's Skylake).
A branch instruction is resolved in stage ~10. Without prediction, the CPU
would have to stall for 10 cycles on EVERY branch — roughly 20% of all
instructions. With prediction, the CPU speculatively fetches down the
predicted path. If the prediction is correct, there's zero cost. If wrong,
the pipeline flushes and restarts — a 10-15 cycle penalty.

The math works out: even 90% accuracy is a huge win.
- Without prediction: 20% branches × 10 cycle stall = 2 cycles/instruction penalty
- With 90% prediction: 20% branches × 10% miss × 15 cycle flush = 0.3 cycles/instruction

This module defines the interface that all predictors implement. By using a
Protocol (structural typing), any class that has predict/update/stats/reset
methods is automatically a valid BranchPredictor — no inheritance required.

Design pattern: Strategy
    Each predictor (AlwaysTaken, TwoBit, etc.) is a strategy that can be
    swapped into any CPU core design. The core only depends on BranchPredictor,
    never on a concrete predictor class.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from branch_predictor.stats import PredictionStats


# ─── Prediction ───────────────────────────────────────────────────────────────
#
# A Prediction is the output of the predict() method. It bundles three pieces
# of information:
#
# 1. taken     — will the branch jump to its target? (the core question)
# 2. confidence — how sure is the predictor? (useful for hybrid predictors
#                 that choose between sub-predictors based on confidence)
# 3. target    — where does the branch go? (from the BTB, if available)
#
# We use a frozen dataclass because predictions are values, not mutable state.
# Once the predictor makes a guess, that guess shouldn't change.


@dataclass(frozen=True)
class Prediction:
    """A branch prediction — the predictor's guess before the branch executes.

    Attributes:
        taken: The predictor's guess — will the branch be taken?
        confidence: How confident the predictor is, from 0.0 (guessing) to
            1.0 (certain). Used by hybrid/tournament predictors to choose
            between competing sub-predictors.
        target: The predicted target address, if known. This comes from the
            Branch Target Buffer (BTB), not the direction predictor itself.
            None means "I know it's taken, but I don't know where it goes."

    Example::

        # A confident prediction that the branch is taken, jumping to 0x400
        pred = Prediction(taken=True, confidence=0.9, target=0x400)

        # A low-confidence prediction from a cold-start predictor
        pred = Prediction(taken=False, confidence=0.0)
    """

    taken: bool
    """The predictor's guess: will the branch be taken?"""

    confidence: float = 0.0
    """Confidence level from 0.0 (no confidence) to 1.0 (certain)."""

    target: int | None = None
    """Predicted target address (from BTB, if available)."""


# ─── BranchPredictor Protocol ─────────────────────────────────────────────────
#
# This is a Protocol — Python's way of defining structural (duck) typing.
# Any class that has these four methods is a valid BranchPredictor, even
# without explicitly inheriting from it.
#
# The lifecycle of a branch prediction:
#
#   1. CPU fetches instruction at address `pc`
#   2. CPU calls predictor.predict(pc) → gets a Prediction
#   3. CPU speculatively fetches from the predicted path
#   4. Several cycles later, the branch resolves
#   5. CPU calls predictor.update(pc, actual_taken, actual_target)
#   6. Predictor adjusts its internal state to learn from the outcome
#
# This predict-then-update cycle is the fundamental feedback loop that makes
# branch prediction work. Without step 5, the predictor can never learn.


class BranchPredictor(Protocol):
    """Interface that all branch predictors must implement.

    The CPU core calls predict() before executing a branch.
    After the branch executes, the core calls update() with the actual outcome.
    This feedback loop is how the predictor learns.

    Any class implementing these methods is automatically a BranchPredictor
    (structural typing via Protocol — no inheritance needed).

    Example::

        class MyPredictor:
            def predict(self, pc: int) -> Prediction: ...
            def update(self, pc: int, taken: bool, target: int | None = None) -> None: ...
            @property
            def stats(self) -> PredictionStats: ...
            def reset(self) -> None: ...

        # MyPredictor is a valid BranchPredictor even without inheriting from it
    """

    def predict(self, pc: int) -> Prediction:
        """Predict whether the branch at address ``pc`` will be taken.

        Args:
            pc: The program counter (address) of the branch instruction.

        Returns:
            A Prediction with the predictor's guess and confidence.
        """
        ...

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:
        """Update the predictor with the actual branch outcome.

        This is the learning step. After the branch resolves in the execute
        stage, the core feeds back the real outcome so the predictor can
        adjust its tables.

        Args:
            pc: The program counter of the branch instruction.
            taken: Whether the branch was actually taken.
            target: The actual target address (if taken).
        """
        ...

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        ...

    def reset(self) -> None:
        """Reset all predictor state (for a new program).

        Clears the prediction table and resets statistics. Call this between
        benchmarks to ensure clean measurements.
        """
        ...
