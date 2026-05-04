"""Tests for ADC, SBC, INC, DEC, INX, INY, DEX, DEY."""

from __future__ import annotations

from mos6502_simulator import MOS6502Simulator


def run(prog: list[int]):
    sim = MOS6502Simulator()
    return sim.execute(bytes(prog + [0x00])).final_state


class TestADC:
    def test_simple_add(self) -> None:
        # LDA #5; ADC #3 → A=8, C=0
        s = run([0xA9, 0x05, 0x69, 0x03])
        assert s.a == 8
        assert s.flag_c is False
        assert s.flag_z is False
        assert s.flag_n is False

    def test_carry_out(self) -> None:
        # LDA #$FF; ADC #$01 → A=0, C=1, Z=1
        s = run([0xA9, 0xFF, 0x69, 0x01])
        assert s.a == 0
        assert s.flag_c is True
        assert s.flag_z is True

    def test_add_with_carry_in(self) -> None:
        # SEC; LDA #5; ADC #3 → A=9 (5+3+1)
        s = run([0x38, 0xA9, 0x05, 0x69, 0x03])
        assert s.a == 9

    def test_overflow_positive(self) -> None:
        # 0x7F + 0x01 = 0x80 → overflow (positive + positive → negative)
        s = run([0xA9, 0x7F, 0x69, 0x01])
        assert s.a == 0x80
        assert s.flag_v is True
        assert s.flag_n is True

    def test_overflow_negative(self) -> None:
        # 0x80 + 0xFF = 0x7F → overflow (negative + negative → positive)
        s = run([0xA9, 0x80, 0x69, 0xFF])
        assert s.a == 0x7F
        assert s.flag_v is True
        assert s.flag_n is False

    def test_no_overflow(self) -> None:
        # 0x50 + 0x10 = 0x60: no overflow
        s = run([0xA9, 0x50, 0x69, 0x10])
        assert s.flag_v is False

    def test_bcd_simple(self) -> None:
        # SED; LDA #$09; ADC #$01 → A=$10 in BCD
        s = run([0xF8, 0xA9, 0x09, 0x69, 0x01])
        assert s.a == 0x10
        assert s.flag_c is False

    def test_bcd_carry(self) -> None:
        # SED; LDA #$99; ADC #$01 → A=$00, C=1
        s = run([0xF8, 0xA9, 0x99, 0x69, 0x01])
        assert s.a == 0x00
        assert s.flag_c is True


class TestSBC:
    def test_simple_sub(self) -> None:
        # SEC; LDA #10; SBC #3 → A=7, C=1 (no borrow)
        s = run([0x38, 0xA9, 0x0A, 0xE9, 0x03])
        assert s.a == 7
        assert s.flag_c is True
        assert s.flag_n is False
        assert s.flag_z is False

    def test_borrow(self) -> None:
        # SEC; LDA #3; SBC #10 → A=249, C=0 (borrow)
        s = run([0x38, 0xA9, 0x03, 0xE9, 0x0A])
        assert s.a == 0xF9
        assert s.flag_c is False
        assert s.flag_n is True

    def test_result_zero(self) -> None:
        # SEC; LDA #5; SBC #5 → A=0, Z=1, C=1
        s = run([0x38, 0xA9, 0x05, 0xE9, 0x05])
        assert s.a == 0
        assert s.flag_z is True
        assert s.flag_c is True

    def test_overflow_sub(self) -> None:
        # SEC; LDA #$80; SBC #$01 → A=$7F, overflow
        s = run([0x38, 0xA9, 0x80, 0xE9, 0x01])
        assert s.a == 0x7F
        assert s.flag_v is True

    def test_bcd_sub(self) -> None:
        # SED; SEC; LDA #$10; SBC #$01 → A=$09
        s = run([0xF8, 0x38, 0xA9, 0x10, 0xE9, 0x01])
        assert s.a == 0x09
        assert s.flag_c is True


class TestINCDEC:
    def test_inx(self) -> None:
        s = run([0xA2, 0x05, 0xE8])
        assert s.x == 6

    def test_inx_wrap(self) -> None:
        s = run([0xA2, 0xFF, 0xE8])
        assert s.x == 0
        assert s.flag_z is True

    def test_iny(self) -> None:
        s = run([0xA0, 0x07, 0xC8])
        assert s.y == 8

    def test_dex(self) -> None:
        s = run([0xA2, 0x05, 0xCA])
        assert s.x == 4

    def test_dex_wrap(self) -> None:
        s = run([0xA2, 0x00, 0xCA])
        assert s.x == 0xFF
        assert s.flag_n is True

    def test_dey(self) -> None:
        s = run([0xA0, 0x05, 0x88])
        assert s.y == 4

    def test_inc_memory(self) -> None:
        # LDA #9; STA $50; INC $50
        result = MOS6502Simulator().execute(bytes([
            0xA9, 0x09, 0x85, 0x50,   # LDA #9; STA $50
            0xE6, 0x50,                # INC $50
            0x00,
        ]))
        assert result.final_state.memory[0x50] == 10

    def test_dec_memory(self) -> None:
        # LDA #5; STA $60; DEC $60
        result = MOS6502Simulator().execute(bytes([
            0xA9, 0x05, 0x85, 0x60,   # LDA #5; STA $60
            0xC6, 0x60,                # DEC $60
            0x00,
        ]))
        assert result.final_state.memory[0x60] == 4
