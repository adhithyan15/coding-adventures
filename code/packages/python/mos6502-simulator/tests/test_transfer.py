"""Tests for TAX, TAY, TXA, TYA, TSX, TXS and compare instructions."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]):
    return MOS6502Simulator().execute(bytes(prog + [0x00])).final_state


class TestTransfers:
    def test_tax(self) -> None:
        s = run([0xA9, 0x42, 0xAA])
        assert s.x == 0x42

    def test_tax_sets_nz(self) -> None:
        s = run([0xA9, 0x00, 0xAA])
        assert s.flag_z is True

    def test_tay(self) -> None:
        s = run([0xA9, 0x55, 0xA8])
        assert s.y == 0x55

    def test_txa(self) -> None:
        s = run([0xA2, 0x33, 0x8A])
        assert s.a == 0x33

    def test_tya(self) -> None:
        s = run([0xA0, 0x11, 0x98])
        assert s.a == 0x11

    def test_tsx(self) -> None:
        # After reset, S = 0xFD
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xBA, 0x00]))  # TSX
        while not sim._halted:
            sim.step()
        assert sim.get_state().x == 0xFD

    def test_txs_no_flags(self) -> None:
        # TXS does NOT set N or Z.  We step through to check S right after
        # TXS — BRK would push 3 bytes onto the stack and change S.
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0xA2, 0x00, 0x9A, 0x00]))  # LDX #0; TXS; BRK
        sim.step()  # LDX #0  → Z=1
        sim.step()  # TXS     → S=0x00, flags unchanged
        s = sim.get_state()
        assert s.s == 0x00
        # N and Z reflect LDX #0, not TXS
        assert s.flag_z is True

        # Check that TXS doesn't re-set Z: after TAX copies 1 into X, Z=0;
        # then TXS should leave Z alone.
        sim2 = MOS6502Simulator()
        sim2.reset()
        sim2.load(bytes([0xA9, 0x01, 0xAA, 0x9A, 0x00]))  # LDA#1; TAX; TXS; BRK
        sim2.step()  # LDA #1
        sim2.step()  # TAX → X=1, Z=0
        sim2.step()  # TXS → S=1, Z unchanged
        s2 = sim2.get_state()
        assert s2.s == 0x01
        assert s2.flag_z is False  # Z still from TAX


class TestCompare:
    def test_cmp_equal(self) -> None:
        # LDA #5; CMP #5 → Z=1 C=1 N=0
        s = run([0xA9, 0x05, 0xC9, 0x05])
        assert s.flag_z is True
        assert s.flag_c is True
        assert s.flag_n is False

    def test_cmp_greater(self) -> None:
        # LDA #10; CMP #5 → Z=0 C=1 N=0
        s = run([0xA9, 0x0A, 0xC9, 0x05])
        assert s.flag_z is False
        assert s.flag_c is True

    def test_cmp_less(self) -> None:
        # LDA #3; CMP #5 → C=0
        s = run([0xA9, 0x03, 0xC9, 0x05])
        assert s.flag_c is False
        assert s.flag_n is True

    def test_cpx(self) -> None:
        s = run([0xA2, 0x05, 0xE0, 0x05])
        assert s.flag_z is True
        assert s.flag_c is True

    def test_cpy(self) -> None:
        s = run([0xA0, 0x08, 0xC0, 0x04])
        assert s.flag_c is True
        assert s.flag_z is False

    def test_cmp_does_not_change_a(self) -> None:
        s = run([0xA9, 0x42, 0xC9, 0x10])
        assert s.a == 0x42  # A unchanged after CMP


class TestFlagInstructions:
    def test_clc(self) -> None:
        s = run([0x38, 0x18])  # SEC; CLC
        assert s.flag_c is False

    def test_sec(self) -> None:
        s = run([0x38])
        assert s.flag_c is True

    def test_cld(self) -> None:
        s = run([0xF8, 0xD8])  # SED; CLD
        assert s.flag_d is False

    def test_sed(self) -> None:
        s = run([0xF8])
        assert s.flag_d is True

    def test_cli(self) -> None:
        # CLI clears the I flag.  We check state right after CLI rather than
        # after BRK, because BRK re-sets I=1 as part of its interrupt-handling
        # behaviour.
        sim = MOS6502Simulator()
        sim.reset()
        sim.load(bytes([0x78, 0x58, 0x00]))  # SEI; CLI; BRK
        sim.step()  # SEI → I=1
        sim.step()  # CLI → I=0
        assert sim.get_state().flag_i is False

    def test_sei(self) -> None:
        s = run([0x78])
        assert s.flag_i is True

    def test_clv(self) -> None:
        # Cause V=1 then CLV
        s = run([0xA9, 0x7F, 0x69, 0x01, 0xB8])  # LDA #$7F; ADC #1 (V=1); CLV
        assert s.flag_v is False
