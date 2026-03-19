"""Combined hazard detection unit — the pipeline's traffic controller.

=== What Is the Hazard Unit? ===

The hazard unit is a single hardware module that runs ALL hazard
detectors every clock cycle and returns ONE decision to the pipeline.
Think of it as an air traffic controller: it monitors all the "planes"
(instructions) in the pipeline and issues commands to prevent collisions.

=== Why a Combined Unit? ===

Multiple hazard types can occur simultaneously:
- A data hazard (RAW) AND a control hazard (branch misprediction)
  could happen at the same time.
- A structural hazard AND a data hazard could overlap.

The combined unit resolves conflicts between detectors by using a
strict priority system.

=== Priority System ===

    FLUSH > STALL > FORWARD > NONE

    1. FLUSH (highest priority):
       A branch misprediction means we're executing WRONG instructions.
       Nothing else matters — flush the pipeline immediately. Even if
       there's a data hazard, it's on a wrong instruction that's about
       to be thrown away.

    2. STALL (second priority):
       We can't proceed because data isn't ready yet. The pipeline must
       freeze. Even if forwarding could help with one register, a stall
       on another register takes precedence.

    3. FORWARD (third priority):
       A data dependency exists, but we can resolve it by forwarding.
       This is the "best case" for hazards — zero penalty.

    4. NONE (lowest priority):
       All clear. The pipeline flows normally.

=== Statistics Tracking ===

The hazard unit maintains a history of all decisions, which is useful
for performance analysis:
- stall_count: total stall cycles (directly reduces throughput)
- flush_count: total flushes (each costs 2 wasted cycles)
- forward_count: total forwards (zero penalty, but indicates
  dependency density in the code)

A well-optimized program (or compiler) minimizes stalls and flushes.
"""

from __future__ import annotations

from hazard_detection.control_hazard import ControlHazardDetector
from hazard_detection.data_hazard import DataHazardDetector
from hazard_detection.structural_hazard import StructuralHazardDetector
from hazard_detection.types import HazardAction, HazardResult, PipelineSlot


class HazardUnit:
    """Combined hazard detection unit — runs all detectors each cycle.

    === Usage Example ===

        # Create the unit (configurable hardware resources)
        unit = HazardUnit(num_alus=1, num_fp_units=1, split_caches=True)

        # Each cycle, pass in the four pipeline stages:
        result = unit.check(if_stage, id_stage, ex_stage, mem_stage)

        # Act on the result:
        if result.action == HazardAction.FLUSH:
            pipeline.flush_if_and_id()
        elif result.action == HazardAction.STALL:
            pipeline.insert_bubble()
        elif result.action in (HazardAction.FORWARD_FROM_EX,
                               HazardAction.FORWARD_FROM_MEM):
            pipeline.forward(result.forwarded_value)
        else:
            pipeline.proceed_normally()

        # Check performance stats:
        print(f"Total stalls: {unit.stall_count}")
        print(f"Total flushes: {unit.flush_count}")
    """

    def __init__(
        self,
        num_alus: int = 1,
        num_fp_units: int = 1,
        split_caches: bool = True,
    ) -> None:
        """Create a hazard unit with configurable hardware resources.

        Parameters
        ----------
        num_alus : int
            Number of integer ALUs. Affects structural hazard detection.
        num_fp_units : int
            Number of floating-point units. Affects structural hazard detection.
        split_caches : bool
            Whether L1I and L1D caches are separate. Affects structural
            hazard detection for memory port conflicts.
        """
        self.data_detector = DataHazardDetector()
        self.control_detector = ControlHazardDetector()
        self.structural_detector = StructuralHazardDetector(
            num_alus=num_alus,
            num_fp_units=num_fp_units,
            split_caches=split_caches,
        )
        self._history: list[HazardResult] = []

    def check(
        self,
        if_stage: PipelineSlot,
        id_stage: PipelineSlot,
        ex_stage: PipelineSlot,
        mem_stage: PipelineSlot,
    ) -> HazardResult:
        """Run all hazard detectors and return the highest-priority action.

        This method is called ONCE per clock cycle. It runs all three
        hazard detectors and returns the single most critical action
        the pipeline should take.

        Parameters
        ----------
        if_stage : PipelineSlot
            Instruction being fetched.
        id_stage : PipelineSlot
            Instruction being decoded.
        ex_stage : PipelineSlot
            Instruction being executed.
        mem_stage : PipelineSlot
            Instruction in memory access stage.

        Returns
        -------
        HazardResult
            The highest-priority hazard result. The pipeline should act
            on this result's action field.

        === Detection Order ===

        We run all detectors regardless of what earlier ones found,
        because the history should record what WOULD have happened.
        The final result uses the highest-priority action.

        However, the ORDER of detection doesn't matter for correctness —
        only the priority comparison at the end determines the result.
        We check control first because flushes are most critical.
        """
        # --- 1. Control hazards (check first — highest priority) ---
        # If there's a misprediction, everything else is moot.
        control_result = self.control_detector.detect(ex_stage)

        # --- 2. Data hazards (forwarding or stalling) ---
        data_result = self.data_detector.detect(id_stage, ex_stage, mem_stage)

        # --- 3. Structural hazards (resource conflicts) ---
        structural_result = self.structural_detector.detect(
            id_stage=id_stage,
            ex_stage=ex_stage,
            if_stage=if_stage,
            mem_stage=mem_stage,
        )

        # --- Pick the highest-priority result ---
        # Priority: FLUSH > STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
        final_result = _pick_highest_priority(
            control_result, data_result, structural_result
        )

        # Record in history for statistics.
        self._history.append(final_result)

        return final_result

    @property
    def history(self) -> list[HazardResult]:
        """Complete history of hazard results, one per cycle.

        Useful for debugging and performance analysis. Each entry
        corresponds to one call to check().
        """
        return list(self._history)

    @property
    def stall_count(self) -> int:
        """Total stall cycles across all instructions.

        Each stall wastes one pipeline cycle. A high stall count
        indicates the code has many data dependencies that can't be
        resolved by forwarding (typically load-use patterns).

        Returns
        -------
        int
            Sum of stall_cycles across all history entries.
        """
        return sum(r.stall_cycles for r in self._history)

    @property
    def flush_count(self) -> int:
        """Total pipeline flushes (branch mispredictions).

        Each flush wastes 2 cycles (IF and ID stages are discarded).
        A high flush count indicates the branch predictor is struggling,
        or the code has many hard-to-predict branches.

        Returns
        -------
        int
            Number of FLUSH actions in the history.
        """
        return sum(
            1 for r in self._history if r.action == HazardAction.FLUSH
        )

    @property
    def forward_count(self) -> int:
        """Total forwarding operations.

        Forwarding resolves data hazards with zero penalty. A high
        forward count isn't bad — it means the forwarding hardware
        is earning its keep. Without it, these would all be stalls.

        Returns
        -------
        int
            Number of FORWARD_FROM_EX or FORWARD_FROM_MEM actions.
        """
        return sum(
            1
            for r in self._history
            if r.action
            in (HazardAction.FORWARD_FROM_EX, HazardAction.FORWARD_FROM_MEM)
        )


def _pick_highest_priority(*results: HazardResult) -> HazardResult:
    """Return the hazard result with the highest-priority action.

    === Priority Map ===

        FLUSH           = 4  (most urgent — wrong instructions in pipeline)
        STALL           = 3  (urgent — would get wrong data)
        FORWARD_FROM_EX = 2  (optimization — grab data from EX)
        FORWARD_FROM_MEM= 1  (optimization — grab data from MEM)
        NONE            = 0  (all clear)

    Parameters
    ----------
    *results : HazardResult
        Variable number of hazard results to compare.

    Returns
    -------
    HazardResult
        The one with the highest-priority action. Ties go to the first one
        encountered (which is fine — same priority means same urgency).
    """
    priority = {
        HazardAction.NONE: 0,
        HazardAction.FORWARD_FROM_MEM: 1,
        HazardAction.FORWARD_FROM_EX: 2,
        HazardAction.STALL: 3,
        HazardAction.FLUSH: 4,
    }

    best = results[0]
    for result in results[1:]:
        if priority[result.action] > priority[best.action]:
            best = result
    return best
