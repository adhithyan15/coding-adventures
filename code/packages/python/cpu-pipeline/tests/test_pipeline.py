"""Comprehensive tests for the cpu_pipeline package.

Ported from the Go reference implementation. These tests verify:
  - Token creation and cloning
  - Pipeline configuration and validation
  - Instruction flow through the 5-stage pipeline
  - Stall behavior (freezing earlier stages, bubble insertion)
  - Flush behavior (replacing speculative stages with bubbles)
  - Forwarding integration
  - Statistics tracking (IPC, CPI, stall/flush counts)
  - Trace and snapshot accuracy
  - Branch predictor integration
  - Deep (13-stage) pipeline behavior
  - Custom stage configurations

Test encoding:
  opcode: bits 31-24
  rd:     bits 23-16
  rs1:    bits 15-8
  rs2:    bits 7-0
"""

from __future__ import annotations

import math

import pytest

from cpu_pipeline import (
    HazardAction,
    HazardResponse,
    Pipeline,
    PipelineConfig,
    PipelineSnapshot,
    PipelineStage,
    PipelineStats,
    PipelineToken,
    StageCategory,
    classic_5_stage,
    deep_13_stage,
    new_bubble,
    new_token,
)

# =========================================================================
# Test helpers -- simple instruction memory and callbacks
# =========================================================================

# Test opcode constants.
OP_NOP = 0x00
OP_ADD = 0x01
OP_LDR = 0x02
OP_STR = 0x03
OP_BEQ = 0x04
OP_HALT = 0xFF


def make_instruction(opcode: int, rd: int, rs1: int, rs2: int) -> int:
    """Encode a test instruction.

    opcode: 8 bits (bits 31-24)
    rd:     8 bits (bits 23-16)
    rs1:    8 bits (bits 15-8)
    rs2:    8 bits (bits 7-0)
    """
    return (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2


def simple_fetch(instrs: list[int]):  # noqa: ANN201
    """Return a fetch function that reads from the given instruction memory.

    If the PC is out of bounds, returns a NOP.
    """

    def fetch(pc: int) -> int:
        idx = pc // 4
        if idx < 0 or idx >= len(instrs):
            return make_instruction(OP_NOP, 0, 0, 0)
        return instrs[idx]

    return fetch


def simple_decode():  # noqa: ANN201
    """Return a decode function that parses our test encoding."""

    def decode(raw: int, tok: PipelineToken) -> PipelineToken:
        opcode = (raw >> 24) & 0xFF
        rd = (raw >> 16) & 0xFF
        rs1 = (raw >> 8) & 0xFF
        rs2 = raw & 0xFF

        if opcode == OP_ADD:
            tok.opcode = "ADD"
            tok.rd = rd
            tok.rs1 = rs1
            tok.rs2 = rs2
            tok.reg_write = True
        elif opcode == OP_LDR:
            tok.opcode = "LDR"
            tok.rd = rd
            tok.rs1 = rs1
            tok.mem_read = True
            tok.reg_write = True
        elif opcode == OP_STR:
            tok.opcode = "STR"
            tok.rs1 = rs1
            tok.rs2 = rs2
            tok.mem_write = True
        elif opcode == OP_BEQ:
            tok.opcode = "BEQ"
            tok.rs1 = rs1
            tok.rs2 = rs2
            tok.is_branch = True
        elif opcode == OP_HALT:
            tok.opcode = "HALT"
            tok.is_halt = True
        else:
            tok.opcode = "NOP"

        return tok

    return decode


def simple_execute():  # noqa: ANN201
    """Return an execute callback that sets alu_result."""

    def execute(tok: PipelineToken) -> PipelineToken:
        if tok.opcode == "ADD":
            tok.alu_result = tok.rs1 + tok.rs2  # Simplified: use reg numbers as values
        elif tok.opcode in ("LDR", "STR"):
            tok.alu_result = tok.rs1 + tok.immediate
        elif tok.opcode == "BEQ":
            tok.branch_target = tok.pc + tok.immediate
        return tok

    return execute


def simple_memory():  # noqa: ANN201
    """Return a memory callback that handles loads."""

    def memory(tok: PipelineToken) -> PipelineToken:
        if tok.mem_read:
            tok.mem_data = 42  # Fixed value for testing
            tok.write_data = tok.mem_data
        else:
            tok.write_data = tok.alu_result
        return tok

    return memory


def simple_writeback(completed: list[int] | None):  # noqa: ANN201
    """Return a writeback callback that records completed instructions."""

    def writeback(tok: PipelineToken) -> None:
        if completed is not None:
            completed.append(tok.pc)

    return writeback


def new_test_pipeline(
    instrs: list[int], completed: list[int] | None = None
) -> Pipeline:
    """Create a 5-stage pipeline with simple test callbacks."""
    config = classic_5_stage()
    return Pipeline(
        config,
        simple_fetch(instrs),
        simple_decode(),
        simple_execute(),
        simple_memory(),
        simple_writeback(completed),
    )


# =========================================================================
# Token tests
# =========================================================================


class TestToken:
    """Tests for PipelineToken creation, display, and cloning."""

    def test_new_token(self) -> None:
        """New token should have default register values of -1."""
        tok = new_token()
        assert tok.rs1 == -1
        assert tok.rs2 == -1
        assert tok.rd == -1
        assert not tok.is_bubble
        assert tok.stage_entered is not None

    def test_new_bubble(self) -> None:
        """Bubble should have is_bubble=True and display as '---'."""
        b = new_bubble()
        assert b.is_bubble
        assert str(b) == "---"

    def test_token_string_with_opcode(self) -> None:
        """Token with opcode should display as 'OPCODE@PC'."""
        tok = new_token()
        tok.opcode = "ADD"
        tok.pc = 100
        assert str(tok) == "ADD@100"

    def test_token_string_without_opcode(self) -> None:
        """Token without opcode should display as 'instr@PC'."""
        tok = new_token()
        tok.pc = 200
        assert str(tok) == "instr@200"

    def test_token_clone(self) -> None:
        """Clone should have same field values but independent stage_entered."""
        tok = new_token()
        tok.pc = 100
        tok.opcode = "ADD"
        tok.stage_entered["IF"] = 1
        tok.stage_entered["ID"] = 2

        clone = tok.clone()
        assert clone.pc == 100
        assert clone.opcode == "ADD"

        # Mutating the clone should not affect the original.
        clone.stage_entered["EX"] = 3
        assert "EX" not in tok.stage_entered

    def test_token_clone_preserves_values(self) -> None:
        """Clone should preserve all field values."""
        tok = new_token()
        tok.pc = 50
        tok.raw_instruction = 0x12345678
        tok.reg_write = True
        tok.alu_result = 42

        clone = tok.clone()
        assert clone.pc == 50
        assert clone.raw_instruction == 0x12345678
        assert clone.reg_write is True
        assert clone.alu_result == 42


# =========================================================================
# PipelineConfig tests
# =========================================================================


class TestPipelineConfig:
    """Tests for pipeline configuration and validation."""

    def test_classic_5_stage(self) -> None:
        """Classic 5-stage pipeline should have 5 stages and be valid."""
        config = classic_5_stage()
        assert config.num_stages() == 5
        config.validate()  # Should not raise
        assert config.stages[0].name == "IF"
        assert config.stages[4].name == "WB"

    def test_deep_13_stage(self) -> None:
        """Deep 13-stage pipeline should have 13 stages and be valid."""
        config = deep_13_stage()
        assert config.num_stages() == 13
        config.validate()  # Should not raise

    def test_too_few_stages(self) -> None:
        """Pipeline with fewer than 2 stages should fail validation."""
        cfg = PipelineConfig(
            stages=[PipelineStage("IF", "Fetch", StageCategory.FETCH)],
            execution_width=1,
        )
        with pytest.raises(ValueError, match="at least 2 stages"):
            cfg.validate()

    def test_zero_execution_width(self) -> None:
        """Pipeline with zero execution width should fail validation."""
        cfg = PipelineConfig(
            stages=[
                PipelineStage("IF", "Fetch", StageCategory.FETCH),
                PipelineStage("WB", "Writeback", StageCategory.WRITEBACK),
            ],
            execution_width=0,
        )
        with pytest.raises(ValueError, match="execution width"):
            cfg.validate()

    def test_duplicate_stage_names(self) -> None:
        """Pipeline with duplicate stage names should fail validation."""
        cfg = PipelineConfig(
            stages=[
                PipelineStage("IF", "Fetch", StageCategory.FETCH),
                PipelineStage("IF", "Writeback", StageCategory.WRITEBACK),
            ],
            execution_width=1,
        )
        with pytest.raises(ValueError, match="duplicate stage name"):
            cfg.validate()

    def test_no_fetch_stage(self) -> None:
        """Pipeline without a fetch stage should fail validation."""
        cfg = PipelineConfig(
            stages=[
                PipelineStage("EX", "Execute", StageCategory.EXECUTE),
                PipelineStage("WB", "Writeback", StageCategory.WRITEBACK),
            ],
            execution_width=1,
        )
        with pytest.raises(ValueError, match="fetch stage"):
            cfg.validate()

    def test_no_writeback_stage(self) -> None:
        """Pipeline without a writeback stage should fail validation."""
        cfg = PipelineConfig(
            stages=[
                PipelineStage("IF", "Fetch", StageCategory.FETCH),
                PipelineStage("EX", "Execute", StageCategory.EXECUTE),
            ],
            execution_width=1,
        )
        with pytest.raises(ValueError, match="writeback stage"):
            cfg.validate()

    def test_valid_2_stage(self) -> None:
        """A minimal 2-stage pipeline (IF, WB) should be valid."""
        cfg = PipelineConfig(
            stages=[
                PipelineStage("IF", "Fetch", StageCategory.FETCH),
                PipelineStage("WB", "Writeback", StageCategory.WRITEBACK),
            ],
            execution_width=1,
        )
        cfg.validate()  # Should not raise


# =========================================================================
# Basic Pipeline tests
# =========================================================================


class TestPipelineBasic:
    """Tests for basic pipeline creation and operation."""

    def test_new_pipeline(self) -> None:
        """New pipeline should start at cycle 0, PC=0, not halted."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)]
        p = new_test_pipeline(instrs)
        assert not p.halted
        assert p.cycle == 0
        assert p.pc == 0

    def test_new_pipeline_invalid_config(self) -> None:
        """Pipeline with invalid config should raise ValueError."""
        cfg = PipelineConfig(
            stages=[PipelineStage("IF", "Fetch", StageCategory.FETCH)],
            execution_width=1,
        )
        with pytest.raises(ValueError):
            Pipeline(
                cfg,
                lambda pc: 0,
                lambda raw, tok: tok,
                lambda tok: tok,
                lambda tok: tok,
                lambda tok: None,
            )

    def test_single_instruction_flows_through_5_stages(self) -> None:
        """A single instruction should complete after 5 cycles in a 5-stage pipeline.

        Timeline:
            Cycle 1: ADD enters IF
            Cycle 2: ADD enters ID
            Cycle 3: ADD enters EX
            Cycle 4: ADD enters MEM
            Cycle 5: ADD enters WB and retires
        """
        instrs = [
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        completed: list[int] = []
        p = new_test_pipeline(instrs, completed)

        for _ in range(5):
            p.step()

        assert len(completed) > 0, "expected at least one completion after 5 cycles"
        assert completed[0] == 0, (
            f"expected first completed at PC=0, got {completed[0]}"
        )

    def test_pipeline_fill_timing(self) -> None:
        """First instruction completes at cycle 5; subsequent at one per cycle.

        Timeline:
            Cycle:  1    2    3    4    5    6    7
            IF:    I1   I2   I3   I4   I5   I6   I7
            ID:    --   I1   I2   I3   I4   I5   I6
            EX:    --   --   I1   I2   I3   I4   I5
            MEM:   --   --   --   I1   I2   I3   I4
            WB:    --   --   --   --   I1   I2   I3
        """
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 20
        completed: list[int] = []
        p = new_test_pipeline(instrs, completed)

        # After 4 cycles, nothing should have completed yet.
        for _ in range(4):
            p.step()
        assert len(completed) == 0

        # After cycle 5, exactly 1 instruction should have completed.
        p.step()
        assert len(completed) == 1

        # After cycle 6, 2 completions. After cycle 7, 3 completions.
        p.step()
        assert len(completed) == 2
        p.step()
        assert len(completed) == 3

    def test_steady_state_ipc(self) -> None:
        """After the pipeline fills, IPC approaches 1.0 for independent instructions."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 100
        p = new_test_pipeline(instrs)

        for _ in range(50):
            p.step()

        stats = p.stats()
        # After 50 cycles: completed = 50 - 5 + 1 = 46
        expected_completed = 50 - 5 + 1
        assert stats.instructions_completed == expected_completed

        ipc = stats.ipc()
        assert 0.85 < ipc <= 1.01, f"expected IPC near 1.0, got {ipc:.3f}"

    def test_empty_pipeline(self) -> None:
        """Stepping an empty pipeline (no program) should not crash."""
        p = new_test_pipeline([])
        snap = p.step()
        assert snap.cycle == 1


# =========================================================================
# Halt tests
# =========================================================================


class TestHalt:
    """Tests for halt instruction propagation."""

    def test_halt_propagation(self) -> None:
        """HALT at PC=8 should halt pipeline at cycle 7.

        Program: ADD, ADD, HALT
        HALT enters IF at cycle 3, reaches WB at cycle 7.
        """
        instrs = [
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_HALT, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        completed: list[int] = []
        p = new_test_pipeline(instrs, completed)

        stats = p.run(100)
        assert p.halted
        assert p.cycle == 7
        # Two ADD + one HALT should have completed.
        assert stats.instructions_completed == 3

    def test_halted_pipeline_does_not_advance(self) -> None:
        """Stepping a halted pipeline should not change cycle or completions."""
        instrs = [
            make_instruction(OP_HALT, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        p = new_test_pipeline(instrs)
        p.run(100)

        cycle_at_halt = p.cycle
        completed_at_halt = p.stats().instructions_completed

        # Step again -- nothing should change.
        p.step()
        p.step()

        assert p.cycle == cycle_at_halt
        assert p.stats().instructions_completed == completed_at_halt


# =========================================================================
# Stall tests
# =========================================================================


class TestStall:
    """Tests for pipeline stall behavior."""

    def test_stall_freezes_earlier_stages(self) -> None:
        """During a stall, IF and ID are frozen and a bubble is inserted at EX."""
        instrs = [
            make_instruction(OP_LDR, 1, 2, 0),  # LDR R1, [R2]
            make_instruction(OP_ADD, 3, 1, 4),   # ADD R3, R1, R4 (depends on LDR)
            make_instruction(OP_ADD, 5, 6, 7),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        completed: list[int] = []
        p = new_test_pipeline(instrs, completed)

        # Detect load-use hazard: LDR in EX, ADD in ID -> stall.
        stall_injected = False

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal stall_injected
            if not stall_injected and len(stages) >= 3:
                ex_tok = stages[2]  # EX stage
                id_tok = stages[1]  # ID stage
                if (
                    ex_tok is not None
                    and not ex_tok.is_bubble
                    and ex_tok.opcode == "LDR"
                    and id_tok is not None
                    and not id_tok.is_bubble
                    and id_tok.opcode == "ADD"
                ):
                    stall_injected = True
                    return HazardResponse(
                        action=HazardAction.STALL,
                        stall_stages=2,
                    )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        # After step 1: stages[0]=LDR
        # After step 2: stages[0]=ADD, stages[1]=LDR
        # After step 3: stages[0]=I3, stages[1]=ADD, stages[2]=LDR
        # At step 4: hazard checks -> EX has LDR, ID has ADD -> STALL!
        p.step()  # cycle 1
        p.step()  # cycle 2
        p.step()  # cycle 3

        snap = p.step()  # cycle 4 -- stall should occur here
        assert snap.stalled, "expected pipeline to be stalled at cycle 4"

        # After stall: EX should have a bubble, ID should still have ADD (frozen).
        ex_tok = p.stage_contents("EX")
        assert ex_tok is not None and ex_tok.is_bubble

        id_tok = p.stage_contents("ID")
        assert id_tok is not None and id_tok.opcode == "ADD"

        stats = p.stats()
        assert stats.stall_cycles == 1

    def test_stall_bubble_insertion(self) -> None:
        """A bubble should be inserted at the stall point."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 10
        p = new_test_pipeline(instrs)

        # Force a stall at cycle 3.
        stall_count = 0

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal stall_count
            stall_count += 1
            if stall_count == 3:
                return HazardResponse(
                    action=HazardAction.STALL,
                    stall_stages=2,
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(3):
            p.step()

        ex_tok = p.stage_contents("EX")
        assert ex_tok is not None and ex_tok.is_bubble

    def test_stall_reduces_ipc(self) -> None:
        """Stalls should reduce IPC below 1.0."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 50
        p = new_test_pipeline(instrs)

        cycle_count = 0

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal cycle_count
            cycle_count += 1
            if cycle_count % 5 == 0:
                return HazardResponse(
                    action=HazardAction.STALL,
                    stall_stages=2,
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(30):
            p.step()

        stats = p.stats()
        assert stats.ipc() < 1.0
        assert stats.stall_cycles > 0

    def test_stall_default_stall_point(self) -> None:
        """When stall_stages is 0, the pipeline should use the default (EX index)."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 20
        p = new_test_pipeline(instrs)

        stalled = False

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal stalled
            if not stalled and len(stages) >= 3 and stages[2] is not None:
                stalled = True
                return HazardResponse(
                    action=HazardAction.STALL,
                    stall_stages=0,  # Use default
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(5):
            p.step()

        assert p.stats().stall_cycles == 1


# =========================================================================
# Flush tests
# =========================================================================


class TestFlush:
    """Tests for pipeline flush behavior."""

    def test_flush_replaces_with_bubbles(self) -> None:
        """Flush should replace speculative stages with bubbles and redirect PC."""
        instrs = [
            make_instruction(OP_BEQ, 0, 1, 2),  # branch at PC=0
            make_instruction(OP_ADD, 1, 2, 3),   # speculative (wrong path)
            make_instruction(OP_ADD, 4, 5, 6),   # speculative (wrong path)
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            # target at PC=20
            make_instruction(OP_ADD, 7, 8, 9),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        p = new_test_pipeline(instrs)

        flushed = False

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal flushed
            if not flushed and len(stages) >= 3:
                ex_tok = stages[2]
                if ex_tok is not None and not ex_tok.is_bubble and ex_tok.is_branch:
                    flushed = True
                    return HazardResponse(
                        action=HazardAction.FLUSH,
                        flush_count=2,
                        redirect_pc=20,
                    )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        p.step()  # cycle 1: IF=BEQ
        p.step()  # cycle 2: IF=ADD, ID=BEQ
        p.step()  # cycle 3: IF=ADD2, ID=ADD, EX=BEQ

        snap = p.step()  # cycle 4 -- flush should occur
        assert snap.flushing

        # After flush, PC should be redirected to 20 + 4 (advanced by fetch).
        assert p.pc == 24

        stats = p.stats()
        assert stats.flush_cycles == 1

    def test_flush_default_flush_count(self) -> None:
        """When flush_count is 0, the pipeline should use the default."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 20
        p = new_test_pipeline(instrs)

        flushed = False

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal flushed
            if (
                not flushed
                and len(stages) >= 3
                and stages[2] is not None
                and not stages[2].is_bubble
            ):
                flushed = True
                return HazardResponse(
                    action=HazardAction.FLUSH,
                    flush_count=0,  # Use default
                    redirect_pc=100,
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(5):
            p.step()

        assert p.stats().flush_cycles == 1


# =========================================================================
# Forwarding integration tests
# =========================================================================


class TestForwarding:
    """Tests for forwarding integration."""

    def test_forwarding_applied(self) -> None:
        """Forwarding should set forwarded_from metadata on the token."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 10
        p = new_test_pipeline(instrs)

        forward_cycle = 0

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal forward_cycle
            forward_cycle += 1
            if forward_cycle == 4:
                return HazardResponse(
                    action=HazardAction.FORWARD_FROM_EX,
                    forward_value=99,
                    forward_source="EX",
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(4):
            p.step()

        # After 4 steps, the forwarded token has moved from ID to EX.
        ex_tok = p.stage_contents("EX")
        assert ex_tok is not None
        assert ex_tok.forwarded_from == "EX"


# =========================================================================
# Statistics tests
# =========================================================================


class TestStatistics:
    """Tests for IPC, CPI, and other statistics."""

    def test_ipc_calculation(self) -> None:
        stats = PipelineStats(total_cycles=100, instructions_completed=80)
        assert math.isclose(stats.ipc(), 0.8, abs_tol=0.001)

    def test_cpi_calculation(self) -> None:
        stats = PipelineStats(total_cycles=120, instructions_completed=100)
        assert math.isclose(stats.cpi(), 1.2, abs_tol=0.001)

    def test_ipc_zero_cycles(self) -> None:
        stats = PipelineStats()
        assert stats.ipc() == 0.0

    def test_cpi_zero_instructions(self) -> None:
        stats = PipelineStats(total_cycles=10)
        assert stats.cpi() == 0.0

    def test_stats_string(self) -> None:
        stats = PipelineStats(
            total_cycles=100,
            instructions_completed=80,
            stall_cycles=5,
            flush_cycles=3,
            bubble_cycles=10,
        )
        s = str(stats)
        assert s != ""
        assert "100" in s
        assert "80" in s


# =========================================================================
# Trace and Snapshot tests
# =========================================================================


class TestSnapshotAndTrace:
    """Tests for snapshots and trace recording."""

    def test_snapshot_accuracy(self) -> None:
        """Snapshots should correctly reflect pipeline contents."""
        instrs = [
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_NOP, 0, 0, 0),
        ]
        p = new_test_pipeline(instrs)

        # After 1 cycle, only IF has a token.
        snap1 = p.step()
        assert snap1.cycle == 1
        assert "IF" in snap1.stages
        assert snap1.stages["IF"].pc == 0

        # After 2 cycles, IF has second instruction, ID has first.
        snap2 = p.step()
        assert snap2.cycle == 2
        assert "ID" in snap2.stages
        assert snap2.stages["ID"].pc == 0

    def test_trace_completeness(self) -> None:
        """Trace should record every cycle's state."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 10
        p = new_test_pipeline(instrs)

        for _ in range(7):
            p.step()

        trace = p.trace()
        assert len(trace) == 7

        # Verify cycle numbering is sequential.
        for i, snap in enumerate(trace):
            assert snap.cycle == i + 1

    def test_snapshot_does_not_advance(self) -> None:
        """Taking a snapshot should not modify the pipeline state."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)]
        p = new_test_pipeline(instrs)
        p.step()

        snap1 = p.snapshot()
        snap2 = p.snapshot()
        assert snap1.cycle == snap2.cycle

    def test_snapshot_string(self) -> None:
        """PipelineSnapshot string representation should not be empty."""
        snap = PipelineSnapshot(cycle=7, pc=28, stalled=True)
        s = str(snap)
        assert s != ""


# =========================================================================
# Configuration preset tests
# =========================================================================


class TestConfigPresets:
    """Tests for deep pipelines and custom configurations."""

    def test_deep_pipeline_longer_fill_time(self) -> None:
        """A 13-stage pipeline takes 13 cycles to produce first completion."""
        config = deep_13_stage()
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 30

        p = Pipeline(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            simple_writeback(None),
        )

        # After 12 cycles, nothing completed.
        for _ in range(12):
            p.step()
        assert p.stats().instructions_completed == 0

        # After cycle 13, exactly 1 completion.
        p.step()
        assert p.stats().instructions_completed == 1

    def test_custom_stage_configuration(self) -> None:
        """Custom 3-stage pipeline (IF, EX, WB) should work correctly."""
        config = PipelineConfig(
            stages=[
                PipelineStage("IF", "Fetch", StageCategory.FETCH),
                PipelineStage("EX", "Execute", StageCategory.EXECUTE),
                PipelineStage("WB", "Writeback", StageCategory.WRITEBACK),
            ],
            execution_width=1,
        )
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 10
        completed: list[int] = []

        p = Pipeline(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            simple_writeback(completed),
        )

        # In a 3-stage pipeline, first completion at cycle 3.
        for _ in range(2):
            p.step()
        assert len(completed) == 0

        p.step()  # cycle 3
        assert len(completed) == 1


# =========================================================================
# Branch prediction integration tests
# =========================================================================


class TestBranchPrediction:
    """Tests for branch predictor integration."""

    def test_branch_predictor_integration(self) -> None:
        """Predict callback should determine the next PC during fetch."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 100
        p = new_test_pipeline(instrs)

        # Predictor that always predicts PC+8 (skip one instruction).
        p.set_predict_func(lambda pc: pc + 8)

        p.step()  # cycle 1: fetches PC=0, predicts next=8
        assert p.pc == 8

        p.step()  # cycle 2: fetches PC=8, predicts next=16
        assert p.pc == 16


# =========================================================================
# SetPC test
# =========================================================================


class TestSetPC:
    """Tests for setting the program counter."""

    def test_set_pc(self) -> None:
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 10
        p = new_test_pipeline(instrs)
        p.set_pc(100)
        assert p.pc == 100


# =========================================================================
# StageCategory and HazardAction string tests
# =========================================================================


class TestEnumStrings:
    """Tests for enum string representations."""

    def test_stage_category_strings(self) -> None:
        assert str(StageCategory.FETCH) == "fetch"
        assert str(StageCategory.DECODE) == "decode"
        assert str(StageCategory.EXECUTE) == "execute"
        assert str(StageCategory.MEMORY) == "memory"
        assert str(StageCategory.WRITEBACK) == "writeback"

    def test_hazard_action_strings(self) -> None:
        assert str(HazardAction.NONE) == "NONE"
        assert str(HazardAction.FORWARD_FROM_EX) == "FORWARD_FROM_EX"
        assert str(HazardAction.FORWARD_FROM_MEM) == "FORWARD_FROM_MEM"
        assert str(HazardAction.STALL) == "STALL"
        assert str(HazardAction.FLUSH) == "FLUSH"

    def test_pipeline_stage_string(self) -> None:
        stage = PipelineStage("IF", "Instruction Fetch", StageCategory.FETCH)
        assert str(stage) == "IF"


# =========================================================================
# Additional pipeline tests
# =========================================================================


class TestPipelineMisc:
    """Miscellaneous pipeline tests."""

    def test_pipeline_config_accessor(self) -> None:
        instrs = [make_instruction(OP_NOP, 0, 0, 0)]
        p = new_test_pipeline(instrs)
        cfg = p.config()
        assert cfg.num_stages() == 5

    def test_stage_contents_invalid_name(self) -> None:
        instrs = [make_instruction(OP_NOP, 0, 0, 0)]
        p = new_test_pipeline(instrs)
        p.step()
        assert p.stage_contents("NONEXISTENT") is None

    def test_multiple_stalls_and_flushes(self) -> None:
        """Multiple stalls and flushes should be tracked correctly."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 50
        p = new_test_pipeline(instrs)

        cycle_counter = 0

        def hazard_fn(
            stages: list[PipelineToken | None],
        ) -> HazardResponse:
            nonlocal cycle_counter
            cycle_counter += 1
            if cycle_counter in (5, 10):
                return HazardResponse(
                    action=HazardAction.STALL,
                    stall_stages=2,
                )
            if cycle_counter == 15:
                return HazardResponse(
                    action=HazardAction.FLUSH,
                    flush_count=2,
                    redirect_pc=0,
                )
            return HazardResponse(action=HazardAction.NONE)

        p.set_hazard_func(hazard_fn)

        for _ in range(20):
            p.step()

        stats = p.stats()
        assert stats.stall_cycles == 2
        assert stats.flush_cycles == 1

    def test_run_max_cycles(self) -> None:
        """Run should stop at the max cycle limit."""
        instrs = [make_instruction(OP_ADD, 1, 2, 3)] * 100
        p = new_test_pipeline(instrs)

        stats = p.run(10)
        assert stats.total_cycles == 10
        assert not p.halted
