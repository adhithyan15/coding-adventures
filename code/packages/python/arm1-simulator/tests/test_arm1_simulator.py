"""
Tests for the ARM1 behavioral simulator.
=========================================================================

These tests mirror the Go test suite and validate every component:
condition evaluator, barrel shifter, ALU, instruction decoder, and
full-program execution including load/store, block transfer, branch,
SWI, and register banking.
"""

from __future__ import annotations

import struct

from arm1_simulator import (
    ARM1,
    COND_AL,
    COND_CC,
    COND_CS,
    COND_EQ,
    COND_GE,
    COND_GT,
    COND_HI,
    COND_LE,
    COND_LS,
    COND_LT,
    COND_MI,
    COND_NE,
    COND_NV,
    COND_PL,
    COND_VC,
    COND_VS,
    Flags,
    HALT_SWI,
    INST_DATA_PROCESSING,
    INST_SWI,
    MASK_32,
    MODE_MASK,
    MODE_SVC,
    MODE_USR,
    OP_ADD,
    OP_ADC,
    OP_AND,
    OP_BIC,
    OP_CMP,
    OP_CMN,
    OP_EOR,
    OP_MOV,
    OP_MVN,
    OP_ORR,
    OP_RSB,
    OP_RSC,
    OP_SBC,
    OP_SUB,
    OP_TEQ,
    OP_TST,
    SHIFT_ASR,
    SHIFT_LSL,
    SHIFT_LSR,
    SHIFT_ROR,
    alu_execute,
    barrel_shift,
    cond_string,
    decode,
    decode_immediate,
    disassemble,
    encode_alu_reg,
    encode_branch,
    encode_data_processing,
    encode_halt,
    encode_ldr,
    encode_ldm,
    encode_mov_imm,
    encode_str,
    encode_stm,
    evaluate_condition,
    is_logical_op,
    is_test_op,
    mode_string,
    op_string,
    shift_string,
)


# =========================================================================
# Helper: load a program from uint32 instruction words
# =========================================================================


def load_program(cpu: ARM1, instructions: list[int]) -> None:
    """Convert a list of 32-bit instruction words to bytes and load them."""
    code = b"".join(struct.pack("<I", inst & MASK_32) for inst in instructions)
    cpu.load_program(code, 0)


# =========================================================================
# Types and Constants
# =========================================================================


class TestModeString:
    def test_all_modes(self) -> None:
        assert mode_string(0) == "USR"
        assert mode_string(1) == "FIQ"
        assert mode_string(2) == "IRQ"
        assert mode_string(3) == "SVC"
        assert mode_string(99) == "???"


class TestOpString:
    def test_known_ops(self) -> None:
        assert op_string(OP_ADD) == "ADD"
        assert op_string(OP_MOV) == "MOV"
        assert op_string(99) == "???"


class TestIsTestOp:
    def test_tst_is_test(self) -> None:
        assert is_test_op(OP_TST)

    def test_cmp_is_test(self) -> None:
        assert is_test_op(OP_CMP)

    def test_add_is_not_test(self) -> None:
        assert not is_test_op(OP_ADD)


class TestIsLogicalOp:
    def test_and_is_logical(self) -> None:
        assert is_logical_op(OP_AND)

    def test_mov_is_logical(self) -> None:
        assert is_logical_op(OP_MOV)

    def test_add_is_not_logical(self) -> None:
        assert not is_logical_op(OP_ADD)


class TestShiftString:
    def test_shift_names(self) -> None:
        assert shift_string(SHIFT_LSL) == "LSL"
        assert shift_string(SHIFT_LSR) == "LSR"
        assert shift_string(SHIFT_ASR) == "ASR"
        assert shift_string(SHIFT_ROR) == "ROR"
        assert shift_string(99) == "???"


class TestCondString:
    def test_cond_strings(self) -> None:
        assert cond_string(COND_EQ) == "EQ"
        assert cond_string(COND_AL) == ""
        assert cond_string(COND_NV) == "NV"


# =========================================================================
# Condition Evaluator
# =========================================================================


class TestEvaluateCondition:
    """Test all 16 condition codes against various flag combinations."""

    def test_eq_z_set(self) -> None:
        assert evaluate_condition(COND_EQ, Flags(z=True)) is True

    def test_eq_z_clear(self) -> None:
        assert evaluate_condition(COND_EQ, Flags()) is False

    def test_ne_z_clear(self) -> None:
        assert evaluate_condition(COND_NE, Flags()) is True

    def test_ne_z_set(self) -> None:
        assert evaluate_condition(COND_NE, Flags(z=True)) is False

    def test_cs_c_set(self) -> None:
        assert evaluate_condition(COND_CS, Flags(c=True)) is True

    def test_cc_c_clear(self) -> None:
        assert evaluate_condition(COND_CC, Flags()) is True

    def test_mi_n_set(self) -> None:
        assert evaluate_condition(COND_MI, Flags(n=True)) is True

    def test_pl_n_clear(self) -> None:
        assert evaluate_condition(COND_PL, Flags()) is True

    def test_vs_v_set(self) -> None:
        assert evaluate_condition(COND_VS, Flags(v=True)) is True

    def test_vc_v_clear(self) -> None:
        assert evaluate_condition(COND_VC, Flags()) is True

    def test_hi_c1_z0(self) -> None:
        assert evaluate_condition(COND_HI, Flags(c=True)) is True

    def test_hi_c1_z1(self) -> None:
        assert evaluate_condition(COND_HI, Flags(c=True, z=True)) is False

    def test_ls_c0(self) -> None:
        assert evaluate_condition(COND_LS, Flags()) is True

    def test_ls_z1(self) -> None:
        assert evaluate_condition(COND_LS, Flags(c=True, z=True)) is True

    def test_ge_n_eq_v_both0(self) -> None:
        assert evaluate_condition(COND_GE, Flags()) is True

    def test_ge_n_eq_v_both1(self) -> None:
        assert evaluate_condition(COND_GE, Flags(n=True, v=True)) is True

    def test_ge_n_ne_v(self) -> None:
        assert evaluate_condition(COND_GE, Flags(n=True)) is False

    def test_lt_n_ne_v(self) -> None:
        assert evaluate_condition(COND_LT, Flags(n=True)) is True

    def test_lt_n_eq_v(self) -> None:
        assert evaluate_condition(COND_LT, Flags()) is False

    def test_gt_z0_n_eq_v(self) -> None:
        assert evaluate_condition(COND_GT, Flags()) is True

    def test_gt_z1(self) -> None:
        assert evaluate_condition(COND_GT, Flags(z=True)) is False

    def test_le_z1(self) -> None:
        assert evaluate_condition(COND_LE, Flags(z=True)) is True

    def test_le_n_ne_v(self) -> None:
        assert evaluate_condition(COND_LE, Flags(n=True)) is True

    def test_al_always(self) -> None:
        assert evaluate_condition(COND_AL, Flags()) is True

    def test_nv_never(self) -> None:
        assert evaluate_condition(COND_NV, Flags()) is False


# =========================================================================
# Barrel Shifter
# =========================================================================


class TestBarrelShiftLSL:
    def test_lsl_0_no_shift(self) -> None:
        val, c = barrel_shift(0xFF, SHIFT_LSL, 0, False, False)
        assert val == 0xFF
        assert c is False

    def test_lsl_1(self) -> None:
        val, c = barrel_shift(0xFF, SHIFT_LSL, 1, False, False)
        assert val == 0x1FE
        assert c is False

    def test_lsl_4(self) -> None:
        val, c = barrel_shift(0xFF, SHIFT_LSL, 4, False, False)
        assert val == 0xFF0
        assert c is False

    def test_lsl_31(self) -> None:
        val, c = barrel_shift(1, SHIFT_LSL, 31, False, False)
        assert val == 0x80000000
        assert c is False

    def test_lsl_32(self) -> None:
        val, c = barrel_shift(1, SHIFT_LSL, 32, False, False)
        assert val == 0
        assert c is True

    def test_lsl_33(self) -> None:
        val, c = barrel_shift(1, SHIFT_LSL, 33, False, False)
        assert val == 0
        assert c is False


class TestBarrelShiftLSR:
    def test_lsr_1(self) -> None:
        val, c = barrel_shift(0xFF, SHIFT_LSR, 1, False, False)
        assert val == 0x7F
        assert c is True

    def test_lsr_8(self) -> None:
        val, c = barrel_shift(0xFF00, SHIFT_LSR, 8, False, False)
        assert val == 0xFF
        assert c is False

    def test_lsr_0_encodes_32(self) -> None:
        val, c = barrel_shift(0x80000000, SHIFT_LSR, 0, False, False)
        assert val == 0
        assert c is True

    def test_lsr_32_by_register(self) -> None:
        val, c = barrel_shift(0x80000000, SHIFT_LSR, 32, False, True)
        assert val == 0
        assert c is True


class TestBarrelShiftASR:
    def test_asr_1_positive(self) -> None:
        val, c = barrel_shift(0x7FFFFFFE, SHIFT_ASR, 1, False, False)
        assert val == 0x3FFFFFFF
        assert c is False

    def test_asr_1_negative(self) -> None:
        val, c = barrel_shift(0x80000000, SHIFT_ASR, 1, False, False)
        assert val == 0xC0000000
        assert c is False

    def test_asr_0_encodes_32_negative(self) -> None:
        val, c = barrel_shift(0x80000000, SHIFT_ASR, 0, False, False)
        assert val == 0xFFFFFFFF
        assert c is True

    def test_asr_0_encodes_32_positive(self) -> None:
        val, c = barrel_shift(0x7FFFFFFF, SHIFT_ASR, 0, False, False)
        assert val == 0
        assert c is False


class TestBarrelShiftROR:
    def test_ror_4(self) -> None:
        val, c = barrel_shift(0x0000000F, SHIFT_ROR, 4, False, False)
        assert val == 0xF0000000
        assert c is True

    def test_ror_8(self) -> None:
        val, c = barrel_shift(0x000000FF, SHIFT_ROR, 8, False, False)
        assert val == 0xFF000000
        assert c is True

    def test_ror_16(self) -> None:
        val, c = barrel_shift(0x0000FFFF, SHIFT_ROR, 16, False, False)
        assert val == 0xFFFF0000
        assert c is True


class TestBarrelShiftRRX:
    def test_rrx_bit0_set_carry_in(self) -> None:
        val, c = barrel_shift(0x00000001, SHIFT_ROR, 0, True, False)
        assert val == 0x80000000
        assert c is True

    def test_rrx_bit0_clear_carry_in(self) -> None:
        val, c = barrel_shift(0x00000000, SHIFT_ROR, 0, True, False)
        assert val == 0x80000000
        assert c is False


class TestDecodeImmediate:
    def test_no_rotation(self) -> None:
        val, _ = decode_immediate(0xFF, 0)
        assert val == 0xFF

    def test_ror_2(self) -> None:
        val, _ = decode_immediate(0x01, 1)
        assert val == 0x40000000

    def test_ror_8(self) -> None:
        val, _ = decode_immediate(0xFF, 4)
        assert val == 0xFF000000


# =========================================================================
# ALU
# =========================================================================


class TestALUAdd:
    def test_basic_add(self) -> None:
        r = alu_execute(OP_ADD, 1, 2, False, False, False)
        assert r.result == 3
        assert not r.n and not r.z and not r.c and not r.v

    def test_signed_overflow(self) -> None:
        r = alu_execute(OP_ADD, 0x7FFFFFFF, 1, False, False, False)
        assert r.result == 0x80000000
        assert r.n is True
        assert r.v is True

    def test_unsigned_overflow(self) -> None:
        r = alu_execute(OP_ADD, 0xFFFFFFFF, 1, False, False, False)
        assert r.result == 0
        assert r.c is True
        assert r.z is True


class TestALUSub:
    def test_basic_sub(self) -> None:
        r = alu_execute(OP_SUB, 5, 3, False, False, False)
        assert r.result == 2
        assert r.c is True

    def test_sub_with_borrow(self) -> None:
        r = alu_execute(OP_SUB, 3, 5, False, False, False)
        assert r.result == 0xFFFFFFFE
        assert r.c is False
        assert r.n is True


class TestALURSB:
    def test_rsb(self) -> None:
        r = alu_execute(OP_RSB, 3, 5, False, False, False)
        assert r.result == 2


class TestALUADC:
    def test_adc(self) -> None:
        r = alu_execute(OP_ADC, 1, 2, True, False, False)
        assert r.result == 4


class TestALUSBC:
    def test_sbc(self) -> None:
        r = alu_execute(OP_SBC, 5, 3, True, False, False)
        assert r.result == 2


class TestALULogical:
    def test_and(self) -> None:
        r = alu_execute(OP_AND, 0xFF00FF00, 0x0FF00FF0, False, False, False)
        assert r.result == 0x0F000F00

    def test_eor(self) -> None:
        r = alu_execute(OP_EOR, 0xFF00FF00, 0x0FF00FF0, False, False, False)
        assert r.result == 0xF0F0F0F0

    def test_orr(self) -> None:
        r = alu_execute(OP_ORR, 0xFF00FF00, 0x0FF00FF0, False, False, False)
        assert r.result == 0xFFF0FFF0

    def test_bic(self) -> None:
        r = alu_execute(OP_BIC, 0xFFFFFFFF, 0x0000FF00, False, False, False)
        assert r.result == 0xFFFF00FF

    def test_mov(self) -> None:
        r = alu_execute(OP_MOV, 0, 42, False, False, False)
        assert r.result == 42

    def test_mvn(self) -> None:
        r = alu_execute(OP_MVN, 0, 0, False, False, False)
        assert r.result == 0xFFFFFFFF


class TestALUTestOps:
    def test_tst_sets_z(self) -> None:
        r = alu_execute(OP_TST, 0xFF, 0x00, False, False, False)
        assert r.write_result is False
        assert r.z is True

    def test_cmp_equal(self) -> None:
        r = alu_execute(OP_CMP, 5, 5, False, False, False)
        assert r.write_result is False
        assert r.z is True
        assert r.c is True


# =========================================================================
# Decoder
# =========================================================================


class TestDecodeDataProcessing:
    def test_add_r2_r0_r1(self) -> None:
        d = decode(0xE0802001)
        assert d.inst_type == INST_DATA_PROCESSING
        assert d.cond == COND_AL
        assert d.opcode == OP_ADD
        assert d.s is False
        assert d.rn == 0
        assert d.rd == 2
        assert d.rm == 1


class TestDecodeMovImmediate:
    def test_mov_r0_42(self) -> None:
        d = decode(0xE3A0002A)
        assert d.inst_type == INST_DATA_PROCESSING
        assert d.opcode == OP_MOV
        assert d.immediate is True
        assert d.rd == 0
        assert d.imm8 == 42


class TestDecodeBranch:
    def test_branch_forward(self) -> None:
        d = decode(0xEA000002)
        assert d.inst_type == 3  # INST_BRANCH
        assert d.link is False
        assert d.branch_offset == 8

    def test_branch_link_backward(self) -> None:
        d = decode(0xEBFFFFFE)
        assert d.inst_type == 3  # INST_BRANCH
        assert d.link is True
        assert d.branch_offset == -8


class TestDecodeSWI:
    def test_swi(self) -> None:
        d = decode(0xEF123456)
        assert d.inst_type == INST_SWI
        assert d.swi_comment == 0x123456


class TestDisassemble:
    def test_mov_r0_42(self) -> None:
        assert disassemble(decode(0xE3A0002A)) == "MOV R0, #42"

    def test_add_r2_r0_r1(self) -> None:
        assert disassemble(decode(0xE0802001)) == "ADD R2, R0, R1"

    def test_adds_r2_r1_r1(self) -> None:
        assert disassemble(decode(0xE0912001)) == "ADDS R2, R1, R1"

    def test_addne(self) -> None:
        assert disassemble(decode(0x10802001)) == "ADDNE R2, R0, R1"

    def test_halt(self) -> None:
        assert disassemble(decode(0xEF123456)) == "HLT"


# =========================================================================
# CPU — Power-on state
# =========================================================================


class TestNewCPU:
    def test_initial_state(self) -> None:
        cpu = ARM1(1024)
        assert cpu.mode == MODE_SVC
        assert cpu.pc == 0
        f = cpu.flags
        assert not f.n and not f.z and not f.c and not f.v


# =========================================================================
# CPU — Basic programs
# =========================================================================


class TestMOVImmediate:
    def test_mov_r0_42(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [encode_mov_imm(COND_AL, 0, 42), encode_halt()])
        cpu.run(10)
        assert cpu.read_register(0) == 42


class TestOnePlusTwo:
    def test_one_plus_two(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 1),
            encode_mov_imm(COND_AL, 1, 2),
            encode_alu_reg(COND_AL, OP_ADD, 0, 2, 0, 1),
            encode_halt(),
        ])
        cpu.run(10)
        assert cpu.read_register(0) == 1
        assert cpu.read_register(1) == 2
        assert cpu.read_register(2) == 3


class TestSUBSWithFlags:
    def test_subs_sets_flags(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_halt(),
        ])
        cpu.run(10)
        assert cpu.read_register(2) == 0
        assert cpu.flags.z is True
        assert cpu.flags.c is True


class TestConditionalExecution:
    def test_conditional(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_mov_imm(COND_NE, 3, 99),
            encode_mov_imm(COND_EQ, 4, 42),
            encode_halt(),
        ])
        cpu.run(20)
        assert cpu.read_register(3) == 0
        assert cpu.read_register(4) == 42


class TestBarrelShifterInInstruction:
    def test_multiply_by_5(self) -> None:
        cpu = ARM1(1024)
        add_with_shift = (
            (COND_AL << 28) | (OP_ADD << 21) | (0 << 16) | (1 << 12)
            | (2 << 7) | (SHIFT_LSL << 5) | 0
        ) & MASK_32
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 7),
            add_with_shift,
            encode_halt(),
        ])
        cpu.run(10)
        assert cpu.read_register(1) == 35


class TestLoopSumOneToTen:
    def test_sum_1_to_10(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 10),
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 1),
            encode_data_processing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
            encode_branch(COND_NE, False, -16),
            encode_halt(),
        ])
        cpu.run(100)
        assert cpu.read_register(0) == 55
        assert cpu.read_register(1) == 0


# =========================================================================
# CPU — Load/Store
# =========================================================================


class TestLDRSTR:
    def test_store_then_load(self) -> None:
        cpu = ARM1(4096)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 42),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
            encode_str(COND_AL, 0, 1, 0, True),
            encode_mov_imm(COND_AL, 0, 0),
            encode_ldr(COND_AL, 0, 1, 0, True),
            encode_halt(),
        ])
        cpu.run(20)
        assert cpu.read_register(0) == 42

    def test_ldrb(self) -> None:
        cpu = ARM1(4096)
        cpu.write_word(0x100, 0xDEADBEEF)
        load_program(cpu, [
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
            (COND_AL << 28) | 0x05D00000 | (1 << 16) | (0 << 12) | 0,
            encode_halt(),
        ])
        cpu.run(10)
        assert cpu.read_register(0) == 0xEF


# =========================================================================
# CPU — Block Transfer (LDM/STM)
# =========================================================================


class TestSTMLDM:
    def test_store_and_load_multiple(self) -> None:
        cpu = ARM1(4096)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 10),
            encode_mov_imm(COND_AL, 1, 20),
            encode_mov_imm(COND_AL, 2, 30),
            encode_mov_imm(COND_AL, 3, 40),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_stm(COND_AL, 5, 0x000F, True, "IA"),
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 0),
            encode_mov_imm(COND_AL, 2, 0),
            encode_mov_imm(COND_AL, 3, 0),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_ldm(COND_AL, 5, 0x000F, True, "IA"),
            encode_halt(),
        ])
        cpu.run(50)
        assert cpu.read_register(0) == 10
        assert cpu.read_register(1) == 20
        assert cpu.read_register(2) == 30
        assert cpu.read_register(3) == 40


# =========================================================================
# CPU — Branch and Link
# =========================================================================


class TestBranchAndLink:
    def test_bl_and_return(self) -> None:
        cpu = ARM1(4096)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 7),
            encode_branch(COND_AL, True, 4),
            encode_halt(),
            0,
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 0),
            encode_data_processing(COND_AL, OP_MOV, 1, 0, 15, 14),
        ])
        cpu.run(20)
        assert cpu.read_register(0) == 14


# =========================================================================
# CPU — Fibonacci
# =========================================================================


class TestFibonacci:
    def test_fib_10(self) -> None:
        cpu = ARM1(4096)
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 1),
            encode_mov_imm(COND_AL, 2, 10),
            encode_alu_reg(COND_AL, OP_ADD, 0, 3, 0, 1),
            encode_alu_reg(COND_AL, OP_MOV, 0, 0, 0, 1),
            encode_alu_reg(COND_AL, OP_MOV, 0, 1, 0, 3),
            encode_data_processing(COND_AL, OP_SUB, 1, 2, 2, (1 << 25) | 1),
            encode_branch(COND_NE, False, -24),
            encode_halt(),
        ])
        cpu.run(200)
        assert cpu.read_register(1) == 89


# =========================================================================
# CPU — Register banking
# =========================================================================


class TestRegisterBanking:
    def test_svc_vs_usr_r13(self) -> None:
        cpu = ARM1(4096)
        cpu.write_register(13, 0xAA000000)
        r15 = cpu._regs[15]
        r15 = (r15 & ~MODE_MASK & MASK_32) | MODE_USR
        cpu._regs[15] = r15
        cpu.write_register(13, 0xBB000000)
        usr_r13 = cpu.read_register(13)
        r15 = cpu._regs[15]
        r15 = (r15 & ~MODE_MASK & MASK_32) | MODE_SVC
        cpu._regs[15] = r15
        svc_r13 = cpu.read_register(13)
        assert usr_r13 != svc_r13
        assert usr_r13 == 0xBB000000
        assert svc_r13 == 0xAA000000


# =========================================================================
# CPU — Memory operations
# =========================================================================


class TestMemoryOperations:
    def test_read_write_word(self) -> None:
        cpu = ARM1(1024)
        cpu.write_word(0, 0xDEADBEEF)
        assert cpu.read_word(0) == 0xDEADBEEF

    def test_read_write_byte(self) -> None:
        cpu = ARM1(1024)
        cpu.write_byte(0, 0xAB)
        assert cpu.read_byte(0) == 0xAB


class TestCPUString:
    def test_string_not_empty(self) -> None:
        cpu = ARM1(1024)
        assert len(str(cpu)) > 0


class TestHalt:
    def test_halt_stops_execution(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [encode_halt()])
        traces = cpu.run(100)
        assert cpu.halted is True
        assert len(traces) == 1


class TestTraceFields:
    def test_trace_captures_state(self) -> None:
        cpu = ARM1(1024)
        load_program(cpu, [encode_mov_imm(COND_AL, 0, 42), encode_halt()])
        trace = cpu.step()
        assert trace.address == 0
        assert trace.condition_met is True
        assert trace.regs_after[0] == 42


# =========================================================================
# CPU — SWI
# =========================================================================


class TestSWI:
    def test_swi_enters_handler(self) -> None:
        cpu = ARM1(4096)
        cpu.write_word(0x08, encode_branch(COND_AL, False, 0xF0 - 0x08 - 8))
        cpu.write_word(0xF0, encode_mov_imm(COND_AL, 5, 99))
        cpu.write_word(0xF4, encode_halt())
        load_program(cpu, [
            encode_mov_imm(COND_AL, 0, 1),
            (COND_AL << 28) | 0x0F000001,
            encode_halt(),
        ])
        cpu.write_word(0x08, encode_branch(COND_AL, False, 0xF0 - 0x08 - 8))
        cpu.run(20)
        assert cpu.read_register(5) == 99
        assert cpu.mode == MODE_SVC


# =========================================================================
# Encoding helpers
# =========================================================================


class TestEncodeHelpers:
    def test_encode_mov_imm(self) -> None:
        inst = encode_mov_imm(COND_AL, 0, 42)
        d = decode(inst)
        assert d.opcode == OP_MOV and d.rd == 0 and d.imm8 == 42

    def test_encode_halt(self) -> None:
        inst = encode_halt()
        d = decode(inst)
        assert d.inst_type == INST_SWI and d.swi_comment == HALT_SWI


# =========================================================================
# Additional coverage
# =========================================================================


class TestAdditionalCoverage:
    def test_cmn(self) -> None:
        r = alu_execute(OP_CMN, 1, 2, False, False, False)
        assert r.result == 3 and r.write_result is False

    def test_teq(self) -> None:
        r = alu_execute(OP_TEQ, 0xFF, 0xFF, False, False, False)
        assert r.z is True and r.write_result is False

    def test_rsc(self) -> None:
        r = alu_execute(OP_RSC, 3, 5, True, False, False)
        assert r.result == 2

    def test_barrel_shift_by_register_0(self) -> None:
        val, c = barrel_shift(0x12345678, SHIFT_LSL, 0, True, True)
        assert val == 0x12345678 and c is True

    def test_decode_coprocessor(self) -> None:
        d = decode((COND_AL << 28) | 0x0C000000)
        assert d.inst_type == 5

    def test_block_transfer_empty_list(self) -> None:
        cpu = ARM1(4096)
        load_program(cpu, [encode_stm(COND_AL, 0, 0x0000, False, "IA"), encode_halt()])
        cpu.run(10)

    def test_read_out_of_bounds(self) -> None:
        cpu = ARM1(64)
        assert cpu.read_word(0x100) == 0
        assert cpu.read_byte(0x100) == 0

    def test_write_out_of_bounds(self) -> None:
        cpu = ARM1(64)
        cpu.write_word(0x100, 0xDEADBEEF)
        cpu.write_byte(0x100, 0xFF)

    def test_memory_property(self) -> None:
        cpu = ARM1(1024)
        assert isinstance(cpu.memory, bytearray)

    def test_reset(self) -> None:
        cpu = ARM1(1024)
        cpu.write_register(0, 42)
        cpu.reset()
        assert cpu.read_register(0) == 0 and cpu.mode == MODE_SVC and cpu.pc == 0

    def test_version(self) -> None:
        from arm1_simulator import __version__
        assert __version__ == "0.1.0"
