"""Tests for LDA/LDX/LDY/STA/STX/STY across all addressing modes."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]) -> MOS6502Simulator:
    sim = MOS6502Simulator()
    sim.execute(bytes(prog + [0x00]))
    return sim


def state(prog: list[int]):
    return run(prog).get_state()


class TestLDA:
    def test_immediate(self) -> None:
        s = state([0xA9, 0x42])
        assert s.a == 0x42

    def test_zero_page(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xA5, 0x10, 0x00]))
        sim._memory[0x10] = 0x77
        sim.execute(bytes([0xA5, 0x10, 0x00]))
        # Re-run with memory pre-seeded
        sim2 = MOS6502Simulator()
        sim2.reset()
        sim2._memory[0x10] = 0x77
        sim2.load(bytes([0xA5, 0x10, 0x00]))
        while not sim2._halted:
            sim2.step()
        assert sim2.get_state().a == 0x77

    def test_zero_page_x(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x15] = 0x99
        sim.load(bytes([0xA2, 0x05, 0xB5, 0x10, 0x00]))  # LDX #5; LDA $10,X
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0x99

    def test_absolute(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x0200] = 0xAB
        sim.load(bytes([0xAD, 0x00, 0x02, 0x00]))  # LDA $0200
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0xAB

    def test_absolute_x(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x0205] = 0x11
        sim.load(bytes([0xA2, 0x05, 0xBD, 0x00, 0x02, 0x00]))  # LDX #5; LDA $0200,X
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0x11

    def test_absolute_y(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x0203] = 0x22
        sim.load(bytes([0xA0, 0x03, 0xB9, 0x00, 0x02, 0x00]))  # LDY #3; LDA $0200,Y
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0x22

    def test_indirect_x(self) -> None:
        # (zp,X): pointer at (0x10 + X=0x02) = 0x12 → address = [0x12:0x13]
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x12] = 0x34  # lo of target address
        sim._memory[0x13] = 0x02  # hi → target = $0234
        sim._memory[0x0234] = 0x55
        sim.load(bytes([0xA2, 0x02, 0xA1, 0x10, 0x00]))  # LDX #2; LDA ($10,X)
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0x55

    def test_indirect_y(self) -> None:
        # (zp),Y: base address at [0x20:0x21], + Y=3 → target
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x20] = 0x00  # lo
        sim._memory[0x21] = 0x03  # hi → base = $0300
        sim._memory[0x0303] = 0x66  # base + Y=3
        sim.load(bytes([0xA0, 0x03, 0xB1, 0x20, 0x00]))  # LDY #3; LDA ($20),Y
        while not sim._halted:
            sim.step()
        assert sim.get_state().a == 0x66

    def test_sets_n_flag(self) -> None:
        s = state([0xA9, 0x80])
        assert s.flag_n is True
        assert s.flag_z is False

    def test_sets_z_flag(self) -> None:
        s = state([0xA9, 0x00])
        assert s.flag_z is True
        assert s.flag_n is False


class TestLDX:
    def test_immediate(self) -> None:
        s = state([0xA2, 0x55])
        assert s.x == 0x55

    def test_zero_page_y(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x13] = 0xAA
        sim.load(bytes([0xA0, 0x03, 0xB6, 0x10, 0x00]))  # LDY #3; LDX $10,Y
        while not sim._halted:
            sim.step()
        assert sim.get_state().x == 0xAA


class TestLDY:
    def test_immediate(self) -> None:
        s = state([0xA0, 0x0F])
        assert s.y == 0x0F

    def test_zero_page_x(self) -> None:
        sim = MOS6502Simulator()
        sim.reset()
        sim._memory[0x12] = 0xBB
        sim.load(bytes([0xA2, 0x02, 0xB4, 0x10, 0x00]))  # LDX #2; LDY $10,X
        while not sim._halted:
            sim.step()
        assert sim.get_state().y == 0xBB


class TestSTA:
    def test_zero_page(self) -> None:
        sim = MOS6502Simulator()
        result = sim.execute(bytes([0xA9, 0x42, 0x85, 0x10, 0x00]))  # LDA #$42; STA $10
        assert result.final_state.memory[0x10] == 0x42

    def test_absolute(self) -> None:
        sim = MOS6502Simulator()
        result = sim.execute(bytes([0xA9, 0x55, 0x8D, 0x00, 0x03, 0x00]))
        assert result.final_state.memory[0x0300] == 0x55


class TestSTX:
    def test_zero_page(self) -> None:
        sim = MOS6502Simulator()
        result = sim.execute(bytes([0xA2, 0x33, 0x86, 0x20, 0x00]))  # LDX #$33; STX $20
        assert result.final_state.memory[0x20] == 0x33


class TestSTY:
    def test_zero_page(self) -> None:
        sim = MOS6502Simulator()
        result = sim.execute(bytes([0xA0, 0x44, 0x84, 0x30, 0x00]))  # LDY #$44; STY $30
        assert result.final_state.memory[0x30] == 0x44
