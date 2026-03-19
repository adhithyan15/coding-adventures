"""Data hazard detection — the most common pipeline hazard.

=== What Is a Data Hazard? ===

A data hazard occurs when an instruction depends on the result of a
previous instruction that hasn't finished yet. In a pipelined CPU,
multiple instructions are "in flight" simultaneously, so an instruction
might try to READ a register before the previous instruction has
WRITTEN its result.

=== The Three Types of Data Hazards ===

1. **RAW (Read After Write)** — the dangerous one:
   An instruction tries to read a register that a previous instruction
   will write, but hasn't yet. This is the "true dependency."

       ADD R1, R2, R3    ← writes R1 (result available in WB, cycle 5)
       SUB R4, R1, R5    ← reads R1 (needs it in ID, cycle 3) — HAZARD!

   Without intervention, SUB reads the OLD value of R1 (stale data).

2. **WAR (Write After Read)** — rare in simple pipelines:
   An instruction writes a register before a previous instruction reads it.
   In our 5-stage pipeline, writes happen in WB (stage 5) and reads in ID
   (stage 2), so WAR can't happen — earlier instructions always read first.
   WAR hazards appear in out-of-order processors.

3. **WAW (Write After Write)** — also rare in simple pipelines:
   Two instructions write the same register, and the second completes
   before the first. In an in-order pipeline, this can't happen because
   instructions complete in order. WAW hazards appear in superscalar and
   out-of-order designs.

This module focuses on RAW hazards — the only type that occurs in our
classic 5-stage in-order pipeline.

=== How We Resolve RAW Hazards ===

Strategy 1: FORWARDING (a.k.a. bypassing)
    Instead of waiting for the result to reach WB, we "forward" it
    directly from the stage where it's available:

    - Forward from EX: The ALU just computed the result. Wire it back
      to the ID stage input. Zero penalty!

          Cycle:  1    2    3    4    5
          ADD:   [IF] [ID] [EX] [MEM] [WB]
          SUB:        [IF] [ID] ←─┘ forward from EX

    - Forward from MEM: The result is in the MEM stage (either an ALU
      result passing through, or a load that just completed).

          Cycle:  1    2    3    4    5    6
          ADD:   [IF] [ID] [EX] [MEM] [WB]
          NOP:        [IF] [ID] [EX]  [MEM]
          SUB:              [IF] [ID] ←──┘ forward from MEM

Strategy 2: STALLING (when forwarding can't help)
    The one case forwarding fails: a LOAD followed immediately by a USE.
    The load value isn't available until AFTER the MEM stage, but the
    next instruction needs it IN the EX stage. There's a 1-cycle gap
    that no wire can bridge:

          Cycle:  1    2    3    4    5
          LW:    [IF] [ID] [EX] [MEM] ← value available HERE
          ADD:        [IF] [ID] [EX]  ← needs value HERE (too early!)

    Solution: insert a 1-cycle bubble (stall), then forward from MEM:

          Cycle:  1    2    3    4    5    6
          LW:    [IF] [ID] [EX] [MEM] [WB]
          ADD:        [IF] [ID] STALL [ID*] [EX] ← forward from MEM
                                       └── re-read with forwarded value
"""

from __future__ import annotations

from hazard_detection.types import HazardAction, HazardResult, PipelineSlot


class DataHazardDetector:
    """Detects Read After Write (RAW) data hazards and resolves them.

    The detector examines the instruction in the ID (decode) stage and
    compares its source registers against the destination registers of
    instructions in the EX and MEM stages.

    === Decision Flow ===

    For each source register of the ID-stage instruction:

        1. Does it match the dest_reg of the EX-stage instruction?
           a. Is the EX instruction a LOAD? → STALL (load-use hazard)
           b. Otherwise → FORWARD from EX (value is ready)

        2. Does it match the dest_reg of the MEM-stage instruction?
           → FORWARD from MEM (value is ready or just loaded)

        3. No match? → No hazard for this register.

    If multiple source registers have hazards, we take the most severe
    action (STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE).

    === Why EX Has Higher Priority Than MEM ===

    If both EX and MEM write the same register, the EX instruction is
    NEWER (entered the pipeline later), so its value is the correct one.

    Example:
        ADD R1, R2, R3    ← in MEM stage (older, writes R1)
        MUL R1, R4, R5    ← in EX stage  (newer, also writes R1)
        SUB R6, R1, R7    ← in ID stage  (reads R1)

    SUB should get R1 from MUL (EX), not ADD (MEM), because MUL
    executes after ADD in program order.
    """

    def detect(
        self,
        id_stage: PipelineSlot,
        ex_stage: PipelineSlot,
        mem_stage: PipelineSlot,
    ) -> HazardResult:
        """Check for data hazards between the ID stage and later stages.

        Parameters
        ----------
        id_stage : PipelineSlot
            The instruction currently being decoded. We check if its
            source registers conflict with EX/MEM destinations.
        ex_stage : PipelineSlot
            The instruction currently executing. Its result may be
            available for forwarding (unless it's a load).
        mem_stage : PipelineSlot
            The instruction in the memory stage. Its result (or loaded
            value) is available for forwarding.

        Returns
        -------
        HazardResult
            The action to take: NONE, FORWARD_FROM_EX, FORWARD_FROM_MEM,
            or STALL (for load-use hazards).
        """
        # If ID stage is empty (bubble), there's nothing to check.
        if not id_stage.valid:
            return HazardResult(
                action=HazardAction.NONE,
                reason="ID stage is empty (bubble)",
            )

        # If the instruction has no source registers, it can't have a
        # data dependency. Examples: NOP, unconditional jump.
        if not id_stage.source_regs:
            return HazardResult(
                action=HazardAction.NONE,
                reason="instruction has no source registers",
            )

        # Check each source register for conflicts with EX and MEM.
        # We track the "worst" hazard found across all source registers.
        worst_result = HazardResult(
            action=HazardAction.NONE,
            reason="no data dependencies detected",
        )

        for src_reg in id_stage.source_regs:
            result = self._check_single_register(src_reg, ex_stage, mem_stage)
            worst_result = self._pick_higher_priority(worst_result, result)

        return worst_result

    def _check_single_register(
        self,
        src_reg: int,
        ex_stage: PipelineSlot,
        mem_stage: PipelineSlot,
    ) -> HazardResult:
        """Check one source register against EX and MEM stage destinations.

        === Priority: EX before MEM ===

        We check EX first because if both EX and MEM write the same
        register, the EX instruction is the more recent one (in program
        order), so its value is what the ID instruction should see.

        Parameters
        ----------
        src_reg : int
            The register number that the ID-stage instruction reads.
        ex_stage : PipelineSlot
            Instruction in the EX (execute) stage.
        mem_stage : PipelineSlot
            Instruction in the MEM (memory) stage.

        Returns
        -------
        HazardResult
            The hazard (if any) for this particular source register.
        """
        # --- Check EX stage first (higher priority — newer instruction) ---
        if (
            ex_stage.valid
            and ex_stage.dest_reg is not None
            and ex_stage.dest_reg == src_reg
        ):
            # Is the EX-stage instruction a LOAD?
            # Loads don't have their value until AFTER the MEM stage,
            # so we can't forward from EX — we must stall 1 cycle.
            if ex_stage.mem_read:
                return HazardResult(
                    action=HazardAction.STALL,
                    stall_cycles=1,
                    reason=(
                        f"load-use hazard: R{src_reg} is being loaded "
                        f"by instruction at PC=0x{ex_stage.pc:04X} — "
                        f"must stall 1 cycle"
                    ),
                )

            # Not a load — the ALU result is available right now.
            # Forward it directly from EX to ID.
            return HazardResult(
                action=HazardAction.FORWARD_FROM_EX,
                forwarded_value=ex_stage.dest_value,
                forwarded_from="EX",
                reason=(
                    f"RAW hazard on R{src_reg}: forwarding value "
                    f"{ex_stage.dest_value} from EX stage "
                    f"(instruction at PC=0x{ex_stage.pc:04X})"
                ),
            )

        # --- Check MEM stage (lower priority — older instruction) ---
        if (
            mem_stage.valid
            and mem_stage.dest_reg is not None
            and mem_stage.dest_reg == src_reg
        ):
            return HazardResult(
                action=HazardAction.FORWARD_FROM_MEM,
                forwarded_value=mem_stage.dest_value,
                forwarded_from="MEM",
                reason=(
                    f"RAW hazard on R{src_reg}: forwarding value "
                    f"{mem_stage.dest_value} from MEM stage "
                    f"(instruction at PC=0x{mem_stage.pc:04X})"
                ),
            )

        # No conflict for this register.
        return HazardResult(
            action=HazardAction.NONE,
            reason=f"R{src_reg} has no pending writes in EX or MEM",
        )

    @staticmethod
    def _pick_higher_priority(
        a: HazardResult, b: HazardResult
    ) -> HazardResult:
        """Return whichever hazard result is more severe.

        Priority order (most severe first):
            STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE

        Why this order?
        - STALL means we CANNOT proceed — the pipeline must freeze.
          This is the most severe because ignoring it causes wrong results.
        - FORWARD_FROM_EX is preferred over MEM because EX-stage data is
          from a more recent instruction (and forwarding from EX is faster).
        - FORWARD_FROM_MEM is still good — avoids a stall.
        - NONE means no action needed.

        Parameters
        ----------
        a, b : HazardResult
            Two hazard results to compare.

        Returns
        -------
        HazardResult
            The more severe of the two.
        """
        priority = {
            HazardAction.NONE: 0,
            HazardAction.FORWARD_FROM_MEM: 1,
            HazardAction.FORWARD_FROM_EX: 2,
            HazardAction.STALL: 3,
            HazardAction.FLUSH: 4,
        }
        if priority[b.action] > priority[a.action]:
            return b
        return a
