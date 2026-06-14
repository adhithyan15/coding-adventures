"""Tests for AND, ORA, EOR, BIT, ASL, LSR, ROL, ROR."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]):
    return MOS6502Simulator().execute(bytes(prog + [0x00])).final_state


class TestAND:
    def test_basic(self) -> None:
        s = run([0xA9, 0xFF, 0x29, 0x0F])
        assert s.a == 0x0F

    def test_zero(self) -> None:
        s = run([0xA9, 0xF0, 0x29, 0x0F])
        assert s.a == 0
        assert s.flag_z is True

    def test_sets_n(self) -> None:
        s = run([0xA9, 0xFF, 0x29, 0x80])
        assert s.flag_n is True


class TestORA:
    def test_basic(self) -> None:
        s = run([0xA9, 0x0F, 0x09, 0xF0])
        assert s.a == 0xFF

    def test_sets_n(self) -> None:
        s = run([0xA9, 0x00, 0x09, 0x80])
        assert s.flag_n is True


class TestEOR:
    def test_self_clears(self) -> None:
        s = run([0xA9, 0x42, 0x49, 0x42])
        assert s.a == 0
        assert s.flag_z is True

    def test_basic(self) -> None:
        s = run([0xA9, 0xFF, 0x49, 0x0F])
        assert s.a == 0xF0

    def test_sets_n(self) -> None:
        s = run([0xA9, 0x00, 0x49, 0x80])
        assert s.flag_n is True


class TestBIT:
    def test_zero_result(self) -> None:
        # LDA #$0F; STA $50; LDA #$F0; BIT $50 → Z=1
        result = MOS6502Simulator().execute(bytes([
            0xA9, 0x0F, 0x85, 0x50,   # LDA #$0F; STA $50
            0xA9, 0xF0,                # LDA #$F0
            0x24, 0x50,                # BIT $50
            0x00,
        ]))
        s = result.final_state
        assert s.flag_z is True
        assert s.flag_n is False   # bit 7 of mem = 0
        assert s.flag_v is False   # bit 6 of mem = 0

    def test_n_v_from_memory(self) -> None:
        # Memory has 0xC0 (N=1, V=1 from bits 7,6)
        result = MOS6502Simulator().execute(bytes([
            0xA9, 0xC0, 0x85, 0x50,   # LDA #$C0; STA $50
            0xA9, 0xFF,                # LDA #$FF
            0x24, 0x50,                # BIT $50 → N=1 V=1 Z=0 (A&M=C0≠0)
            0x00,
        ]))
        s = result.final_state
        assert s.flag_n is True
        assert s.flag_v is True
        assert s.flag_z is False


class TestASL:
    def test_accumulator(self) -> None:
        s = run([0xA9, 0x41, 0x0A])
        assert s.a == 0x82
        assert s.flag_c is False
        assert s.flag_n is True

    def test_carry_out(self) -> None:
        s = run([0xA9, 0x81, 0x0A])
        assert s.a == 0x02
        assert s.flag_c is True

    def test_memory(self) -> None:
        result = MOS6502Simulator().execute(bytes([
            0xA9, 0x02, 0x85, 0x10,   # LDA #2; STA $10
            0x06, 0x10,                # ASL $10
            0x00,
        ]))
        assert result.final_state.memory[0x10] == 4


class TestLSR:
    def test_accumulator(self) -> None:
        s = run([0xA9, 0x42, 0x4A])
        assert s.a == 0x21
        assert s.flag_c is False

    def test_carry_from_bit0(self) -> None:
        s = run([0xA9, 0x01, 0x4A])
        assert s.a == 0
        assert s.flag_c is True
        assert s.flag_z is True

    def test_n_always_clear(self) -> None:
        # LSR always shifts 0 into bit 7 → N always 0
        s = run([0xA9, 0xFF, 0x4A])
        assert s.flag_n is False


class TestROL:
    def test_with_carry(self) -> None:
        # SEC; LDA #$40; ROL → 0x81, C=0
        s = run([0x38, 0xA9, 0x40, 0x2A])
        assert s.a == 0x81
        assert s.flag_c is False

    def test_carry_rotated_in(self) -> None:
        # SEC; LDA #$00; ROL → 0x01
        s = run([0x38, 0xA9, 0x00, 0x2A])
        assert s.a == 0x01

    def test_carry_rotated_out(self) -> None:
        # CLC; LDA #$80; ROL → 0x00, C=1
        s = run([0x18, 0xA9, 0x80, 0x2A])
        assert s.a == 0
        assert s.flag_c is True


class TestROR:
    def test_with_carry(self) -> None:
        # SEC; LDA #$02; ROR → 0x81, C=0
        s = run([0x38, 0xA9, 0x02, 0x6A])
        assert s.a == 0x81
        assert s.flag_c is False

    def test_carry_rotated_in(self) -> None:
        # SEC; LDA #$00; ROR → 0x80, C=0
        s = run([0x38, 0xA9, 0x00, 0x6A])
        assert s.a == 0x80
        assert s.flag_n is True

    def test_carry_rotated_out(self) -> None:
        # CLC; LDA #$01; ROR → 0x00, C=1
        s = run([0x18, 0xA9, 0x01, 0x6A])
        assert s.a == 0
        assert s.flag_c is True
        assert s.flag_z is True
