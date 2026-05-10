"""Tests for Intel8080State — frozen immutable snapshot."""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from intel8080_simulator import Intel8080Simulator, Intel8080State


def _make_state(**overrides: object) -> Intel8080State:
    base: dict[str, object] = dict(
        a=0, b=0, c=0, d=0, e=0, h=0, l=0,
        sp=0, pc=0,
        flag_s=False, flag_z=False, flag_ac=False, flag_p=False, flag_cy=False,
        interrupts_enabled=False, halted=False,
        memory=tuple([0] * 65536),
        input_ports=tuple([0] * 256),
        output_ports=tuple([0] * 256),
    )
    base.update(overrides)
    return Intel8080State(**base)  # type: ignore[arg-type]


class TestStateImmutability:
    def test_frozen(self) -> None:
        state = _make_state()
        with pytest.raises(FrozenInstanceError):
            state.a = 99  # type: ignore[misc]

    def test_memory_is_tuple(self) -> None:
        state = _make_state()
        assert isinstance(state.memory, tuple)
        with pytest.raises(TypeError):
            state.memory[0] = 1  # type: ignore[index]

    def test_input_ports_is_tuple(self) -> None:
        state = _make_state()
        assert isinstance(state.input_ports, tuple)

    def test_output_ports_is_tuple(self) -> None:
        state = _make_state()
        assert isinstance(state.output_ports, tuple)


class TestStateDerivedProperties:
    def test_hl_pair(self) -> None:
        state = _make_state(h=0x12, l=0x34)
        assert state.hl == 0x1234

    def test_bc_pair(self) -> None:
        state = _make_state(b=0xAB, c=0xCD)
        assert state.bc == 0xABCD

    def test_de_pair(self) -> None:
        state = _make_state(d=0x00, e=0xFF)
        assert state.de == 0x00FF

    def test_flags_byte_all_clear(self) -> None:
        state = _make_state()
        # All flags clear: S=0, Z=0, AC=0, P=0, CY=0; bit1 always 1
        assert state.flags_byte == 0b00000010  # 0x02

    def test_flags_byte_all_set(self) -> None:
        state = _make_state(
            flag_s=True, flag_z=True, flag_ac=True, flag_p=True, flag_cy=True
        )
        # S=1,Z=1,0,AC=1,0,P=1,1,CY=1 → bits 7,6,4,2,1,0 = 11010111 = 0xD7
        assert state.flags_byte == 0xD7

    def test_flags_byte_carry_only(self) -> None:
        state = _make_state(flag_cy=True)
        # bit1=1 always, bit0=CY=1 → 0x03
        assert state.flags_byte == 0x03

    def test_flags_byte_zero_only(self) -> None:
        state = _make_state(flag_z=True)
        # bit6=1, bit1=1 → 0x42
        assert state.flags_byte == 0x42


class TestGetState:
    def test_returns_intel8080state(self) -> None:
        sim = Intel8080Simulator()
        assert isinstance(sim.get_state(), Intel8080State)

    def test_snapshot_independent_of_mutation(self) -> None:
        sim = Intel8080Simulator()
        snap = sim.get_state()
        sim._memory[10] = 0xFF  # noqa: SLF001
        assert snap.memory[10] == 0

    def test_reflects_current_state(self) -> None:
        sim = Intel8080Simulator()
        sim._a = 42  # noqa: SLF001
        assert sim.get_state().a == 42
