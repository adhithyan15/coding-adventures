"""Prediction statistics — measuring how well a branch predictor performs.

Every branch predictor needs a scorecard. When a CPU designer evaluates a
predictor, the first question is always: "What's the accuracy?" A predictor
that's 95% accurate causes a pipeline flush on only 5% of branches, while
a 70% accurate predictor flushes on 30% — potentially halving throughput
on a deeply pipelined machine.

This module provides PredictionStats, a simple counter-based tracker that
records every prediction and computes accuracy metrics.

Real-world context:
- Intel's Pentium Pro achieved ~90% accuracy with a two-level adaptive predictor
- Modern CPUs (since ~2015) achieve 95-99% accuracy using TAGE or perceptron predictors
- Even a 1% improvement in accuracy can yield measurable speedups on branch-heavy code
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ─── PredictionStats ──────────────────────────────────────────────────────────
#
# A dataclass that acts as the scoreboard for any predictor. Every time the
# predictor makes a guess, the core calls `record(correct)` to log whether
# the guess was right or wrong.
#
# We track three counters:
#   predictions — total number of branches seen
#   correct     — how many the predictor got right
#   incorrect   — how many it got wrong
#
# From these, we derive:
#   accuracy            — correct / predictions × 100 (as a percentage)
#   misprediction_rate  — incorrect / predictions × 100 (the complement)
#
# Edge case: if no predictions have been made yet, both rates return 0.0
# rather than raising a ZeroDivisionError. This is a design choice — a
# predictor that hasn't seen any branches has no accuracy, not infinite accuracy.


@dataclass
class PredictionStats:
    """Tracks prediction accuracy for a branch predictor.

    Example usage::

        stats = PredictionStats()
        stats.record(correct=True)   # predictor got it right
        stats.record(correct=True)   # right again
        stats.record(correct=False)  # wrong this time
        print(stats.accuracy)        # 66.67 (2 out of 3)

    The stats object is usually owned by a predictor and exposed via its
    ``stats`` property. The CPU core never creates PredictionStats directly —
    it just reads the predictor's stats after running a benchmark.
    """

    # ── Counters ──────────────────────────────────────────────────────────
    #
    # We use field(default=0) so that creating PredictionStats() starts at
    # zero without needing an __init__ override.

    predictions: int = field(default=0)
    """Total number of predictions made."""

    correct: int = field(default=0)
    """Number of correct predictions."""

    incorrect: int = field(default=0)
    """Number of incorrect predictions (mispredictions)."""

    # ── Derived Metrics ───────────────────────────────────────────────────
    #
    # These are properties, not stored fields, because they're computed from
    # the counters. This avoids the classic bug of updating a counter but
    # forgetting to update the derived value.

    @property
    def accuracy(self) -> float:
        """Prediction accuracy as a percentage (0.0 to 100.0).

        Returns 0.0 if no predictions have been made yet, because we can't
        divide by zero, and "no data" is semantically closer to "0% accurate"
        than "100% accurate" in a benchmarking context.

        Example::

            stats = PredictionStats(predictions=100, correct=87, incorrect=13)
            assert stats.accuracy == 87.0
        """
        if self.predictions == 0:
            return 0.0
        return (self.correct / self.predictions) * 100.0

    @property
    def misprediction_rate(self) -> float:
        """Misprediction rate as a percentage (0.0 to 100.0).

        This is the complement of accuracy:  misprediction_rate = 100 - accuracy.
        CPU architects often think in terms of misprediction rate because each
        misprediction causes a pipeline flush — a concrete, measurable cost.

        Example::

            stats = PredictionStats(predictions=100, correct=87, incorrect=13)
            assert stats.misprediction_rate == 13.0
        """
        if self.predictions == 0:
            return 0.0
        return (self.incorrect / self.predictions) * 100.0

    # ── Mutation ──────────────────────────────────────────────────────────

    def record(self, *, correct: bool) -> None:
        """Record the outcome of a single prediction.

        Args:
            correct: True if the predictor guessed correctly, False otherwise.

        This is the primary API that the CPU core calls after every branch.
        It's keyword-only to prevent accidentally swapping arguments.

        Example::

            stats = PredictionStats()
            stats.record(correct=True)
            assert stats.predictions == 1
            assert stats.correct == 1
        """
        self.predictions += 1
        if correct:
            self.correct += 1
        else:
            self.incorrect += 1

    def reset(self) -> None:
        """Reset all counters to zero.

        Called when starting a new benchmark or program execution. Without
        this, stats from a previous run would contaminate the new measurement.

        Example::

            stats = PredictionStats(predictions=50, correct=40, incorrect=10)
            stats.reset()
            assert stats.predictions == 0
            assert stats.accuracy == 0.0
        """
        self.predictions = 0
        self.correct = 0
        self.incorrect = 0
