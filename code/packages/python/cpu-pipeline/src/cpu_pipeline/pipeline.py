"""Pipeline -- the main CPU pipeline simulator.

The pipeline uses dependency injection: instead of importing the cache,
hazard detection, and branch predictor packages, the pipeline accepts
callback functions. This decouples the pipeline from specific implementations.

Analogy: the pipeline is like a conveyor belt. It does not care what
is ON the belt (that is the callbacks' job). It only cares about
MOVING items along the belt and handling stalls/flushes.
"""

from __future__ import annotations

import copy
from collections.abc import Callable
from dataclasses import dataclass
from enum import IntEnum

from cpu_pipeline.snapshot import PipelineSnapshot, PipelineStats
from cpu_pipeline.token import (
    PipelineConfig,
    PipelineToken,
    StageCategory,
    new_bubble,
    new_token,
)

# =========================================================================
# Callback function types
# =========================================================================

# FetchFunc fetches the raw instruction bits at the given program counter.
# Signature: (pc: int) -> int (raw instruction bits)
FetchFunc = Callable[[int], int]

# DecodeFunc decodes a raw instruction and fills in the token's fields.
# Signature: (raw_instruction: int, token: PipelineToken) -> PipelineToken
DecodeFunc = Callable[[int, PipelineToken], PipelineToken]

# ExecuteFunc performs the ALU operation for the instruction.
# Signature: (token: PipelineToken) -> PipelineToken
ExecuteFunc = Callable[[PipelineToken], PipelineToken]

# MemoryFunc performs the memory access (load/store) for the instruction.
# Signature: (token: PipelineToken) -> PipelineToken
MemoryFunc = Callable[[PipelineToken], PipelineToken]

# WritebackFunc writes the instruction's result to the register file.
# Signature: (token: PipelineToken) -> None
WritebackFunc = Callable[[PipelineToken], None]


# =========================================================================
# HazardAction -- what the hazard detector tells the pipeline to do
# =========================================================================


class HazardAction(IntEnum):
    """Represents the action the hazard unit tells the pipeline to take.

    These are "traffic signals" for the pipeline:
        NONE:             Green light -- pipeline flows normally
        FORWARD_FROM_EX:  Shortcut -- grab value from EX stage output
        FORWARD_FROM_MEM: Shortcut -- grab value from MEM stage output
        STALL:            Red light -- freeze earlier stages, insert bubble
        FLUSH:            Emergency stop -- discard speculative instructions

    Priority: FLUSH > STALL > FORWARD > NONE
    """

    NONE = 0
    FORWARD_FROM_EX = 1
    FORWARD_FROM_MEM = 2
    STALL = 3
    FLUSH = 4

    def __str__(self) -> str:
        """Return a human-readable name for the hazard action."""
        names = {
            HazardAction.NONE: "NONE",
            HazardAction.FORWARD_FROM_EX: "FORWARD_FROM_EX",
            HazardAction.FORWARD_FROM_MEM: "FORWARD_FROM_MEM",
            HazardAction.STALL: "STALL",
            HazardAction.FLUSH: "FLUSH",
        }
        return names.get(self, "UNKNOWN")


@dataclass
class HazardResponse:
    """The full response from the hazard detection callback.

    Tells the pipeline what to do and provides additional context
    (forwarded values, stall duration, flush target).
    """

    action: HazardAction = HazardAction.NONE
    forward_value: int = 0  # Value to forward (only for FORWARD actions)
    forward_source: str = ""  # Stage that provided the forwarded value
    stall_stages: int = 0  # Number of stages to stall (typically 1)
    flush_count: int = 0  # Number of stages to flush on a misprediction
    redirect_pc: int = 0  # Correct PC to fetch from after a flush


# HazardFunc checks for hazards given the current pipeline stage contents.
# Signature: (stages: list[PipelineToken | None]) -> HazardResponse
HazardFunc = Callable[[list[PipelineToken | None]], HazardResponse]

# PredictFunc predicts the next PC given the current PC.
# Signature: (pc: int) -> int (predicted next PC)
PredictFunc = Callable[[int], int]


# =========================================================================
# Pipeline -- the main pipeline struct
# =========================================================================


class Pipeline:
    """A configurable N-stage instruction pipeline.

    How it Works:

    The pipeline is a list of "slots", one per stage. Each slot holds a
    PipelineToken (or None if the stage is empty). On each clock cycle
    (call to step()):

      1. Check for hazards (via hazard_fn callback)
      2. If stalled: freeze stages before the stall point, insert bubble
      3. If flushing: replace speculative stages with bubbles
      4. Otherwise: shift all tokens one stage forward
      5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
      6. Record a snapshot for tracing

    Example: 5-cycle execution of ADD instruction:

        Cycle 1: IF  -- fetch instruction at PC, ask branch predictor for next PC
        Cycle 2: ID  -- decode: extract opcode=ADD, Rd=1, Rs1=2, Rs2=3
        Cycle 3: EX  -- execute: ALUResult = Reg[2] + Reg[3]
        Cycle 4: MEM -- memory: pass through (ADD doesn't access memory)
        Cycle 5: WB  -- writeback: Reg[1] = ALUResult
    """

    def __init__(
        self,
        config: PipelineConfig,
        fetch_fn: FetchFunc,
        decode_fn: DecodeFunc,
        execute_fn: ExecuteFunc,
        memory_fn: MemoryFunc,
        writeback_fn: WritebackFunc,
    ) -> None:
        """Create a new pipeline with the given configuration and callbacks.

        The configuration is validated before use. All five stage callbacks
        are required; hazard and predict callbacks are optional (set via
        set_hazard_func / set_predict_func).

        Raises:
            ValueError: If the configuration is invalid.
        """
        config.validate()

        self._config = config
        self._stages: list[PipelineToken | None] = [None] * config.num_stages()
        self._pc: int = 0
        self._cycle: int = 0
        self._halted: bool = False
        self._stats = PipelineStats()
        self._history: list[PipelineSnapshot] = []

        # Required callbacks
        self._fetch_fn = fetch_fn
        self._decode_fn = decode_fn
        self._execute_fn = execute_fn
        self._memory_fn = memory_fn
        self._writeback_fn = writeback_fn

        # Optional callbacks
        self._hazard_fn: HazardFunc | None = None
        self._predict_fn: PredictFunc | None = None

    def set_hazard_func(self, fn: HazardFunc) -> None:
        """Set the optional hazard detection callback."""
        self._hazard_fn = fn

    def set_predict_func(self, fn: PredictFunc) -> None:
        """Set the optional branch prediction callback."""
        self._predict_fn = fn

    def set_pc(self, pc: int) -> None:
        """Set the program counter (address of next instruction to fetch)."""
        self._pc = pc

    @property
    def pc(self) -> int:
        """Return the current program counter."""
        return self._pc

    @property
    def cycle(self) -> int:
        """Return the current cycle number."""
        return self._cycle

    @property
    def halted(self) -> bool:
        """Return True if a halt instruction has reached the last stage."""
        return self._halted

    def stats(self) -> PipelineStats:
        """Return a copy of the current execution statistics."""
        return copy.copy(self._stats)

    def config(self) -> PipelineConfig:
        """Return the pipeline configuration."""
        return self._config

    def step(self) -> PipelineSnapshot:
        """Advance the pipeline by one clock cycle.

        This is the heart of the pipeline simulator. Each call to step()
        corresponds to one rising clock edge in hardware.

        Step Algorithm:
          1. If halted, return the current snapshot (do nothing).
          2. Increment the cycle counter.
          3. Check for hazards by calling hazard_fn (if set).
          4. Handle the hazard response (flush, stall, forward, or none).
          5. Advance tokens through stages.
          6. Execute stage callbacks on each token.
          7. Update statistics.
          8. Record a snapshot and return it.
        """
        if self._halted:
            return self._take_snapshot()

        self._cycle += 1
        self._stats.total_cycles += 1
        num_stages = self._config.num_stages()

        # --- Phase 1: Check for hazards ---
        hazard = HazardResponse(action=HazardAction.NONE)
        if self._hazard_fn is not None:
            stages_copy = list(self._stages)
            hazard = self._hazard_fn(stages_copy)

        # --- Phase 2: Compute next state ---
        next_stages: list[PipelineToken | None] = [None] * num_stages
        stalled = False
        flushing = False

        if hazard.action == HazardAction.FLUSH:
            # FLUSH: Replace speculative stages with bubbles.
            flushing = True
            self._stats.flush_cycles += 1

            # Determine how many stages to flush (from the front).
            flush_count = hazard.flush_count
            if flush_count <= 0:
                for i, s in enumerate(self._config.stages):
                    if s.category == StageCategory.EXECUTE:
                        flush_count = i
                        break
                if flush_count <= 0:
                    flush_count = 1
            if flush_count > num_stages:
                flush_count = num_stages

            # Shift non-flushed stages forward (from back to front).
            for i in range(num_stages - 1, flush_count - 1, -1):
                if i > 0 and i - 1 >= flush_count:
                    next_stages[i] = self._stages[i - 1]
                elif i > 0:
                    bubble = new_bubble()
                    bubble.stage_entered[self._config.stages[i].name] = self._cycle
                    next_stages[i] = bubble
                else:
                    next_stages[i] = self._stages[i]

            # Replace flushed stages with bubbles.
            for i in range(flush_count):
                bubble = new_bubble()
                bubble.stage_entered[self._config.stages[i].name] = self._cycle
                next_stages[i] = bubble

            # Redirect PC and fetch from the correct target.
            self._pc = hazard.redirect_pc
            tok = self._fetch_new_instruction()
            next_stages[0] = tok

        elif hazard.action == HazardAction.STALL:
            # STALL: Freeze earlier stages and insert a bubble.
            stalled = True
            self._stats.stall_cycles += 1

            # Find the stall insertion point.
            stall_point = hazard.stall_stages
            if stall_point <= 0:
                for i, s in enumerate(self._config.stages):
                    if s.category == StageCategory.EXECUTE:
                        stall_point = i
                        break
                if stall_point <= 0:
                    stall_point = 1
            if stall_point >= num_stages:
                stall_point = num_stages - 1

            # Stages AFTER the stall point advance normally.
            for i in range(num_stages - 1, stall_point, -1):
                next_stages[i] = self._stages[i - 1]

            # Insert bubble at the stall point.
            bubble = new_bubble()
            bubble.stage_entered[self._config.stages[stall_point].name] = self._cycle
            next_stages[stall_point] = bubble

            # Stages BEFORE the stall point are frozen.
            for i in range(stall_point):
                next_stages[i] = self._stages[i]

            # PC does NOT advance during a stall.

        else:
            # NONE or FORWARD: Normal advancement.

            # Handle forwarding if needed.
            if hazard.action in (
                HazardAction.FORWARD_FROM_EX,
                HazardAction.FORWARD_FROM_MEM,
            ):
                for i, s in enumerate(self._config.stages):
                    if (
                        s.category == StageCategory.DECODE
                        and self._stages[i] is not None
                        and not self._stages[i].is_bubble
                    ):
                        self._stages[i].alu_result = hazard.forward_value
                        self._stages[i].forwarded_from = hazard.forward_source
                        break

            # Shift tokens forward (from back to front).
            for i in range(num_stages - 1, 0, -1):
                next_stages[i] = self._stages[i - 1]

            # Fetch new instruction into IF stage.
            tok = self._fetch_new_instruction()
            next_stages[0] = tok

        # --- Phase 3: Commit the new state ---
        self._stages = next_stages

        # --- Phase 4: Execute stage callbacks ---
        for i in range(num_stages - 1, -1, -1):
            tok = self._stages[i]
            if tok is None or tok.is_bubble:
                continue

            stage = self._config.stages[i]

            # Record when this token entered this stage.
            if stage.name not in tok.stage_entered:
                tok.stage_entered[stage.name] = self._cycle

            if stage.category == StageCategory.FETCH:
                # Already handled by _fetch_new_instruction().
                pass
            elif stage.category == StageCategory.DECODE:
                if not tok.opcode:
                    self._stages[i] = self._decode_fn(tok.raw_instruction, tok)
            elif stage.category == StageCategory.EXECUTE:
                if tok.stage_entered[stage.name] == self._cycle:
                    self._stages[i] = self._execute_fn(tok)
            elif stage.category == StageCategory.MEMORY:
                if tok.stage_entered[stage.name] == self._cycle:
                    self._stages[i] = self._memory_fn(tok)
            elif stage.category == StageCategory.WRITEBACK:
                # Writeback is handled in Phase 5 (retirement).
                pass

        # --- Phase 5: Retire the instruction in the last stage ---
        last_tok = self._stages[num_stages - 1]
        if last_tok is not None and not last_tok.is_bubble:
            self._writeback_fn(last_tok)
            self._stats.instructions_completed += 1
            if last_tok.is_halt:
                self._halted = True

        # Count bubbles across all stages.
        for tok in self._stages:
            if tok is not None and tok.is_bubble:
                self._stats.bubble_cycles += 1

        # --- Phase 6: Take snapshot ---
        snap = PipelineSnapshot(
            cycle=self._cycle,
            stages={},
            stalled=stalled,
            flushing=flushing,
            pc=self._pc,
        )
        for i, stage in enumerate(self._config.stages):
            if self._stages[i] is not None:
                snap.stages[stage.name] = self._stages[i].clone()
        self._history.append(snap)

        return snap

    def _fetch_new_instruction(self) -> PipelineToken:
        """Create a new token by calling the fetch callback.

        This is called at the start of each cycle to fetch the instruction
        at the current PC. The PC is then advanced (either by the branch
        predictor's prediction or by the default PC+4).
        """
        tok = new_token()
        tok.pc = self._pc
        tok.raw_instruction = self._fetch_fn(self._pc)
        tok.stage_entered[self._config.stages[0].name] = self._cycle

        # Advance PC: use branch predictor if available, otherwise PC+4.
        if self._predict_fn is not None:
            self._pc = self._predict_fn(self._pc)
        else:
            self._pc += 4

        return tok

    def run(self, max_cycles: int) -> PipelineStats:
        """Execute the pipeline until halt or max cycle count.

        This is the main simulation loop. It calls step() repeatedly until
        the pipeline halts or the cycle budget is exhausted.

        Returns the final execution statistics.
        """
        while self._cycle < max_cycles and not self._halted:
            self.step()
        return self.stats()

    def snapshot(self) -> PipelineSnapshot:
        """Return the current pipeline state without advancing the clock."""
        return self._take_snapshot()

    def trace(self) -> list[PipelineSnapshot]:
        """Return the complete history of pipeline snapshots."""
        return list(self._history)

    def stage_contents(self, stage_name: str) -> PipelineToken | None:
        """Return the token currently occupying the given stage.

        Returns None if the stage is empty or the stage name is invalid.
        """
        for i, s in enumerate(self._config.stages):
            if s.name == stage_name:
                return self._stages[i]
        return None

    def _take_snapshot(self) -> PipelineSnapshot:
        """Create a snapshot of the current pipeline state."""
        snap = PipelineSnapshot(
            cycle=self._cycle,
            stages={},
            pc=self._pc,
        )
        for i, stage in enumerate(self._config.stages):
            if self._stages[i] is not None:
                snap.stages[stage.name] = self._stages[i].clone()
        return snap
