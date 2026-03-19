"""Control hazard detection — handling branch mispredictions.

=== What Is a Control Hazard? ===

A control hazard occurs when the pipeline doesn't know which instruction
to fetch next because a branch hasn't been resolved yet. Modern CPUs
use branch predictors to GUESS the outcome, but sometimes they guess
wrong. When that happens, instructions that were fetched based on the
wrong guess must be thrown away (flushed).

=== Timeline of a Branch ===

    Cycle:   1    2    3    4    5
    BEQ:    [IF] [ID] [EX] [MEM] [WB]
                       ↑
                       Branch resolved HERE (EX stage)
                       Now we know if prediction was correct.

During cycles 2 and 3, the pipeline kept fetching and decoding
instructions ASSUMING the prediction was right. If the prediction
was wrong, those instructions are garbage and must be flushed.

=== Misprediction Scenarios ===

Scenario 1: Predicted NOT TAKEN, but branch IS TAKEN
    The pipeline continued fetching the fall-through instructions
    (PC+4, PC+8), but it should have jumped to the branch target.
    We must flush IF and ID stages (2 instructions) and redirect
    the PC to the branch target.

    Before flush:
        BEQ R1, R2, target  [EX] ← branch IS taken!
        PC+4 (wrong!)       [ID] ← must flush
        PC+8 (wrong!)       [IF] ← must flush

    After flush:
        BEQ R1, R2, target  [EX]
        bubble               [ID]
        bubble               [IF] ← PC redirected to target

Scenario 2: Predicted TAKEN, but branch is NOT TAKEN
    The pipeline fetched instructions from the branch target,
    but it should have continued with the fall-through path.
    Same fix: flush IF and ID, redirect PC.

Scenario 3: Prediction was CORRECT
    No hazard! The pipeline guessed right, and the instructions
    in IF and ID are valid. This is the happy path.

=== Cost of Misprediction ===

Each misprediction costs 2 cycles (the IF and ID stages are wasted).
This is called the "branch penalty." Good branch predictors reduce
the frequency of mispredictions, but they can never eliminate them
entirely. Typical modern CPUs mispredict ~2-5% of branches.
"""

from __future__ import annotations

from hazard_detection.types import HazardAction, HazardResult, PipelineSlot


class ControlHazardDetector:
    """Detects control hazards from branch mispredictions.

    This detector examines the EX stage for branch instructions whose
    actual outcome differs from what was predicted. When a misprediction
    is found, it signals the pipeline to flush the IF and ID stages.

    === Design Decision: Branch Resolution in EX ===

    In our 5-stage pipeline, branches resolve in the EX stage (stage 3).
    Some real CPUs resolve branches earlier (in ID) to reduce the penalty
    from 2 cycles to 1. Others use deep pipelines where the penalty can
    be 10+ cycles. We use EX because it's the classic textbook design
    and because the branch condition (e.g., "is R1 == R2?") requires
    the ALU comparison.

    === What We Flush ===

    When a misprediction is detected, we flush 2 stages:
    1. The IF stage — contains a wrongly-fetched instruction
    2. The ID stage — contains a wrongly-fetched instruction

    The EX stage (where the branch is) is NOT flushed — the branch
    itself is a valid instruction that should complete.
    """

    def detect(self, ex_stage: PipelineSlot) -> HazardResult:
        """Check if a branch in the EX stage was mispredicted.

        Parameters
        ----------
        ex_stage : PipelineSlot
            The instruction currently in the EX stage. We check if it's
            a branch and whether its prediction was correct.

        Returns
        -------
        HazardResult
            FLUSH if mispredicted (with flush_count=2 for IF and ID),
            NONE otherwise.

        === Decision Logic ===

        1. Is EX valid?           No  → NONE (empty stage)
        2. Is EX a branch?        No  → NONE (not a branch)
        3. predicted == actual?    Yes → NONE (correct prediction!)
        4. Otherwise               → FLUSH (misprediction!)

        This is beautifully simple because we only need to compare
        one boolean (branch_predicted_taken) against another
        (branch_taken). All the complexity of prediction lives in the
        branch predictor — we just check if it was right.
        """
        # No instruction in EX? Nothing to check.
        if not ex_stage.valid:
            return HazardResult(
                action=HazardAction.NONE,
                reason="EX stage is empty (bubble)",
            )

        # Not a branch? No control hazard possible.
        if not ex_stage.is_branch:
            return HazardResult(
                action=HazardAction.NONE,
                reason="EX stage instruction is not a branch",
            )

        # Branch prediction was correct — no hazard!
        # This is the common case with a good branch predictor.
        if ex_stage.branch_predicted_taken == ex_stage.branch_taken:
            return HazardResult(
                action=HazardAction.NONE,
                reason=(
                    f"branch at PC=0x{ex_stage.pc:04X} correctly predicted "
                    f"{'taken' if ex_stage.branch_taken else 'not taken'}"
                ),
            )

        # === Misprediction detected! ===
        # The predicted and actual outcomes disagree.
        # We must flush the two pipeline stages that contain wrong
        # instructions (IF and ID).
        if ex_stage.branch_taken:
            # Predicted NOT taken, but actually TAKEN.
            # The pipeline was fetching fall-through instructions,
            # but should have jumped to the branch target.
            direction = "predicted not-taken, actually taken"
        else:
            # Predicted TAKEN, but actually NOT taken.
            # The pipeline was fetching from the branch target,
            # but should have continued with fall-through.
            direction = "predicted taken, actually not-taken"

        return HazardResult(
            action=HazardAction.FLUSH,
            flush_count=2,  # flush IF and ID stages
            reason=(
                f"branch misprediction at PC=0x{ex_stage.pc:04X}: "
                f"{direction} — flushing IF and ID stages"
            ),
        )
