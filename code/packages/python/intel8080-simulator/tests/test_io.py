"""Tests for Intel 8080 I/O and control instructions.

Covers: IN port, OUT port, EI, DI, NOP
"""

from __future__ import annotations

import pytest

from intel8080_simulator import Intel8080Simulator


def run(program: list[int]) -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim.reset()
    sim.load(bytes(program + [0x76]))
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


class TestIN:
    def test_in_reads_port(self) -> None:
        sim = Intel8080Simulator()
        sim.set_input_port(5, 0xAB)
        result = sim.execute(bytes([0xDB, 0x05, 0x76]))  # IN 5; HLT
        assert result.final_state.a == 0xAB

    def test_in_port_zero(self) -> None:
        sim = Intel8080Simulator()
        sim.set_input_port(0, 0x42)
        result = sim.execute(bytes([0xDB, 0x00, 0x76]))
        assert result.final_state.a == 0x42

    def test_in_port_255(self) -> None:
        sim = Intel8080Simulator()
        sim.set_input_port(255, 0xFF)
        result = sim.execute(bytes([0xDB, 0xFF, 0x76]))
        assert result.final_state.a == 0xFF

    def test_in_uninitialized_port_is_zero(self) -> None:
        result = Intel8080Simulator().execute(bytes([0xDB, 0x42, 0x76]))
        assert result.final_state.a == 0

    def test_set_input_port_invalid_port(self) -> None:
        sim = Intel8080Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(256, 0)

    def test_set_input_port_invalid_value(self) -> None:
        sim = Intel8080Simulator()
        with pytest.raises(ValueError):
            sim.set_input_port(0, 256)


class TestOUT:
    def test_out_writes_port(self) -> None:
        sim = Intel8080Simulator()
        result = sim.execute(bytes([0x3E, 0x77, 0xD3, 0x03, 0x76]))  # MVI A,0x77; OUT 3; HLT  # noqa: E501
        assert result.final_state.output_ports[3] == 0x77

    def test_out_port_255(self) -> None:
        sim = Intel8080Simulator()
        result = sim.execute(bytes([0x3E, 0xAA, 0xD3, 0xFF, 0x76]))  # MVI A,0xAA; OUT 255  # noqa: E501
        assert result.final_state.output_ports[255] == 0xAA

    def test_out_does_not_change_a(self) -> None:
        sim = Intel8080Simulator()
        result = sim.execute(bytes([0x3E, 0x55, 0xD3, 0x00, 0x76]))  # MVI A,0x55; OUT 0
        assert result.final_state.a == 0x55  # A unchanged

    def test_get_output_port(self) -> None:
        sim = Intel8080Simulator()
        sim.execute(bytes([0x3E, 0x42, 0xD3, 0x07, 0x76]))
        assert sim.get_output_port(7) == 0x42

    def test_get_output_port_invalid(self) -> None:
        sim = Intel8080Simulator()
        with pytest.raises(ValueError):
            sim.get_output_port(256)


class TestEIDI:
    def test_ei_enables_interrupts(self) -> None:
        result = Intel8080Simulator().execute(bytes([0xFB, 0x76]))  # EI; HLT
        assert result.final_state.interrupts_enabled is True

    def test_di_disables_interrupts(self) -> None:
        result = Intel8080Simulator().execute(bytes([0xFB, 0xF3, 0x76]))  # EI; DI; HLT
        assert result.final_state.interrupts_enabled is False

    def test_interrupts_disabled_on_reset(self) -> None:
        sim = Intel8080Simulator()
        assert sim.get_state().interrupts_enabled is False


class TestNOP:
    def test_nop_does_nothing(self) -> None:
        result = Intel8080Simulator().execute(bytes([0x00, 0x00, 0x00, 0x76]))  # NOP NOP NOP HLT  # noqa: E501
        state = result.final_state
        assert state.a == 0
        assert state.b == 0
        assert state.pc == 4  # advanced past all 3 NOPs and HLT
        assert result.steps == 4

    def test_nop_in_trace(self) -> None:
        result = Intel8080Simulator().execute(bytes([0x00, 0x76]))  # NOP; HLT
        assert result.traces[0].mnemonic == "NOP"
