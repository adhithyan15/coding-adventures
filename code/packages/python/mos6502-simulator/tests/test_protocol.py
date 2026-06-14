"""SIM00 protocol compliance tests for MOS6502Simulator."""

from __future__ import annotations

import pytest

from mos6502_simulator import MOS6502Simulator, MOS6502State


class TestReset:
    def test_registers_cleared(self) -> None:
        sim = MOS6502Simulator()
        sim.execute(bytes([0xA9, 0x42, 0x00]))  # LDA #0x42, BRK
        sim.reset()
        s = sim.get_state()
        assert s.a == 0
        assert s.x == 0
        assert s.y == 0
        assert s.pc == 0

    def test_stack_pointer_is_fd(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        assert sim.get_state().s == 0xFD

    def test_i_flag_set(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        assert sim.get_state().flag_i is True

    def test_not_halted_after_reset(self) -> None:
        sim = MOS6502Simulator()
        sim.execute(bytes([0x00]))
        sim.reset()
        assert sim.get_state().halted is False


class TestLoad:
    def test_load_sets_pc(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xA9, 0x42, 0x00]), origin=0x0200)
        assert sim.get_state().pc == 0x0200

    def test_load_at_default_origin(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xEA, 0x00]))
        assert sim.get_state().pc == 0x0000

    def test_load_invalid_origin(self) -> None:
        sim = MOS6502Simulator()
        with pytest.raises(ValueError):
            sim.load(bytes([0x00]), origin=0x10000)


class TestStep:
    def test_step_executes_one_instruction(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xA9, 0x42, 0x00]))
        trace = sim.step()
        assert trace.mnemonic == "LDA"
        assert trace.pc_before == 0x0000
        assert trace.pc_after == 0x0002
        assert sim.get_state().a == 0x42

    def test_step_after_halt_raises(self) -> None:
        sim = MOS6502Simulator()
        sim.execute(bytes([0x00]))
        with pytest.raises(RuntimeError):
            sim.step()

    def test_step_returns_step_trace(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xEA, 0x00]))  # NOP, BRK
        trace = sim.step()
        assert trace.mnemonic == "NOP"


class TestExecute:
    def test_execute_returns_halted_true(self) -> None:
        result = MOS6502Simulator().execute(bytes([0x00]))
        assert result.halted is True

    def test_execute_step_count(self) -> None:
        # NOP NOP NOP BRK = 4 steps
        result = MOS6502Simulator().execute(bytes([0xEA, 0xEA, 0xEA, 0x00]))
        assert result.steps == 4

    def test_execute_traces(self) -> None:
        result = MOS6502Simulator().execute(bytes([0xEA, 0x00]))
        assert result.traces[0].mnemonic == "NOP"
        assert result.traces[1].mnemonic == "BRK"

    def test_max_steps_prevents_infinite_loop(self) -> None:
        # JMP $0000 = infinite loop
        prog = bytes([0x4C, 0x00, 0x00])
        result = MOS6502Simulator().execute(prog, max_steps=100)
        assert result.steps == 100
        assert result.halted is False

    def test_execute_preserves_ports(self) -> None:
        sim = MOS6502Simulator()
        sim.set_input_port(1, 0xAB)
        sim.execute(bytes([0x00]))  # BRK immediately
        result = sim.execute(bytes([
            0xAD, 0x01, 0xFF,  # LDA $FF01 (port 1)
            0x00,
        ]))
        assert result.final_state.a == 0xAB

    def test_get_state_returns_mos6502state(self) -> None:
        sim = MOS6502Simulator()
        state = sim.get_state()
        assert isinstance(state, MOS6502State)


class TestPorts:
    def test_set_input_port_range(self) -> None:
        sim = MOS6502Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(240, 0)

    def test_set_input_port_value_range(self) -> None:
        sim = MOS6502Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(0, 256)

    def test_get_output_port_range(self) -> None:
        sim = MOS6502Simulator()
        with pytest.raises(ValueError):
            sim.get_output_port(240)

    def test_io_via_memory_mapped(self) -> None:
        sim = MOS6502Simulator()
        sim.set_input_port(5, 0xCD)
        result = sim.execute(bytes([
            0xAD, 0x05, 0xFF,  # LDA $FF05
            0x8D, 0x0A, 0xFF,  # STA $FF0A
            0x00,
        ]))
        assert result.final_state.a == 0xCD
        assert sim.get_output_port(10) == 0xCD
