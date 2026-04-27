"""Tests for the GE-225 Python simulator."""

from __future__ import annotations

import dataclasses

import pytest

from ge225_simulator import (
    GE225Indicators,
    GE225Simulator,
    GE225State,
    assemble_fixed,
    assemble_shift,
    decode_instruction,
    encode_instruction,
    pack_words,
    unpack_words,
)
from simulator_protocol import ExecutionResult, StepTrace


def ins(opcode: int, address: int = 0, modifier: int = 0) -> int:
    return encode_instruction(opcode, modifier, address)


class TestEncodingHelpers:
    def test_encode_decode_round_trip(self) -> None:
        word = encode_instruction(0o01, 0x02, 0x1234 & 0x1FFF)
        opcode, modifier, address = decode_instruction(word)
        assert opcode == 0o01
        assert modifier == 0x02
        assert address == (0x1234 & 0x1FFF)

    def test_pack_unpack_round_trip(self) -> None:
        words = [ins(0o00, 10), ins(0o01, 11), assemble_fixed("NOP")]
        assert unpack_words(pack_words(words)) == words

    def test_unpack_rejects_non_multiple_of_three_bytes(self) -> None:
        with pytest.raises(ValueError):
            unpack_words(b"\x00\x01")

    def test_assemble_fixed_and_shift(self) -> None:
        assert assemble_fixed("XAQ") == int("2504005", 8)
        assert assemble_shift("SRA", 3) == (int("2510000", 8) | 3)


class TestState:
    def test_get_state_returns_frozen_state(self) -> None:
        sim = GE225Simulator()
        state = sim.get_state()
        assert isinstance(state, GE225State)
        assert isinstance(state.indicators, GE225Indicators)
        with pytest.raises(dataclasses.FrozenInstanceError):
            state.a = 99  # type: ignore[misc]

    def test_reset_clears_registers_but_not_loaded_memory(self) -> None:
        sim = GE225Simulator()
        sim.write_word(10, 0xABCDE)
        sim.load_words([assemble_fixed("NOP")])
        sim.step()
        sim.reset()
        state = sim.get_state()
        assert state.a == 0
        assert state.q == 0
        assert state.n == 0
        assert state.halted is False
        assert state.memory[10] == 0xABCDE & ((1 << 20) - 1)


class TestExecution:
    def test_lda_add_sta_program(self) -> None:
        sim = GE225Simulator()
        program = [
            ins(0o00, 10),
            ins(0o01, 11),
            ins(0o03, 12),
            assemble_fixed("NOP"),
            0,
            0,
            0,
            0,
            0,
            0,
            1,
            2,
            0,
        ]
        sim.load_words(program)
        traces = sim.run(max_steps=4)
        state = sim.get_state()
        assert len(traces) == 4
        assert state.a == 3
        assert state.memory[12] == 3

    def test_spb_stores_instruction_address_in_selected_x_word(self) -> None:
        sim = GE225Simulator()
        sim.load_words(
            [
                ins(0o07, 4, modifier=2),  # SPB 4, X2
                assemble_fixed("NOP"),
                assemble_fixed("NOP"),
                assemble_fixed("NOP"),
                ins(0o00, 10),             # target
                assemble_fixed("NOP"),
                0,
                0,
                0,
                0,
                0x12345,
            ]
        )
        sim.run(max_steps=3)
        state = sim.get_state()
        assert state.x_words[2] == 0
        assert state.a == 0x12345

    def test_bxl_and_bxh_skip_style_behavior(self) -> None:
        sim = GE225Simulator()
        sim.load_words(
            [
                ins(0o06, 10, modifier=1),  # LDX 10,X1 => X1 = 4
                ins(0o04, 5, modifier=1),   # BXL 5,X1 => true, do not skip BRU
                ins(0o26, 6),               # BRU 6
                ins(0o00, 11),              # skipped by BRU
                assemble_fixed("NOP"),
                assemble_fixed("NOP"),
                ins(0o05, 4, modifier=1),   # BXH 4,X1 => true, do not skip BRU
                ins(0o26, 9),               # BRU 9
                ins(0o00, 12),              # skipped by BRU
                assemble_fixed("NOP"),
                4,
                111,
                222,
            ]
        )
        sim.run(max_steps=7)
        assert sim.get_state().pc == 11

    def test_cmp_sets_program_counter_skip_pattern(self) -> None:
        sim = GE225Simulator()
        sim.load_words(
            [ins(0o00, 10), ins(0o21, 11), assemble_fixed("NOP"), 0, 0, 0, 0, 0, 0, 0, 7, 7]
        )
        sim.run(max_steps=2)
        assert sim.get_state().pc == 3

    def test_dld_dst_odd_address_behavior(self) -> None:
        sim = GE225Simulator()
        sim.write_word(11, 0x13579)
        sim.load_words([ins(0o10, 11), ins(0o13, 13), assemble_fixed("NOP")])
        sim.run(max_steps=3)
        state = sim.get_state()
        assert state.a == 0x13579
        assert state.q == 0x13579
        assert state.memory[13] == 0x13579

    def test_sto_only_updates_low_13_bits(self) -> None:
        sim = GE225Simulator()
        sim.write_word(20, 0o300000)
        sim.load_words([ins(0o00, 21), ins(0o27, 20), assemble_fixed("NOP")])
        sim.write_word(21, 0x1ABC)
        sim.run(max_steps=3)
        assert sim.read_word(20) == (0o300000 | (0x1ABC & 0x1FFF))

    def test_moy_moves_block_and_clears_a(self) -> None:
        sim = GE225Simulator()
        sim.write_word(20, 0x11111)
        sim.write_word(21, 0x22222)
        sim.load_words(
            [
                ins(0o00, 30),              # A = destination 40
                assemble_fixed("LQA"),      # Q = A
                ins(0o00, 31),              # A = count in 2's complement (-2)
                assemble_fixed("XAQ"),      # A = 40, Q = -2
                ins(0o24, 20),              # MOY 20
                assemble_fixed("NOP"),
            ]
        )
        sim.write_word(30, 40)
        sim.write_word(31, ((1 << 20) - 2))
        sim.run(max_steps=6)
        state = sim.get_state()
        assert state.a == 0
        assert state.memory[40] == 0x11111
        assert state.memory[41] == 0x22222

    def test_console_typewriter_path(self) -> None:
        sim = GE225Simulator()
        sim.set_control_switches(0o1633)
        sim.load_words(
            [
                assemble_fixed("RCS"),
                assemble_fixed("TON"),
                assemble_shift("SAN", 6),  # shift A's low character into N
                assemble_fixed("TYP"),
                assemble_fixed("NOP"),
            ]
        )
        sim.run(max_steps=5)
        state = sim.get_state()
        assert state.typewriter_power is True
        assert sim.get_typewriter_output() == "-"
        assert state.n == 0o33

    def test_branch_indicators_clear_after_test(self) -> None:
        sim = GE225Simulator()
        sim.load_words([ins(0o00, 10), ins(0o01, 11), assemble_fixed("BOV"), assemble_fixed("NOP"), 0, 0, 0, 0, 0, 0, 0x7FFFF, 1])
        sim.run(max_steps=3)
        assert sim.get_state().overflow is False

    def test_typewriter_ready_branches(self) -> None:
        sim = GE225Simulator()
        sim.load_words([assemble_fixed("HPT"), assemble_fixed("BNN"), ins(0o26, 4), assemble_fixed("NOP"), assemble_fixed("NOP")])
        sim.run(max_steps=4)
        assert sim.get_state().pc == 5

    def test_inx_and_indexed_addressing_use_x_words(self) -> None:
        sim = GE225Simulator()
        sim.load_words(
            [
                ins(0o14, 2, modifier=1),
                ins(0o00, 20, modifier=1),
                assemble_fixed("NOP"),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        )
        sim.write_word(22, 0x12345)
        sim.run(max_steps=3)
        state = sim.get_state()
        assert state.a == 0x12345
        assert state.x_words[1] == 2

    def test_rcd_loads_queued_record(self) -> None:
        sim = GE225Simulator()
        sim.queue_card_reader_record([0x11111, 0x22222])
        sim.load_words([ins(0o25, 10), assemble_fixed("NOP")])
        sim.run(max_steps=2)
        assert sim.read_word(10) == 0x11111
        assert sim.read_word(11) == 0x22222

    def test_disassemble_fixed_and_memory_op(self) -> None:
        sim = GE225Simulator()
        assert sim.disassemble_word(assemble_fixed("NOP")) == "NOP"
        assert sim.disassemble_word(ins(0o00, 10)) == "LDA 0x00A,X0"


class TestBranchSemantics:
    """Verify skip-if-TRUE semantics for all conditional branch instructions.

    The GE-225 branch-test instructions skip the *next* word when their named
    condition is TRUE.  Pattern used::

        addr 0..n-1 : setup instructions
        addr n      : B??          (branch-test under test)
        addr n+1    : BRU 99       (jumped to when condition is FALSE)
        addr n+2    : NOP          (skipped over when condition is TRUE)
        addr n+3..  : NOP padding  (BRU-99 target is here)

    After ``n + 3`` steps:
    - TRUE path:  branch skips BRU → NOP at n+2 → NOP padding → pc = n+4
    - FALSE path: BRU 99 executes → NOP at 99 → pc = 100
    """

    def _probe(self, setup_words: list[int], branch_word: int, steps: int | None = None) -> int:
        """Return PC after running the probe program."""
        sim = GE225Simulator()
        n = len(setup_words)
        program = setup_words + [
            branch_word,
            ins(0o26, 99),         # BRU 99: taken when condition is FALSE
            assemble_fixed("NOP"), # skipped when condition is TRUE
        ] + [assemble_fixed("NOP")] * 100
        sim.load_words(program)
        sim.run(max_steps=steps if steps is not None else n + 3)
        return sim.get_state().pc

    def test_bze_skips_when_a_is_zero(self) -> None:
        # A=0 → BZE condition TRUE → skip BRU → pc = 1+4 = 5
        assert self._probe([assemble_fixed("LDZ")], assemble_fixed("BZE")) == 5

    def test_bze_does_not_skip_when_a_is_nonzero(self) -> None:
        # A=1 → BZE condition FALSE → BRU 99 → pc = 100
        assert self._probe([assemble_fixed("LDO")], assemble_fixed("BZE")) == 100

    def test_bnz_skips_when_a_is_nonzero(self) -> None:
        # A=1 → BNZ condition TRUE → skip BRU → pc = 5
        assert self._probe([assemble_fixed("LDO")], assemble_fixed("BNZ")) == 5

    def test_bnz_does_not_skip_when_a_is_zero(self) -> None:
        # A=0 → BNZ condition FALSE → BRU 99 → pc = 100
        assert self._probe([assemble_fixed("LDZ")], assemble_fixed("BNZ")) == 100

    def test_bpl_skips_when_a_is_nonnegative(self) -> None:
        # A=0 → BPL condition TRUE → skip BRU → pc = 5
        assert self._probe([assemble_fixed("LDZ")], assemble_fixed("BPL")) == 5

    def test_bpl_does_not_skip_when_a_is_negative(self) -> None:
        # A=-1 (LDO + NEG) → BPL condition FALSE → BRU 99 → pc = 100
        assert self._probe(
            [assemble_fixed("LDO"), assemble_fixed("NEG")], assemble_fixed("BPL"), steps=5
        ) == 100

    def test_bmi_skips_when_a_is_negative(self) -> None:
        # A=-1 (LDO + NEG) → BMI condition TRUE → skip BRU → pc = 2+4 = 6
        assert self._probe(
            [assemble_fixed("LDO"), assemble_fixed("NEG")], assemble_fixed("BMI"), steps=5
        ) == 6

    def test_bod_skips_when_a_is_odd(self) -> None:
        # A=1 → BOD condition TRUE → skip BRU → pc = 5
        assert self._probe([assemble_fixed("LDO")], assemble_fixed("BOD")) == 5

    def test_bod_does_not_skip_when_a_is_even(self) -> None:
        # A=0 → BOD condition FALSE → BRU 99 → pc = 100
        assert self._probe([assemble_fixed("LDZ")], assemble_fixed("BOD")) == 100

    def test_bev_skips_when_a_is_even(self) -> None:
        # A=0 → BEV condition TRUE → skip BRU → pc = 5
        assert self._probe([assemble_fixed("LDZ")], assemble_fixed("BEV")) == 5

    def test_bev_does_not_skip_when_a_is_odd(self) -> None:
        # A=1 → BEV condition FALSE → BRU 99 → pc = 100
        assert self._probe([assemble_fixed("LDO")], assemble_fixed("BEV")) == 100


class TestProtocolExecution:
    def test_execute_returns_execution_result(self) -> None:
        sim = GE225Simulator()
        result = sim.execute(pack_words([assemble_fixed("NOP")]), max_steps=1)
        assert isinstance(result, ExecutionResult)
        assert result.ok is False
        assert result.halted is False
        assert result.steps == 1
        assert isinstance(result.traces[0], StepTrace)

    def test_execute_reports_max_steps_exceeded(self) -> None:
        sim = GE225Simulator()
        result = sim.execute(pack_words([ins(0o26, 0)]), max_steps=3)
        assert result.ok is False
        assert result.halted is False
        assert result.error == "max_steps (3) exceeded"
        assert result.steps == 3

    def test_execute_runs_small_program(self) -> None:
        sim = GE225Simulator()
        result = sim.execute(
            pack_words(
                [
                    ins(0o00, 10),
                    ins(0o01, 11),
                    ins(0o03, 12),
                    assemble_fixed("NOP"),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    5,
                    7,
                    0,
                ]
            ),
            max_steps=4,
        )
        assert result.ok is False
        assert result.final_state.a == 12
        assert result.final_state.memory[12] == 12
