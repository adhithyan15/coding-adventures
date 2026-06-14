"""Tests for Intel 8080 arithmetic instructions.

Covers: ADD, ADC, SUB, SBB, INR, DCR, INX, DCX, DAD, DAA
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


class TestADD:
    def test_add_b_no_carry(self) -> None:
        sim = run([0x3E, 0x0A, 0x06, 0x05, 0x80])  # MVI A,10; MVI B,5; ADD B
        assert sim._a == 15  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001
        assert sim._flag_z is False  # noqa: SLF001

    def test_add_produces_carry(self) -> None:
        sim = run([0x3E, 0xFF, 0x06, 0x01, 0x80])  # MVI A,255; MVI B,1; ADD B
        assert sim._a == 0  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001

    def test_add_sets_sign(self) -> None:
        sim = run([0x3E, 0x70, 0x06, 0x70, 0x80])  # MVI A,0x70; MVI B,0x70; ADD B
        assert sim._flag_s is True  # 0xE0 — bit 7 set  # noqa: SLF001

    def test_add_sets_parity(self) -> None:
        sim = run([0x3E, 0x03, 0x06, 0x00, 0x80])  # 3 + 0 = 3 = 0b00000011 → 2 ones → even parity  # noqa: E501
        assert sim._flag_p is True  # noqa: SLF001

    def test_add_m(self) -> None:
        # LXI H,0x200; MVI M,7; MVI A,3; ADD M
        sim = run([0x21, 0x00, 0x02, 0x36, 0x07, 0x3E, 0x03, 0x86])
        assert sim._a == 10  # noqa: SLF001

    def test_adi_immediate(self) -> None:
        sim = run([0x3E, 0x20, 0xC6, 0x10])  # MVI A,0x20; ADI 0x10
        assert sim._a == 0x30  # noqa: SLF001


class TestADC:
    def test_adc_without_carry(self) -> None:
        sim = run([0x3E, 0x05, 0x06, 0x03, 0x88])  # MVI A,5; MVI B,3; ADC B (CY=0)
        assert sim._a == 8  # noqa: SLF001

    def test_adc_with_carry(self) -> None:
        # First set CY via STC (0x37), then ADC B
        sim = run([0x3E, 0x05, 0x06, 0x03, 0x37, 0x88])
        assert sim._a == 9  # 5 + 3 + 1  # noqa: SLF001

    def test_aci_with_carry(self) -> None:
        sim = run([0x3E, 0x10, 0x37, 0xCE, 0x01])  # MVI A,16; STC; ACI 1
        assert sim._a == 18  # 16 + 1 + 1  # noqa: SLF001


class TestSUB:
    def test_sub_b_no_borrow(self) -> None:
        sim = run([0x3E, 0x0A, 0x06, 0x03, 0x90])  # MVI A,10; MVI B,3; SUB B
        assert sim._a == 7  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_sub_produces_borrow(self) -> None:
        sim = run([0x3E, 0x03, 0x06, 0x0A, 0x90])  # MVI A,3; MVI B,10; SUB B
        assert sim._a == 0xF9  # 3 - 10 = -7 = 0xF9 in 8-bit two's complement  # noqa: SLF001,E501
        assert sim._flag_cy is True  # borrow  # noqa: SLF001

    def test_sub_self_yields_zero(self) -> None:
        sim = run([0x3E, 0x42, 0x97])  # MVI A,0x42; SUB A (opcode: SUB A = 0x97)
        assert sim._a == 0  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_sui_immediate(self) -> None:
        sim = run([0x3E, 0x20, 0xD6, 0x10])  # MVI A,0x20; SUI 0x10
        assert sim._a == 0x10  # noqa: SLF001


class TestSBB:
    def test_sbb_without_borrow(self) -> None:
        sim = run([0x3E, 0x08, 0x06, 0x03, 0x98])  # MVI A,8; MVI B,3; SBB B
        assert sim._a == 5  # 8 - 3 - 0  # noqa: SLF001

    def test_sbb_with_borrow(self) -> None:
        sim = run([0x3E, 0x08, 0x06, 0x03, 0x37, 0x98])  # MVI A,8; MVI B,3; STC; SBB B
        assert sim._a == 4  # 8 - 3 - 1  # noqa: SLF001

    def test_sbi(self) -> None:
        sim = run([0x3E, 0x10, 0x37, 0xDE, 0x05])  # MVI A,16; STC; SBI 5
        assert sim._a == 10  # 16 - 5 - 1  # noqa: SLF001


class TestINR:
    @pytest.mark.parametrize("mvi_op,reg_attr,inr_op", [
        (0x06, "_b", 0x04),
        (0x0E, "_c", 0x0C),
        (0x16, "_d", 0x14),
        (0x1E, "_e", 0x1C),
        (0x26, "_h", 0x24),
        (0x2E, "_l", 0x2C),
        (0x3E, "_a", 0x3C),
    ])
    def test_inr_register(self, mvi_op: int, reg_attr: str, inr_op: int) -> None:
        sim = run([mvi_op, 0x05, inr_op])
        assert getattr(sim, reg_attr) == 6  # noqa: SLF001

    def test_inr_does_not_affect_carry(self) -> None:
        sim = run([0x37, 0x3E, 0x05, 0x3C])  # STC; MVI A,5; INR A
        assert sim._flag_cy is True  # CY unchanged  # noqa: SLF001

    def test_inr_wraps(self) -> None:
        sim = run([0x3E, 0xFF, 0x3C])  # MVI A,255; INR A
        assert sim._a == 0  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001

    def test_inr_m(self) -> None:
        sim = run([0x21, 0x00, 0x02, 0x36, 0x09, 0x34])  # LXI H,0x200; MVI M,9; INR M
        assert sim._memory[0x200] == 10  # noqa: SLF001


class TestDCR:
    def test_dcr_a(self) -> None:
        sim = run([0x3E, 0x05, 0x3D])  # MVI A,5; DCR A
        assert sim._a == 4  # noqa: SLF001

    def test_dcr_does_not_affect_carry(self) -> None:
        sim = run([0x37, 0x3E, 0x05, 0x3D])  # STC; MVI A,5; DCR A
        assert sim._flag_cy is True  # noqa: SLF001

    def test_dcr_wraps(self) -> None:
        sim = run([0x3E, 0x00, 0x3D])  # MVI A,0; DCR A
        assert sim._a == 0xFF  # noqa: SLF001


class TestINXDCX:
    def test_inx_b(self) -> None:
        sim = run([0x01, 0xFF, 0x00, 0x03])  # LXI B,0x00FF; INX B
        assert sim._b == 0x01  # noqa: SLF001
        assert sim._c == 0x00  # noqa: SLF001

    def test_inx_wraps_16bit(self) -> None:
        sim = run([0x01, 0xFF, 0xFF, 0x03])  # LXI B,0xFFFF; INX B
        assert sim._b == 0x00  # noqa: SLF001
        assert sim._c == 0x00  # noqa: SLF001

    def test_inx_no_flags(self) -> None:
        # INX does not affect any flags
        sim = run([0x3E, 0x00, 0x97, 0x01, 0xFF, 0xFF, 0x03])
        # After SUB A (Z=1), INX B — Z should still be 1
        assert sim._flag_z is True  # noqa: SLF001

    def test_dcx_h(self) -> None:
        sim = run([0x21, 0x00, 0x01, 0x2B])  # LXI H,0x0100; DCX H
        assert sim._h == 0x00  # noqa: SLF001
        assert sim._l == 0xFF  # noqa: SLF001

    def test_dcx_sp(self) -> None:
        sim = run([0x31, 0x01, 0x00, 0x3B])  # LXI SP,0x0001; DCX SP
        assert sim._sp == 0x0000  # noqa: SLF001


class TestDAD:
    def test_dad_b(self) -> None:
        sim = run([
            0x21, 0x34, 0x12,  # LXI H,0x1234
            0x01, 0x78, 0x56,  # LXI B,0x5678
            0x09,              # DAD B
        ])
        assert (sim._h << 8 | sim._l) == 0x1234 + 0x5678  # noqa: SLF001

    def test_dad_produces_carry(self) -> None:
        sim = run([
            0x21, 0x00, 0xFF,  # LXI H,0xFF00
            0x01, 0x00, 0x01,  # LXI B,0x0100
            0x09,              # DAD B  → 0xFF00 + 0x0100 = 0x10000
        ])
        assert sim._flag_cy is True  # noqa: SLF001
        assert (sim._h << 8 | sim._l) == 0x0000  # low 16 bits  # noqa: SLF001

    def test_dad_h(self) -> None:
        # DAD H doubles HL
        sim = run([0x21, 0x10, 0x00, 0x29])  # LXI H,0x0010; DAD H
        assert (sim._h << 8 | sim._l) == 0x0020  # noqa: SLF001

    def test_dad_sp(self) -> None:
        sim = run([
            0x21, 0x00, 0x10,  # LXI H,0x1000
            0x31, 0x00, 0x01,  # LXI SP,0x0100
            0x39,              # DAD SP
        ])
        assert (sim._h << 8 | sim._l) == 0x1100  # noqa: SLF001


class TestDAA:
    def test_daa_no_adjustment_needed(self) -> None:
        # 0x05 is valid BCD, no adjustment
        sim = run([0x3E, 0x05, 0x27])  # MVI A,0x05; DAA
        assert sim._a == 0x05  # noqa: SLF001

    def test_daa_low_nibble_correction(self) -> None:
        # 0x0A (invalid BCD) → add 6 → 0x10
        sim = run([0x3E, 0x0A, 0x27])  # MVI A,0x0A; DAA
        assert sim._a == 0x10  # noqa: SLF001

    def test_daa_after_bcd_add(self) -> None:
        # BCD addition: 25 + 38 = 63
        # 0x25 + 0x38 = 0x5D; after DAA → 0x63
        sim = run([0x3E, 0x25, 0x06, 0x38, 0x80, 0x27])  # MVI A,0x25; MVI B,0x38; ADD B; DAA  # noqa: E501
        assert sim._a == 0x63  # noqa: SLF001
