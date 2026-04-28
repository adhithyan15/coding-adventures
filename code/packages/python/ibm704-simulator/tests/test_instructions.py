"""Per-instruction unit tests.

Each test puts the simulator in a known starting state, executes one or two
instructions via ``step()``, and asserts the resulting state. Where convenient
we use ``encode_type_b`` / ``encode_type_a`` to build instruction words.
"""

from __future__ import annotations

import pytest

from ibm704_simulator import (
    OP_ADD,
    OP_ADM,
    OP_CAL,
    OP_CLA,
    OP_DVH,
    OP_DVP,
    OP_FAD,
    OP_FDP,
    OP_FMP,
    OP_FSB,
    OP_HPR,
    OP_HTR,
    OP_LDQ,
    OP_LXA,
    OP_LXD,
    OP_MPY,
    OP_NOP,
    OP_PAX,
    OP_PDX,
    OP_PXA,
    OP_STO,
    OP_STQ,
    OP_STZ,
    OP_SUB,
    OP_SXA,
    OP_SXD,
    OP_TMI,
    OP_TNO,
    OP_TNZ,
    OP_TOV,
    OP_TPL,
    OP_TQO,
    OP_TQP,
    OP_TRA,
    OP_TZE,
    OP_XCA,
    PREFIX_TIX,
    PREFIX_TXH,
    PREFIX_TXI,
    PREFIX_TXL,
    IBM704Simulator,
    encode_type_a,
    encode_type_b,
    make_word,
)


def _load(sim: IBM704Simulator, *words: int, start: int = 0) -> None:
    """Write words directly into memory starting at ``start``."""
    for i, w in enumerate(words):
        sim._memory[start + i] = w  # noqa: SLF001


# ---------------------------------------------------------------------------
# Halts and NOP
# ---------------------------------------------------------------------------


class TestHalts:
    def test_htr_halts(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_HTR, 0, 100))
        sim.step()
        assert sim._halted  # noqa: SLF001
        assert sim._pc == 100  # noqa: SLF001

    def test_hpr_halts(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_HPR, 0, 50))
        sim.step()
        assert sim._halted  # noqa: SLF001
        assert sim._pc == 50  # noqa: SLF001

    def test_nop_advances_pc(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_NOP))
        sim.step()
        assert sim._pc == 1  # noqa: SLF001
        assert not sim._halted  # noqa: SLF001

    def test_step_after_halt_raises(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_HTR))
        sim.step()
        with pytest.raises(RuntimeError):
            sim.step()


# ---------------------------------------------------------------------------
# Loads and stores
# ---------------------------------------------------------------------------


class TestLoadStore:
    def test_cla_loads_positive_word(self) -> None:
        sim = IBM704Simulator()
        _load(
            sim,
            encode_type_b(OP_CLA, 0, 100),
            encode_type_b(OP_HTR),
        )
        sim._memory[100] = make_word(0, 42)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 0  # noqa: SLF001
        assert sim._ac_magnitude == 42  # noqa: SLF001
        assert sim._ac_p == 0  # noqa: SLF001
        assert sim._ac_q == 0  # noqa: SLF001

    def test_cla_loads_negative_word(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_CLA, 0, 100))
        sim._memory[100] = make_word(1, 42)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 1  # noqa: SLF001
        assert sim._ac_magnitude == 42  # noqa: SLF001

    def test_cla_clears_q_and_p(self) -> None:
        sim = IBM704Simulator()
        sim._ac_q = 1  # noqa: SLF001
        sim._ac_p = 1  # noqa: SLF001
        _load(sim, encode_type_b(OP_CLA, 0, 100))
        sim._memory[100] = make_word(0, 5)  # noqa: SLF001
        sim.step()
        assert sim._ac_q == 0  # noqa: SLF001
        assert sim._ac_p == 0  # noqa: SLF001

    def test_cal_logical_load(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_CAL, 0, 100))
        # Memory word with both sign and magnitude bits set.
        sim._memory[100] = make_word(1, 7)  # noqa: SLF001
        sim.step()
        # After CAL: AC sign=0, mag includes the original sign bit at bit 35.
        assert sim._ac_sign == 0  # noqa: SLF001
        # The high bit of the original word ends up in Q.
        assert sim._ac_q == 1  # noqa: SLF001
        assert sim._ac_magnitude == 7  # noqa: SLF001

    def test_sto_stores_ac(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_STO, 0, 200))
        sim._ac_sign = 1  # noqa: SLF001
        sim._ac_magnitude = 12345  # noqa: SLF001
        sim.step()
        assert sim._memory[200] == make_word(1, 12345)  # noqa: SLF001

    def test_stz_stores_zero(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_STZ, 0, 300))
        sim._memory[300] = 0xDEADBEEF  # noqa: SLF001
        sim.step()
        assert sim._memory[300] == 0  # noqa: SLF001

    def test_stq_stores_mq(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_STQ, 0, 400))
        sim._mq = make_word(1, 999)  # noqa: SLF001
        sim.step()
        assert sim._memory[400] == make_word(1, 999)  # noqa: SLF001

    def test_ldq_loads_mq(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_LDQ, 0, 500))
        sim._memory[500] = make_word(0, 777)  # noqa: SLF001
        sim.step()
        assert sim._mq == make_word(0, 777)  # noqa: SLF001

    def test_xca_swaps_ac_and_mq(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_XCA))
        sim._ac_sign = 0  # noqa: SLF001
        sim._ac_magnitude = 5  # noqa: SLF001
        sim._mq = make_word(1, 9)  # noqa: SLF001
        sim.step()
        # AC was +5, MQ was -9 → AC becomes -9, MQ becomes +5
        assert sim._ac_sign == 1  # noqa: SLF001
        assert sim._ac_magnitude == 9  # noqa: SLF001
        assert sim._mq == make_word(0, 5)  # noqa: SLF001


# ---------------------------------------------------------------------------
# Integer arithmetic
# ---------------------------------------------------------------------------


class TestArithmetic:
    def test_add_positive_sum(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_ADD, 0, 100))
        sim._ac_magnitude = 3  # noqa: SLF001
        sim._memory[100] = make_word(0, 4)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 0  # noqa: SLF001
        assert sim._ac_magnitude == 7  # noqa: SLF001
        assert sim._overflow_trigger is False  # noqa: SLF001

    def test_add_overflow_sets_trigger(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_ADD, 0, 100))
        sim._ac_magnitude = (1 << 35) - 1  # max 35-bit magnitude  # noqa: SLF001
        sim._memory[100] = make_word(0, 1)  # noqa: SLF001
        sim.step()
        assert sim._overflow_trigger is True  # noqa: SLF001
        assert sim._ac_p == 1  # noqa: SLF001

    def test_sub(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_SUB, 0, 100))
        sim._ac_magnitude = 10  # noqa: SLF001
        sim._memory[100] = make_word(0, 3)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 0  # noqa: SLF001
        assert sim._ac_magnitude == 7  # noqa: SLF001

    def test_sub_negative_result(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_SUB, 0, 100))
        sim._ac_magnitude = 3  # noqa: SLF001
        sim._memory[100] = make_word(0, 10)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 1  # noqa: SLF001
        assert sim._ac_magnitude == 7  # noqa: SLF001

    def test_adm_treats_operand_as_positive(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_ADM, 0, 100))
        sim._ac_magnitude = 3  # noqa: SLF001
        # ADM ignores the operand sign — adds magnitude regardless.
        sim._memory[100] = make_word(1, 4)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 0  # noqa: SLF001
        assert sim._ac_magnitude == 7  # noqa: SLF001

    def test_mpy(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_MPY, 0, 100))
        sim._mq = make_word(0, 6)  # noqa: SLF001
        sim._memory[100] = make_word(0, 7)  # noqa: SLF001
        sim.step()
        # Product is 42; it fits in MQ low 35 bits, so AC = 0, MQ = 42.
        assert sim._ac_magnitude == 0  # noqa: SLF001
        assert sim._mq == make_word(0, 42)  # noqa: SLF001

    def test_mpy_negative_signs_xor(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_MPY, 0, 100))
        sim._mq = make_word(1, 6)  # noqa: SLF001
        sim._memory[100] = make_word(0, 7)  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 1  # noqa: SLF001
        assert sim._mq == make_word(1, 42)  # noqa: SLF001

    def test_dvp_simple(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_DVP, 0, 100))
        # 7 / 3 = 2 remainder 1 (with dividend small enough to fit in MQ alone)
        sim._ac_magnitude = 0  # noqa: SLF001
        sim._mq = make_word(0, 7)  # noqa: SLF001
        sim._memory[100] = make_word(0, 3)  # noqa: SLF001
        sim.step()
        assert sim._mq == make_word(0, 2)  # noqa: SLF001 — quotient
        assert sim._ac_magnitude == 1  # noqa: SLF001 — remainder

    def test_dvp_divide_check_proceeds(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_DVP, 0, 100))
        sim._ac_magnitude = 0  # noqa: SLF001
        sim._mq = 0  # noqa: SLF001
        sim._memory[100] = make_word(0, 0)  # divide by zero  # noqa: SLF001
        sim.step()
        assert sim._divide_check_trigger is True  # noqa: SLF001
        assert sim._halted is False  # noqa: SLF001

    def test_dvh_divide_check_halts(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_DVH, 0, 100))
        sim._memory[100] = 0  # divide by zero  # noqa: SLF001
        sim.step()
        assert sim._divide_check_trigger is True  # noqa: SLF001
        assert sim._halted is True  # noqa: SLF001


# ---------------------------------------------------------------------------
# Transfers
# ---------------------------------------------------------------------------


class TestTransfers:
    def test_tra_unconditional(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TRA, 0, 200))
        sim.step()
        assert sim._pc == 200  # noqa: SLF001

    def test_tze_taken_when_ac_zero(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TZE, 0, 200))
        sim._ac_magnitude = 0  # noqa: SLF001
        sim.step()
        assert sim._pc == 200  # noqa: SLF001

    def test_tze_not_taken_when_ac_nonzero(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TZE, 0, 200))
        sim._ac_magnitude = 1  # noqa: SLF001
        sim.step()
        assert sim._pc == 1  # noqa: SLF001

    def test_tze_taken_for_negative_zero(self) -> None:
        # AC = -0 → TZE still treats it as zero (magnitude is 0).
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TZE, 0, 200))
        sim._ac_sign = 1  # noqa: SLF001
        sim._ac_magnitude = 0  # noqa: SLF001
        sim.step()
        assert sim._pc == 200  # noqa: SLF001

    def test_tnz_inverse(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TNZ, 0, 50))
        sim._ac_magnitude = 1  # noqa: SLF001
        sim.step()
        assert sim._pc == 50  # noqa: SLF001

    def test_tpl_taken_when_positive(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TPL, 0, 60))
        sim._ac_sign = 0  # noqa: SLF001
        sim.step()
        assert sim._pc == 60  # noqa: SLF001

    def test_tmi_taken_when_negative(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TMI, 0, 70))
        sim._ac_sign = 1  # noqa: SLF001
        sim.step()
        assert sim._pc == 70  # noqa: SLF001

    def test_tov_taken_clears_trigger(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TOV, 0, 80))
        sim._overflow_trigger = True  # noqa: SLF001
        sim.step()
        assert sim._pc == 80  # noqa: SLF001
        assert sim._overflow_trigger is False  # noqa: SLF001

    def test_tno_taken_when_no_overflow(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TNO, 0, 90))
        sim._overflow_trigger = False  # noqa: SLF001
        sim.step()
        assert sim._pc == 90  # noqa: SLF001

    def test_tqo_taken_when_mq_overflow(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TQO, 0, 110))
        sim._mq_overflow = True  # noqa: SLF001
        sim.step()
        assert sim._pc == 110  # noqa: SLF001
        assert sim._mq_overflow is False  # noqa: SLF001

    def test_tqp_taken_when_mq_positive(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_TQP, 0, 120))
        sim._mq = make_word(0, 5)  # noqa: SLF001
        sim.step()
        assert sim._pc == 120  # noqa: SLF001


# ---------------------------------------------------------------------------
# Index registers
# ---------------------------------------------------------------------------


class TestIndexRegisters:
    def test_lxa_loads_address_field(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_LXA, 1, 100))  # tag = 1 → IRA
        # Memory word: address field = 1234, decrement = 5678 (irrelevant).
        sim._memory[100] = (5678 << 18) | 1234  # noqa: SLF001
        sim.step()
        assert sim._index_a == 1234  # noqa: SLF001

    def test_lxd_loads_decrement_field(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_LXD, 2, 100))  # tag = 2 → IRB
        sim._memory[100] = (5678 << 18) | 1234  # noqa: SLF001
        sim.step()
        assert sim._index_b == 5678  # noqa: SLF001

    def test_sxa_writes_address_field(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_SXA, 1, 200))
        sim._index_a = 9999  # noqa: SLF001
        # Pre-fill memory so we can verify the decrement field is preserved.
        sim._memory[200] = (1111 << 18) | 0  # noqa: SLF001
        sim.step()
        word = sim._memory[200]  # noqa: SLF001
        assert word & 0x7FFF == 9999  # address field
        assert (word >> 18) & 0x7FFF == 1111  # decrement preserved

    def test_sxd_writes_decrement_field(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_SXD, 4, 200))  # tag = 4 → IRC
        sim._index_c = 333  # noqa: SLF001
        sim._memory[200] = 444  # address field set; decrement empty  # noqa: SLF001
        sim.step()
        word = sim._memory[200]  # noqa: SLF001
        assert word & 0x7FFF == 444  # address preserved
        assert (word >> 18) & 0x7FFF == 333

    def test_pax_takes_address_from_ac(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_PAX, 1, 0))
        # AC's address bits = 1234; we set them through the magnitude.
        sim._ac_magnitude = 1234  # noqa: SLF001
        sim.step()
        assert sim._index_a == 1234  # noqa: SLF001

    def test_pdx_takes_decrement_from_ac(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_PDX, 2, 0))
        # AC magnitude bits 18-32 = decrement field. Put 4321 there.
        sim._ac_magnitude = 4321 << 18  # noqa: SLF001
        sim.step()
        assert sim._index_b == 4321  # noqa: SLF001

    def test_pxa_places_index_into_ac(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_PXA, 4, 0))
        sim._index_c = 999  # noqa: SLF001
        sim.step()
        assert sim._ac_sign == 0  # noqa: SLF001
        assert sim._ac_magnitude == 999  # noqa: SLF001


# ---------------------------------------------------------------------------
# Effective address with indexing
# ---------------------------------------------------------------------------


class TestEffectiveAddress:
    def test_indexing_subtracts_index_from_address(self) -> None:
        sim = IBM704Simulator()
        # CLA Y=20, tag=1 → eff_addr = 20 - IRA
        _load(sim, encode_type_b(OP_CLA, 1, 20))
        sim._index_a = 5  # noqa: SLF001
        sim._memory[15] = make_word(0, 777)  # noqa: SLF001
        sim.step()
        assert sim._ac_magnitude == 777  # noqa: SLF001

    def test_indexing_with_no_tag_uses_raw_address(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_CLA, 0, 20))
        sim._index_a = 5  # noqa: SLF001
        sim._memory[20] = make_word(0, 100)  # noqa: SLF001
        sim.step()
        assert sim._ac_magnitude == 100  # noqa: SLF001

    def test_indexing_underflow_wraps(self) -> None:
        # Y = 5, IRA = 10 → eff_addr = (5 - 10) & 0x7FFF = 0x7FFB
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_CLA, 1, 5))
        sim._index_a = 10  # noqa: SLF001
        sim._memory[0x7FFB] = make_word(0, 42)  # noqa: SLF001
        sim.step()
        assert sim._ac_magnitude == 42  # noqa: SLF001

    def test_multiple_tag_bits_or_index_registers(self) -> None:
        # tag = 3 (binary 011) → OR of IRA and IRB.
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_CLA, 3, 20))
        sim._index_a = 0b0101  # noqa: SLF001
        sim._index_b = 0b0011  # noqa: SLF001
        # OR = 0b0111 = 7. eff_addr = 20 - 7 = 13.
        sim._memory[13] = make_word(0, 999)  # noqa: SLF001
        sim.step()
        assert sim._ac_magnitude == 999  # noqa: SLF001


# ---------------------------------------------------------------------------
# Type A index transfers
# ---------------------------------------------------------------------------


class TestTypeAIndex:
    def test_txi_always_transfers_and_increments(self) -> None:
        sim = IBM704Simulator()
        # IR1 += 5; PC = 100
        _load(sim, encode_type_a(PREFIX_TXI, decrement=5, tag=1, address=100))
        sim._index_a = 10  # noqa: SLF001
        sim.step()
        assert sim._pc == 100  # noqa: SLF001
        assert sim._index_a == 15  # noqa: SLF001

    def test_tix_transfers_when_ir_greater(self) -> None:
        sim = IBM704Simulator()
        _load(
            sim,
            encode_type_a(PREFIX_TIX, decrement=3, tag=1, address=200),
        )
        sim._index_a = 10  # noqa: SLF001
        sim.step()
        # 10 > 3 → IRA -= 3, PC = 200
        assert sim._pc == 200  # noqa: SLF001
        assert sim._index_a == 7  # noqa: SLF001

    def test_tix_falls_through_when_ir_not_greater(self) -> None:
        sim = IBM704Simulator()
        _load(
            sim,
            encode_type_a(PREFIX_TIX, decrement=10, tag=1, address=200),
        )
        sim._index_a = 5  # noqa: SLF001
        sim.step()
        # 5 not > 10 → fall through; IRA unchanged.
        assert sim._pc == 1  # noqa: SLF001
        assert sim._index_a == 5  # noqa: SLF001

    def test_txh_transfers_when_ir_greater(self) -> None:
        sim = IBM704Simulator()
        _load(
            sim,
            encode_type_a(PREFIX_TXH, decrement=3, tag=2, address=300),
        )
        sim._index_b = 10  # noqa: SLF001
        sim.step()
        assert sim._pc == 300  # noqa: SLF001
        # TXH does NOT modify the index register.
        assert sim._index_b == 10  # noqa: SLF001

    def test_txl_transfers_when_ir_le(self) -> None:
        sim = IBM704Simulator()
        _load(
            sim,
            encode_type_a(PREFIX_TXL, decrement=10, tag=4, address=400),
        )
        sim._index_c = 5  # noqa: SLF001
        sim.step()
        assert sim._pc == 400  # noqa: SLF001
        assert sim._index_c == 5  # noqa: SLF001


# ---------------------------------------------------------------------------
# Floating-point
# ---------------------------------------------------------------------------


class TestFloatingPoint:
    def test_fad_basic(self) -> None:
        from ibm704_simulator.word import float_to_fp, fp_to_float
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_FAD, 0, 100))
        # AC = 1.5 (FP), M = 2.5 (FP) → result 4.0
        ac_fp = float_to_fp(1.5)
        sim._ac_sign = (ac_fp >> 35) & 1  # noqa: SLF001
        sim._ac_magnitude = ac_fp & ((1 << 35) - 1)  # noqa: SLF001
        sim._memory[100] = float_to_fp(2.5)  # noqa: SLF001
        sim.step()
        # Read the AC back as FP.
        from ibm704_simulator.word import make_word as _mw
        ac_word = _mw(sim._ac_sign, sim._ac_magnitude)  # noqa: SLF001
        assert fp_to_float(ac_word) == 4.0

    def test_fsb_basic(self) -> None:
        from ibm704_simulator.word import float_to_fp, fp_to_float, make_word
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_FSB, 0, 100))
        ac_fp = float_to_fp(5.0)
        sim._ac_sign = (ac_fp >> 35) & 1  # noqa: SLF001
        sim._ac_magnitude = ac_fp & ((1 << 35) - 1)  # noqa: SLF001
        sim._memory[100] = float_to_fp(2.0)  # noqa: SLF001
        sim.step()
        ac_word = make_word(sim._ac_sign, sim._ac_magnitude)  # noqa: SLF001
        assert fp_to_float(ac_word) == 3.0

    def test_fmp_basic(self) -> None:
        from ibm704_simulator.word import float_to_fp, fp_to_float, make_word
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_FMP, 0, 100))
        sim._mq = float_to_fp(3.0)  # noqa: SLF001
        sim._memory[100] = float_to_fp(4.0)  # noqa: SLF001
        sim.step()
        ac_word = make_word(sim._ac_sign, sim._ac_magnitude)  # noqa: SLF001
        assert fp_to_float(ac_word) == 12.0

    def test_fdp_basic(self) -> None:
        from ibm704_simulator.word import float_to_fp, fp_to_float
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_FDP, 0, 100))
        ac_fp = float_to_fp(10.0)
        sim._ac_sign = (ac_fp >> 35) & 1  # noqa: SLF001
        sim._ac_magnitude = ac_fp & ((1 << 35) - 1)  # noqa: SLF001
        sim._memory[100] = float_to_fp(2.0)  # noqa: SLF001
        sim.step()
        # Quotient lands in MQ.
        assert fp_to_float(sim._mq) == 5.0  # noqa: SLF001

    def test_fdp_divide_check_on_zero(self) -> None:
        sim = IBM704Simulator()
        _load(sim, encode_type_b(OP_FDP, 0, 100))
        sim._memory[100] = 0  # FP zero  # noqa: SLF001
        sim.step()
        assert sim._divide_check_trigger is True  # noqa: SLF001


# ---------------------------------------------------------------------------
# Encoder validation
# ---------------------------------------------------------------------------


class TestEncoders:
    def test_encode_type_b_rejects_oversize_opcode(self) -> None:
        with pytest.raises(ValueError):
            encode_type_b(0x1000)

    def test_encode_type_b_rejects_oversize_tag(self) -> None:
        with pytest.raises(ValueError):
            encode_type_b(OP_NOP, tag=8)

    def test_encode_type_b_rejects_oversize_address(self) -> None:
        with pytest.raises(ValueError):
            encode_type_b(OP_NOP, address=0x8000)

    def test_encode_type_a_rejects_invalid_prefix(self) -> None:
        with pytest.raises(ValueError):
            encode_type_a(0)
        with pytest.raises(ValueError):
            encode_type_a(4)

    def test_encode_type_a_rejects_oversize_decrement(self) -> None:
        with pytest.raises(ValueError):
            encode_type_a(PREFIX_TXI, decrement=0x8000)


# ---------------------------------------------------------------------------
# Unknown opcode
# ---------------------------------------------------------------------------


class TestUnknownOpcode:
    def test_unknown_opcode_step_raises(self) -> None:
        sim = IBM704Simulator()
        # 0x0F0 has top 3 bits 0b000 (Type B range) and is not in v1 dispatch.
        _load(sim, encode_type_b(0x0F0))
        with pytest.raises(RuntimeError, match="unknown opcode"):
            sim.step()

    def test_unknown_opcode_in_execute_records_error(self) -> None:
        sim = IBM704Simulator()
        from ibm704_simulator.word import pack_program
        result = sim.execute(pack_program([encode_type_b(0x0F0)]))
        assert result.ok is False
        assert result.error is not None
        assert "unknown opcode" in result.error
        assert result.halted is True


# ---------------------------------------------------------------------------
# Memory size validation
# ---------------------------------------------------------------------------


class TestMemoryValidation:
    def test_invalid_memory_size_raises(self) -> None:
        with pytest.raises(ValueError):
            IBM704Simulator(memory_words=0)
        with pytest.raises(ValueError):
            IBM704Simulator(memory_words=33000)

    def test_program_too_large_raises(self) -> None:
        sim = IBM704Simulator(memory_words=4)
        from ibm704_simulator.word import pack_program
        with pytest.raises(ValueError):
            sim.load(pack_program([0] * 5))
