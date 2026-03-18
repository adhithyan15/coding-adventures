"""Static branch predictors — the simplest strategies, requiring no learning.

Static predictors make the same prediction every time, regardless of history.
They require zero hardware (no tables, no counters, no state) and serve as
baselines against which dynamic predictors are measured.

Three strategies are implemented here:

1. **AlwaysTakenPredictor** — always predicts "taken"
   Accuracy: ~60-70% on typical code. Why? Most branches are loop back-edges,
   which are taken on every iteration except the last. A loop that runs 100
   times has 100 branches: 99 taken + 1 not-taken = 99% accuracy on that loop.
   The overall ~60% comes from mixing loops with if-else branches.

2. **AlwaysNotTakenPredictor** — always predicts "not taken"
   Accuracy: ~30-40% on typical code. This is the worst reasonable strategy,
   but it has a hardware advantage: the "not taken" path is just the next
   sequential instruction, so the CPU doesn't need to compute a target address.
   Early processors (Intel 8086) effectively used this because they had no
   branch prediction unit — they just fetched the next instruction.

3. **BackwardTakenForwardNotTaken (BTFNT)** — direction-based heuristic
   Accuracy: ~65-75% on typical code. Backward branches (target < pc) are
   usually loop back-edges, so predict taken. Forward branches (target > pc)
   are usually if-else, so predict not-taken. This is what early MIPS R4000
   and SPARC processors used. It requires knowing the branch target at decode
   time, which is available for direct branches but not indirect ones.

Historical note:
    The MIPS architecture was designed with "branch delay slots" specifically
    because early MIPS had no branch prediction. The instruction after a branch
    always executes (the CPU doesn't try to predict), and the compiler fills
    that slot with useful work. This architectural decision haunts MIPS to this
    day — even MIPS64 Release 6 still has branch delay slots for compatibility.
"""

from __future__ import annotations

from branch_predictor.base import Prediction
from branch_predictor.stats import PredictionStats


# ─── AlwaysTakenPredictor ─────────────────────────────────────────────────────
#
# The simplest "optimistic" predictor. Always bets that the branch will be
# taken (jump to the target address).
#
# Hardware cost: zero. No tables, no counters, no state at all.
# The prediction logic is just a wire tied to 1.
#
# When it works well:
#   - Tight loops (for i in range(1000): ...) — 999/1000 correct
#   - Unconditional jumps — 100% correct (they're always taken)
#
# When it fails:
#   - if x > 0: ... (random data) — ~50% correct
#   - Early loop exits — misses every exit


class AlwaysTakenPredictor:
    """Always predicts 'taken'. Simple but surprisingly effective (~60% accurate).

    Why? Most branches in real programs are loop back-edges, which are taken
    on every iteration except the last. So "always taken" gets the loop body
    right every time, only missing the final exit.

    Example::

        predictor = AlwaysTakenPredictor()
        pred = predictor.predict(pc=0x100)
        assert pred.taken is True
    """

    def __init__(self) -> None:
        self._stats = PredictionStats()

    def predict(self, pc: int) -> Prediction:  # noqa: ARG002
        """Always predict taken, with zero confidence (it's just a guess).

        Args:
            pc: The program counter of the branch instruction (unused).

        Returns:
            Prediction(taken=True, confidence=0.0)
        """
        return Prediction(taken=True, confidence=0.0)

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:  # noqa: ARG002
        """Record whether the always-taken guess was correct.

        Args:
            pc: The program counter (unused — no per-branch state).
            taken: Whether the branch was actually taken.
            target: The actual target address (unused).
        """
        self._stats.record(correct=taken)

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        return self._stats

    def reset(self) -> None:
        """Reset statistics (no predictor state to clear)."""
        self._stats.reset()


# ─── AlwaysNotTakenPredictor ──────────────────────────────────────────────────
#
# The simplest "pessimistic" predictor. Always bets the branch falls through
# to the next sequential instruction.
#
# Hardware advantage: the "next sequential instruction" is already being fetched
# by the instruction fetch unit. No target address computation needed. This is
# why the earliest processors implicitly used this strategy — they didn't have
# a branch prediction unit, so they just kept fetching sequentially.
#
# The Intel 8086 (1978) worked this way:
#   FETCH → DECODE → EXECUTE (3-stage pipeline, no prediction)
#   On a taken branch, it flushed 2 instructions and restarted.
#   With ~20% branch frequency and ~60% taken rate, this cost ~0.24 CPI.


class AlwaysNotTakenPredictor:
    """Always predicts 'not taken'. The simplest possible predictor.

    This is the baseline against which all other predictors are measured.
    If your fancy predictor can't beat "always not taken", something is wrong.

    Example::

        predictor = AlwaysNotTakenPredictor()
        pred = predictor.predict(pc=0x100)
        assert pred.taken is False
    """

    def __init__(self) -> None:
        self._stats = PredictionStats()

    def predict(self, pc: int) -> Prediction:  # noqa: ARG002
        """Always predict not taken, with zero confidence.

        Args:
            pc: The program counter of the branch instruction (unused).

        Returns:
            Prediction(taken=False, confidence=0.0)
        """
        return Prediction(taken=False, confidence=0.0)

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:  # noqa: ARG002
        """Record whether the always-not-taken guess was correct.

        Args:
            pc: The program counter (unused).
            taken: Whether the branch was actually taken.
            target: The actual target address (unused).
        """
        # We predicted NOT taken, so we're correct when the branch is NOT taken
        self._stats.record(correct=not taken)

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        return self._stats

    def reset(self) -> None:
        """Reset statistics (no predictor state to clear)."""
        self._stats.reset()


# ─── BackwardTakenForwardNotTaken (BTFNT) ─────────────────────────────────────
#
# A direction-based heuristic that uses the branch's target address relative
# to its own PC to make the prediction:
#
#   - Backward branch (target < pc) → predict TAKEN
#     These are almost always loop back-edges:
#       0x100: loop_body:
#       0x104:   add r1, r2, r3
#       0x108:   bne r1, r0, loop_body    ← target (0x100) < pc (0x108) → backward
#
#   - Forward branch (target > pc) → predict NOT TAKEN
#     These are usually if-then-else:
#       0x200:   beq r1, r0, skip         ← target (0x20C) > pc (0x200) → forward
#       0x204:   add r3, r4, r5           ← fall-through (common case)
#       0x208:   sub r6, r7, r8
#       0x20C: skip:
#
#   - Equal (target == pc) → predict TAKEN
#     This is a degenerate case (infinite loop). Rare, but we predict taken
#     to avoid an infinite stream of mispredictions.
#
# This predictor needs to know the target address at prediction time, which
# means it must be called after the decode stage (or the target must come
# from a BTB lookup). The BTFNT predictor stores the most recently known
# target for each branch so it can predict even before decode.
#
# Used in: MIPS R4000, SPARC V8, some early ARM processors.


class BackwardTakenForwardNotTaken:
    """BTFNT — predicts taken for backward branches, not-taken for forward.

    Backward branches (target < pc) are usually loop back-edges -> predict taken.
    Forward branches (target > pc) are usually if-else -> predict not-taken.

    This is what early MIPS and SPARC processors used. It's a good balance
    between simplicity and accuracy, achieving ~65-75% on typical code.

    The predictor requires knowing the branch target. On the first encounter
    of a branch (cold start), it defaults to predicting NOT taken, since we
    don't yet know the target direction.

    Example::

        predictor = BackwardTakenForwardNotTaken()

        # Backward branch (loop back-edge): predict taken
        pred = predictor.predict(pc=0x108)
        predictor.update(pc=0x108, taken=True, target=0x100)

        # Now the predictor knows the target, future predictions use direction
        pred = predictor.predict(pc=0x108)
        assert pred.taken is True  # backward → taken
    """

    def __init__(self) -> None:
        self._stats = PredictionStats()
        # Maps PC → last known target address. We need this because predict()
        # is called before decode, so we rely on the BTB (or previous updates)
        # to know the branch direction.
        self._targets: dict[int, int] = {}

    def predict(self, pc: int) -> Prediction:
        """Predict based on branch direction: backward=taken, forward=not-taken.

        If we haven't seen this branch before (no known target), we default
        to NOT taken — the safe choice that doesn't require a target address.

        Args:
            pc: The program counter of the branch instruction.

        Returns:
            Prediction with taken=True for backward branches,
            taken=False for forward branches or unknown targets.
        """
        target = self._targets.get(pc)
        if target is None:
            # Cold start — we don't know the target direction yet.
            # Default to not-taken (the safe fallback).
            return Prediction(taken=False, confidence=0.0)

        # Backward branch (target <= pc) → taken (loop back-edge)
        # Forward branch (target > pc)  → not taken (if-else)
        taken = target <= pc
        return Prediction(taken=taken, confidence=0.5, target=target)

    def update(self, pc: int, taken: bool, target: int | None = None) -> None:
        """Record the branch outcome and learn the target address.

        The key learning here is remembering the target address for future
        predictions. The BTFNT predictor doesn't adapt its strategy — it
        always uses the direction heuristic — but it needs to know the target
        to determine the direction.

        Args:
            pc: The program counter of the branch instruction.
            taken: Whether the branch was actually taken.
            target: The actual target address (stored for future direction checks).
        """
        # Store the target so we can use it for future direction-based predictions
        if target is not None:
            self._targets[pc] = target

        # Determine what we would have predicted, accounting for cold starts
        known_target = self._targets.get(pc)
        if known_target is None:
            # We had no target info — we predicted not-taken
            predicted_taken = False
        else:
            predicted_taken = known_target <= pc

        self._stats.record(correct=(predicted_taken == taken))

    @property
    def stats(self) -> PredictionStats:
        """Get prediction accuracy statistics."""
        return self._stats

    def reset(self) -> None:
        """Reset all state — target cache and statistics."""
        self._targets.clear()
        self._stats.reset()
