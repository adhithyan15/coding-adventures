"""Structural hazard detection — when hardware resources collide.

=== What Is a Structural Hazard? ===

A structural hazard occurs when two instructions need the same hardware
resource in the same clock cycle. It's like two people trying to use
the same bathroom at the same time — someone has to wait.

=== Common Examples ===

1. **Single-Port Memory** (fetch + data access conflict):
   The CPU needs to fetch an instruction (IF stage) AND load/store
   data (MEM stage) in the same cycle, but the memory only has one
   port (one "door" for reading/writing).

       Cycle 4:
       Instr A:  [IF] ← needs to read instruction from memory
       Instr B:       [MEM] ← needs to read/write data from memory
                 ↑ CONFLICT! Memory can only serve one request.

   Solution: Split L1 cache into L1I (instructions) and L1D (data).
   Each has its own port, so fetch and data access happen in parallel.
   This is what ALL modern CPUs do.

2. **Single ALU** (two ALU instructions at once):
   In a simple pipeline, there's only one ALU. If two instructions
   both need the ALU in the same cycle, one must wait. This mostly
   matters for superscalar CPUs that try to execute multiple
   instructions per cycle.

       IF → ID → [EX] → MEM → WB   ← uses ALU
       IF → ID → [EX] → MEM → WB   ← also uses ALU (superscalar)
                  ↑ CONFLICT! Only one ALU available.

   Solution: Add more ALUs (superscalar CPUs have 2-8+ ALUs).

3. **Single FP Unit** (two floating-point instructions at once):
   Floating-point units are expensive (lots of transistors), so many
   CPUs have fewer FP units than integer ALUs. Two FP instructions
   may conflict.

=== For Our Basic 5-Stage Pipeline ===

With split L1I/L1D caches (the default), structural hazards are rare.
The main case is when two instructions in adjacent stages both need
the same execution unit (ALU or FP unit). We detect this for
completeness and for future superscalar extensions.

=== Configurability ===

The detector is configurable:
- num_alus: How many ALU units are available (default: 1)
- num_fp_units: How many FP units are available (default: 1)
- split_caches: Whether L1I and L1D are separate (default: True)

With enough resources, structural hazards disappear entirely.
This is exactly how real CPUs evolved — adding more hardware to
eliminate stalls.
"""

from __future__ import annotations

from hazard_detection.types import HazardAction, HazardResult, PipelineSlot


class StructuralHazardDetector:
    """Detects structural hazards — two instructions competing for hardware.

    === How Detection Works ===

    We check two things each cycle:

    1. **Execution unit conflict**: Is the ID-stage instruction about to
       enter EX, while the EX-stage instruction is still using the same
       execution unit? With 1 ALU, two ALU instructions can't both be
       in EX. With 2 ALUs, they can.

       The check is:
       - Both ID and EX need the ALU, and we have fewer ALUs than needed?
       - Both ID and EX need the FP unit, and we have fewer FP units?

       For a single-issue pipeline (1 instruction enters EX per cycle),
       this only matters when the execution unit is MULTI-CYCLE (i.e.,
       the EX-stage instruction hasn't finished yet and is still occupying
       the unit). For simplicity, we flag the conflict whenever both stages
       want the same unit and there's only one of it.

    2. **Memory port conflict**: Is IF trying to fetch while MEM is
       accessing data, and we have a single (shared) cache?

       With split_caches=True (default), no conflict — each cache has
       its own port. With split_caches=False, fetch and data access
       compete for the single memory port.
    """

    def __init__(
        self,
        num_alus: int = 1,
        num_fp_units: int = 1,
        split_caches: bool = True,
    ) -> None:
        """Configure the structural hazard detector.

        Parameters
        ----------
        num_alus : int
            Number of integer ALU units. With 1 ALU (default), two
            ALU-using instructions in EX simultaneously causes a stall.
            With 2+ ALUs, they can execute in parallel.

        num_fp_units : int
            Number of floating-point execution units. Same logic as ALUs.

        split_caches : bool
            If True (default), L1I and L1D are separate — no memory port
            conflict between IF and MEM stages. If False, a shared cache
            means IF and MEM can't access memory in the same cycle.
        """
        self._num_alus = num_alus
        self._num_fp_units = num_fp_units
        self._split_caches = split_caches

    def detect(
        self,
        id_stage: PipelineSlot,
        ex_stage: PipelineSlot,
        if_stage: PipelineSlot | None = None,
        mem_stage: PipelineSlot | None = None,
    ) -> HazardResult:
        """Check for structural hazards between pipeline stages.

        Parameters
        ----------
        id_stage : PipelineSlot
            Instruction about to enter EX. We check if it needs the same
            resources as the instruction currently in EX.
        ex_stage : PipelineSlot
            Instruction currently in EX. Occupying an execution unit.
        if_stage : PipelineSlot | None
            Instruction being fetched. Used for memory port conflict check.
        mem_stage : PipelineSlot | None
            Instruction accessing memory. Used for memory port conflict check.

        Returns
        -------
        HazardResult
            STALL if a structural hazard is detected, NONE otherwise.
        """
        # --- Check execution unit conflicts ---
        # Both instructions must be valid (non-bubble) to conflict.
        exec_result = self._check_execution_unit_conflict(id_stage, ex_stage)
        if exec_result.action != HazardAction.NONE:
            return exec_result

        # --- Check memory port conflicts ---
        if if_stage is not None and mem_stage is not None:
            mem_result = self._check_memory_port_conflict(if_stage, mem_stage)
            if mem_result.action != HazardAction.NONE:
                return mem_result

        return HazardResult(
            action=HazardAction.NONE,
            reason="no structural hazards — all resources available",
        )

    def _check_execution_unit_conflict(
        self,
        id_stage: PipelineSlot,
        ex_stage: PipelineSlot,
    ) -> HazardResult:
        """Check if ID and EX need the same execution unit.

        === Logic ===

        For ALU conflict:
            Both id_stage.uses_alu AND ex_stage.uses_alu must be True,
            AND num_alus must be < 2 (only 1 ALU to share).

        For FP conflict:
            Both id_stage.uses_fp AND ex_stage.uses_fp must be True,
            AND num_fp_units must be < 2.

        === Truth Table for ALU Conflict (1 ALU) ===

            ID.uses_alu | EX.uses_alu | Conflict?
            -----------+-----------+----------
            False      | False     | No
            False      | True      | No  (ID doesn't need ALU)
            True       | False     | No  (EX doesn't need ALU)
            True       | True      | YES (both need the 1 ALU)

        Parameters
        ----------
        id_stage, ex_stage : PipelineSlot
            The instructions in the ID and EX stages.

        Returns
        -------
        HazardResult
            STALL if conflict found, NONE otherwise.
        """
        if not id_stage.valid or not ex_stage.valid:
            return HazardResult(
                action=HazardAction.NONE,
                reason="one or both stages are empty (bubble)",
            )

        # ALU conflict: both need ALU, but we only have 1.
        if id_stage.uses_alu and ex_stage.uses_alu and self._num_alus < 2:
            return HazardResult(
                action=HazardAction.STALL,
                stall_cycles=1,
                reason=(
                    f"structural hazard: both ID (PC=0x{id_stage.pc:04X}) "
                    f"and EX (PC=0x{ex_stage.pc:04X}) need the ALU, "
                    f"but only {self._num_alus} ALU available"
                ),
            )

        # FP unit conflict: both need FP, but we only have 1.
        if (
            id_stage.uses_fp
            and ex_stage.uses_fp
            and self._num_fp_units < 2
        ):
            return HazardResult(
                action=HazardAction.STALL,
                stall_cycles=1,
                reason=(
                    f"structural hazard: both ID (PC=0x{id_stage.pc:04X}) "
                    f"and EX (PC=0x{ex_stage.pc:04X}) need the FP unit, "
                    f"but only {self._num_fp_units} FP unit available"
                ),
            )

        return HazardResult(
            action=HazardAction.NONE,
            reason="no execution unit conflict",
        )

    def _check_memory_port_conflict(
        self,
        if_stage: PipelineSlot,
        mem_stage: PipelineSlot,
    ) -> HazardResult:
        """Check if IF and MEM both need the memory bus.

        This only matters when split_caches is False (shared L1 cache).
        With split caches, IF reads from L1I and MEM reads/writes L1D
        independently — no conflict.

        === When Does This Happen? ===

        IF always needs memory (to fetch the next instruction).
        MEM only needs memory when it's a load (mem_read) or store
        (mem_write). So the conflict occurs when:

            if_stage.valid AND mem_stage.valid AND
            (mem_stage.mem_read OR mem_stage.mem_write) AND
            NOT split_caches

        Parameters
        ----------
        if_stage : PipelineSlot
            Instruction being fetched (always reads from instruction memory).
        mem_stage : PipelineSlot
            Instruction in memory stage (reads/writes data memory).

        Returns
        -------
        HazardResult
            STALL if shared cache conflict, NONE otherwise.
        """
        # With split caches, fetch and data access never conflict.
        if self._split_caches:
            return HazardResult(
                action=HazardAction.NONE,
                reason="split caches — no memory port conflict",
            )

        # Both stages must be valid and MEM must actually access memory.
        if (
            if_stage.valid
            and mem_stage.valid
            and (mem_stage.mem_read or mem_stage.mem_write)
        ):
            access_type = "load" if mem_stage.mem_read else "store"
            return HazardResult(
                action=HazardAction.STALL,
                stall_cycles=1,
                reason=(
                    f"structural hazard: IF (fetch at PC=0x{if_stage.pc:04X}) "
                    f"and MEM ({access_type} at PC=0x{mem_stage.pc:04X}) "
                    f"both need the shared memory bus"
                ),
            )

        return HazardResult(
            action=HazardAction.NONE,
            reason="no memory port conflict",
        )
