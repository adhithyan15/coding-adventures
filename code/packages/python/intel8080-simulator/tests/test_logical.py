"""Tests for Intel 8080 logical instructions.

Covers: ANA, ANI, ORA, ORI, XRA, XRI, CMP, CPI, RLC, RRC, RAL, RAR, CMA, CMC, STC
"""

from __future__ import annotations

from intel8080_simulator import Intel8080Simulator


def run(program: list[int]) -> Intel8080Simulator:
    sim = Intel8080Simulator()
    sim.reset()
    sim.load(bytes(program + [0x76]))
    while not sim._halted:  # noqa: SLF001
        sim.step()
    return sim


class TestANA:
    def test_ana_b(self) -> None:
        sim = run([0x3E, 0xFF, 0x06, 0x0F, 0xA0])  # MVI A,0xFF; MVI B,0x0F; ANA B
        assert sim._a == 0x0F  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_ani_immediate(self) -> None:
        sim = run([0x3E, 0xFF, 0xE6, 0xAA])  # MVI A,0xFF; ANI 0xAA
        assert sim._a == 0xAA  # noqa: SLF001

    def test_ana_clears_carry(self) -> None:
        sim = run([0x37, 0x3E, 0xFF, 0x06, 0xFF, 0xA0])  # STC; MVI A,0xFF; MVI B,0xFF; ANA B  # noqa: E501
        assert sim._flag_cy is False  # noqa: SLF001

    def test_ana_sets_parity(self) -> None:
        sim = run([0x3E, 0xFF, 0x06, 0xFF, 0xA0])  # 0xFF & 0xFF = 0xFF → 8 ones → even
        assert sim._flag_p is True  # noqa: SLF001

    def test_ana_zero_result(self) -> None:
        sim = run([0x3E, 0x0F, 0x06, 0xF0, 0xA0])  # 0x0F & 0xF0 = 0
        assert sim._a == 0x00  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001


class TestORA:
    def test_ora_b(self) -> None:
        sim = run([0x3E, 0x0F, 0x06, 0xF0, 0xB0])  # MVI A,0x0F; MVI B,0xF0; ORA B
        assert sim._a == 0xFF  # noqa: SLF001

    def test_ori_immediate(self) -> None:
        sim = run([0x3E, 0x0F, 0xF6, 0xF0])  # MVI A,0x0F; ORI 0xF0
        assert sim._a == 0xFF  # noqa: SLF001

    def test_ora_clears_carry(self) -> None:
        sim = run([0x37, 0x3E, 0x01, 0x06, 0x02, 0xB0])  # STC; MVI A,1; MVI B,2; ORA B
        assert sim._flag_cy is False  # noqa: SLF001

    def test_ora_clears_ac(self) -> None:
        # ORA always clears AC
        sim = run([0x3E, 0xFF, 0x06, 0xFF, 0xB0])
        assert sim._flag_ac is False  # noqa: SLF001


class TestXRA:
    def test_xra_a_clears_accumulator(self) -> None:
        sim = run([0x3E, 0xFF, 0xAF])  # MVI A,0xFF; XRA A
        assert sim._a == 0x00  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001

    def test_xri_immediate(self) -> None:
        sim = run([0x3E, 0xFF, 0xEE, 0x0F])  # MVI A,0xFF; XRI 0x0F
        assert sim._a == 0xF0  # noqa: SLF001

    def test_xra_clears_carry_and_ac(self) -> None:
        sim = run([0x37, 0x3E, 0x01, 0x06, 0x02, 0xA8])  # STC; MVI A,1; MVI B,2; XRA B
        assert sim._flag_cy is False  # noqa: SLF001
        assert sim._flag_ac is False  # noqa: SLF001


class TestCMP:
    def test_cmp_equal(self) -> None:
        sim = run([0x3E, 0x05, 0x06, 0x05, 0xB8])  # MVI A,5; MVI B,5; CMP B
        assert sim._a == 0x05  # A unchanged  # noqa: SLF001
        assert sim._flag_z is True  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_cmp_a_greater(self) -> None:
        sim = run([0x3E, 0x0A, 0x06, 0x05, 0xB8])  # MVI A,10; MVI B,5; CMP B
        assert sim._flag_z is False  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_cmp_a_less(self) -> None:
        sim = run([0x3E, 0x05, 0x06, 0x0A, 0xB8])  # MVI A,5; MVI B,10; CMP B
        assert sim._flag_cy is True  # borrow  # noqa: SLF001

    def test_cpi_immediate(self) -> None:
        sim = run([0x3E, 0x10, 0xFE, 0x10])  # MVI A,16; CPI 16
        assert sim._flag_z is True  # noqa: SLF001

    def test_cmp_a_unchanged(self) -> None:
        sim = run([0x3E, 0x42, 0x06, 0x10, 0xB8])  # MVI A,0x42; MVI B,0x10; CMP B
        assert sim._a == 0x42  # A must not change  # noqa: SLF001


class TestRotates:
    def test_rlc(self) -> None:
        # RLC: rotate left; A7 → CY; A7 → A0
        sim = run([0x3E, 0x85, 0x07])  # MVI A,0x85 (0b10000101); RLC
        # 0x85 << 1 = 0b00001010 | 1 = 0x0B; CY=1
        assert sim._a == 0x0B  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001

    def test_rlc_no_carry(self) -> None:
        sim = run([0x3E, 0x05, 0x07])  # MVI A,0x05 (0b00000101); RLC
        # 0x05 << 1 = 0x0A; CY=0
        assert sim._a == 0x0A  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001

    def test_rrc(self) -> None:
        # RRC: rotate right; A0 → CY; A0 → A7
        sim = run([0x3E, 0x85, 0x0F])  # MVI A,0x85 (0b10000101); RRC
        # A0=1 → CY=1; 0x85 >> 1 | 0x80 = 0x42 | 0x80 = 0xC2
        assert sim._a == 0xC2  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001

    def test_ral(self) -> None:
        # RAL: A7 → CY; CY → A0
        sim = run([0x37, 0x3E, 0x85, 0x17])  # STC; MVI A,0x85; RAL
        # CY_in=1; new_CY=A7=1; A = (0x85<<1)|1 = 0x0B
        assert sim._a == 0x0B  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001

    def test_ral_without_carry(self) -> None:
        sim = run([0x3E, 0x85, 0x17])  # MVI A,0x85; RAL (CY=0)
        # A = (0x85<<1)|0 = 0x0A; new_CY=1
        assert sim._a == 0x0A  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001

    def test_rar(self) -> None:
        # RAR: A0 → CY; CY → A7
        sim = run([0x37, 0x3E, 0x85, 0x1F])  # STC; MVI A,0x85; RAR
        # CY_in=1; new_CY=A0=1; A = (1<<7) | (0x85>>1) = 0x80 | 0x42 = 0xC2
        assert sim._a == 0xC2  # noqa: SLF001
        assert sim._flag_cy is True  # noqa: SLF001

    def test_rar_without_carry(self) -> None:
        sim = run([0x3E, 0x84, 0x1F])  # MVI A,0x84 (bit0=0); RAR (CY=0)
        # A = 0 | (0x84>>1) = 0x42; new_CY=0
        assert sim._a == 0x42  # noqa: SLF001
        assert sim._flag_cy is False  # noqa: SLF001


class TestCMA:
    def test_cma(self) -> None:
        sim = run([0x3E, 0xAA, 0x2F])  # MVI A,0xAA; CMA
        assert sim._a == 0x55  # noqa: SLF001

    def test_cma_does_not_affect_flags(self) -> None:
        # Run a SUB to set some flags, then CMA should not change them
        sim = run([0x3E, 0x05, 0x97, 0x2F])  # MVI A,5; SUB A (Z=1); CMA
        assert sim._flag_z is True  # Z unchanged by CMA  # noqa: SLF001


class TestSTC_CMC:
    def test_stc_sets_carry(self) -> None:
        sim = run([0x37])  # STC
        assert sim._flag_cy is True  # noqa: SLF001

    def test_cmc_toggles_carry(self) -> None:
        sim = run([0x3F])  # CMC (CY starts at 0)
        assert sim._flag_cy is True  # noqa: SLF001

    def test_cmc_twice_restores(self) -> None:
        sim = run([0x37, 0x3F, 0x3F])  # STC; CMC; CMC
        assert sim._flag_cy is True  # noqa: SLF001
