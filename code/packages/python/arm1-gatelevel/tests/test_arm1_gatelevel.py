"""
Tests for the ARM1 gate-level simulator.
=========================================================================

These tests validate the gate-level components (ALU, barrel shifter, bit
conversion) and cross-validate against the behavioral simulator to ensure
both produce identical results for any program.
"""

from __future__ import annotations

import struct

from arm1_gatelevel import (
    ARM1GateLevel,
    bits_to_int,
    gate_alu_execute,
    gate_barrel_shift,
    gate_decode_immediate,
    int_to_bits,
)

from arm1_simulator import (
    ARM1 as BehavioralARM1,
    COND_AL,
    COND_EQ,
    COND_NE,
    Flags,
    MASK_32,
    MODE_SVC,
    OP_ADD,
    OP_AND,
    OP_EOR,
    OP_MOV,
    OP_ORR,
    OP_SUB,
    SHIFT_LSL,
    Trace,
    encode_alu_reg,
    encode_branch,
    encode_data_processing,
    encode_halt,
    encode_ldr,
    encode_ldm,
    encode_mov_imm,
    encode_str,
    encode_stm,
)


# =========================================================================
# Helper: load a program from uint32 instruction words
# =========================================================================


def load_gate_program(cpu: ARM1GateLevel, instructions: list[int]) -> None:
    """Load instructions into the gate-level CPU."""
    code = b"".join(struct.pack("<I", inst & MASK_32) for inst in instructions)
    cpu.load_program(code, 0)


def load_behavioral_program(cpu: BehavioralARM1, instructions: list[int]) -> None:
    """Load instructions into the behavioral CPU."""
    code = b"".join(struct.pack("<I", inst & MASK_32) for inst in instructions)
    cpu.load_program(code, 0)


# =========================================================================
# Bit conversion
# =========================================================================


class TestIntToBits:
    def test_basic_conversion(self) -> None:
        bits = int_to_bits(5, 32)
        assert bits[0] == 1  # 5 = ...101
        assert bits[2] == 1
        assert bits_to_int(bits) == 5

    def test_round_trip(self) -> None:
        values = [0, 1, 42, 0xFF, 0xDEADBEEF, 0xFFFFFFFF]
        for v in values:
            bits = int_to_bits(v, 32)
            assert bits_to_int(bits) == v


# =========================================================================
# Gate-level ALU
# =========================================================================


class TestGateALUAdd:
    def test_basic_add(self) -> None:
        a = int_to_bits(1, 32)
        b = int_to_bits(2, 32)
        r = gate_alu_execute(OP_ADD, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 3
        assert r["n"] == 0 and r["z"] == 0 and r["c"] == 0 and r["v"] == 0


class TestGateALUSubZero:
    def test_sub_zero(self) -> None:
        a = int_to_bits(5, 32)
        b = int_to_bits(5, 32)
        r = gate_alu_execute(OP_SUB, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 0
        assert r["z"] == 1
        assert r["c"] == 1  # No borrow


class TestGateALULogical:
    def test_and(self) -> None:
        a = int_to_bits(0xFF00FF00, 32)
        b = int_to_bits(0x0FF00FF0, 32)
        r = gate_alu_execute(OP_AND, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 0x0F000F00

    def test_eor(self) -> None:
        a = int_to_bits(0xFF00FF00, 32)
        b = int_to_bits(0x0FF00FF0, 32)
        r = gate_alu_execute(OP_EOR, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 0xF0F0F0F0

    def test_orr(self) -> None:
        a = int_to_bits(0xFF00FF00, 32)
        b = int_to_bits(0x0FF00FF0, 32)
        r = gate_alu_execute(OP_ORR, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 0xFFF0FFF0


# =========================================================================
# Gate-level barrel shifter
# =========================================================================


class TestGateBarrelShiftLSL:
    def test_lsl_4(self) -> None:
        value = int_to_bits(0xFF, 32)
        result, _ = gate_barrel_shift(value, 0, 4, 0, False)
        assert bits_to_int(result) == 0xFF0


class TestGateBarrelShiftLSR:
    def test_lsr_8(self) -> None:
        value = int_to_bits(0xFF00, 32)
        result, _ = gate_barrel_shift(value, 1, 8, 0, False)
        assert bits_to_int(result) == 0xFF


class TestGateBarrelShiftROR:
    def test_ror_4(self) -> None:
        value = int_to_bits(0x0000000F, 32)
        result, _ = gate_barrel_shift(value, 3, 4, 0, False)
        assert bits_to_int(result) == 0xF0000000


class TestGateBarrelShiftRRX:
    def test_rrx(self) -> None:
        value = int_to_bits(0x00000001, 32)
        result, carry = gate_barrel_shift(value, 3, 0, 1, False)
        assert bits_to_int(result) == 0x80000000
        assert carry == 1


class TestGateDecodeImmediate:
    def test_basic(self) -> None:
        bits, _ = gate_decode_immediate(0xFF, 0)
        assert bits_to_int(bits) == 0xFF

    def test_rotated(self) -> None:
        bits, _ = gate_decode_immediate(0x01, 1)
        assert bits_to_int(bits) == 0x40000000


# =========================================================================
# Cross-validation: Gate-level vs Behavioral
# =========================================================================
#
# This is the ultimate correctness guarantee. We run the same program on
# both simulators and verify they produce identical results.


def cross_validate(name: str, instructions: list[int]) -> None:
    """Run the same program on both simulators and compare results."""
    behavioral = BehavioralARM1(4096)
    gate_lev = ARM1GateLevel(4096)

    load_behavioral_program(behavioral, instructions)
    load_gate_program(gate_lev, instructions)

    b_traces = behavioral.run(200)
    g_traces = gate_lev.run(200)

    assert len(b_traces) == len(g_traces), (
        f"{name}: trace count mismatch: behavioral={len(b_traces)} "
        f"gate-level={len(g_traces)}"
    )

    for i in range(len(b_traces)):
        bt = b_traces[i]
        gt = g_traces[i]

        assert bt.address == gt.address, (
            f"{name} step {i}: address mismatch: B=0x{bt.address:X} G=0x{gt.address:X}"
        )
        assert bt.condition_met == gt.condition_met, (
            f"{name} step {i}: condition mismatch"
        )

        for r in range(16):
            assert bt.regs_after[r] == gt.regs_after[r], (
                f"{name} step {i}: R{r} mismatch: "
                f"B=0x{bt.regs_after[r]:X} G=0x{gt.regs_after[r]:X}"
            )

        assert bt.flags_after == gt.flags_after, (
            f"{name} step {i}: flags mismatch: B={bt.flags_after} G={gt.flags_after}"
        )


class TestCrossValidateOnePlusTwo:
    def test_cross(self) -> None:
        cross_validate("1+2", [
            encode_mov_imm(COND_AL, 0, 1),
            encode_mov_imm(COND_AL, 1, 2),
            encode_alu_reg(COND_AL, OP_ADD, 0, 2, 0, 1),
            encode_halt(),
        ])


class TestCrossValidateSUBSWithFlags:
    def test_cross(self) -> None:
        cross_validate("SUBS", [
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_halt(),
        ])


class TestCrossValidateConditional:
    def test_cross(self) -> None:
        cross_validate("conditional", [
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_mov_imm(COND_NE, 3, 99),
            encode_mov_imm(COND_EQ, 4, 42),
            encode_halt(),
        ])


class TestCrossValidateBarrelShifter:
    def test_cross(self) -> None:
        add_with_shift = (
            (COND_AL << 28)
            | (OP_ADD << 21)
            | (0 << 16)
            | (1 << 12)
            | (2 << 7)
            | (SHIFT_LSL << 5)
            | 0
        ) & MASK_32
        cross_validate("barrel_shifter", [
            encode_mov_imm(COND_AL, 0, 7),
            add_with_shift,
            encode_halt(),
        ])


class TestCrossValidateLoop:
    def test_cross(self) -> None:
        cross_validate("loop_sum_1_to_10", [
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 10),
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 1),
            encode_data_processing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
            encode_branch(COND_NE, False, -16),
            encode_halt(),
        ])


class TestCrossValidateLDRSTR:
    def test_cross(self) -> None:
        cross_validate("ldr_str", [
            encode_mov_imm(COND_AL, 0, 42),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
            encode_str(COND_AL, 0, 1, 0, True),
            encode_mov_imm(COND_AL, 0, 0),
            encode_ldr(COND_AL, 0, 1, 0, True),
            encode_halt(),
        ])


class TestCrossValidateSTMLDM:
    def test_cross(self) -> None:
        cross_validate("stm_ldm", [
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


class TestCrossValidateBranchAndLink:
    def test_cross(self) -> None:
        cross_validate("branch_and_link", [
            encode_mov_imm(COND_AL, 0, 7),
            encode_branch(COND_AL, True, 4),
            encode_halt(),
            0,
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 0),
            encode_data_processing(COND_AL, OP_MOV, 1, 0, 15, 14),
        ])


# =========================================================================
# Gate-level specific tests
# =========================================================================


class TestGateLevelNewAndReset:
    def test_initial_state(self) -> None:
        cpu = ARM1GateLevel(1024)
        assert cpu.mode == MODE_SVC
        assert cpu.pc == 0


class TestGateLevelHalt:
    def test_halt(self) -> None:
        cpu = ARM1GateLevel(1024)
        load_gate_program(cpu, [encode_halt()])
        traces = cpu.run(10)
        assert cpu.halted is True
        assert len(traces) == 1


class TestGateLevelGateOpsTracking:
    def test_gate_ops_tracked(self) -> None:
        cpu = ARM1GateLevel(1024)
        load_gate_program(cpu, [encode_mov_imm(COND_AL, 0, 42), encode_halt()])
        cpu.run(10)
        assert cpu.gate_ops > 0


class TestGateLevelVersion:
    def test_version(self) -> None:
        from arm1_gatelevel import __version__
        assert __version__ == "0.1.0"


# =========================================================================
# Additional coverage: ALU opcodes, barrel shifter edge cases, conditions
# =========================================================================


class TestGateALUMoreOps:
    """Cover all 16 ALU opcodes through gate-level execution."""

    def test_bic(self) -> None:
        a = int_to_bits(0xFFFFFFFF, 32)
        b = int_to_bits(0x0000FF00, 32)
        r = gate_alu_execute(0xE, a, b, 0, 0, 0)  # BIC
        assert bits_to_int(r["result"]) == 0xFFFF00FF

    def test_mvn(self) -> None:
        a = int_to_bits(0, 32)
        b = int_to_bits(0, 32)
        r = gate_alu_execute(0xF, a, b, 0, 0, 0)  # MVN
        assert bits_to_int(r["result"]) == 0xFFFFFFFF

    def test_mov(self) -> None:
        a = int_to_bits(0, 32)
        b = int_to_bits(42, 32)
        r = gate_alu_execute(0xD, a, b, 0, 0, 0)  # MOV
        assert bits_to_int(r["result"]) == 42

    def test_adc(self) -> None:
        a = int_to_bits(1, 32)
        b = int_to_bits(2, 32)
        r = gate_alu_execute(0x5, a, b, 1, 0, 0)  # ADC
        assert bits_to_int(r["result"]) == 4

    def test_sbc(self) -> None:
        a = int_to_bits(5, 32)
        b = int_to_bits(3, 32)
        r = gate_alu_execute(0x6, a, b, 1, 0, 0)  # SBC
        assert bits_to_int(r["result"]) == 2

    def test_rsb(self) -> None:
        a = int_to_bits(3, 32)
        b = int_to_bits(5, 32)
        r = gate_alu_execute(0x3, a, b, 0, 0, 0)  # RSB
        assert bits_to_int(r["result"]) == 2

    def test_rsc(self) -> None:
        a = int_to_bits(3, 32)
        b = int_to_bits(5, 32)
        r = gate_alu_execute(0x7, a, b, 1, 0, 0)  # RSC
        assert bits_to_int(r["result"]) == 2

    def test_cmn(self) -> None:
        a = int_to_bits(1, 32)
        b = int_to_bits(2, 32)
        r = gate_alu_execute(0xB, a, b, 0, 0, 0)  # CMN
        assert bits_to_int(r["result"]) == 3

    def test_tst(self) -> None:
        a = int_to_bits(0xFF, 32)
        b = int_to_bits(0x00, 32)
        r = gate_alu_execute(0x8, a, b, 0, 0, 0)  # TST
        assert r["z"] == 1

    def test_teq(self) -> None:
        a = int_to_bits(0xFF, 32)
        b = int_to_bits(0xFF, 32)
        r = gate_alu_execute(0x9, a, b, 0, 0, 0)  # TEQ
        assert r["z"] == 1

    def test_cmp(self) -> None:
        a = int_to_bits(5, 32)
        b = int_to_bits(5, 32)
        r = gate_alu_execute(0xA, a, b, 0, 0, 0)  # CMP
        assert r["z"] == 1 and r["c"] == 1

    def test_unknown_opcode(self) -> None:
        a = int_to_bits(0, 32)
        b = int_to_bits(0, 32)
        r = gate_alu_execute(0x10, a, b, 0, 0, 0)
        assert bits_to_int(r["result"]) == 0


class TestGateBarrelShiftEdgeCases:
    """Cover barrel shifter edge cases."""

    def test_lsl_0(self) -> None:
        value = int_to_bits(0xFF, 32)
        result, carry = gate_barrel_shift(value, 0, 0, 0, False)
        assert bits_to_int(result) == 0xFF

    def test_lsl_32(self) -> None:
        value = int_to_bits(1, 32)
        result, carry = gate_barrel_shift(value, 0, 32, 0, False)
        assert bits_to_int(result) == 0
        assert carry == 1

    def test_lsl_33(self) -> None:
        value = int_to_bits(1, 32)
        result, carry = gate_barrel_shift(value, 0, 33, 0, False)
        assert bits_to_int(result) == 0
        assert carry == 0

    def test_lsr_0_encodes_32(self) -> None:
        value = int_to_bits(0x80000000, 32)
        result, carry = gate_barrel_shift(value, 1, 0, 0, False)
        assert bits_to_int(result) == 0
        assert carry == 1

    def test_lsr_32_by_reg(self) -> None:
        value = int_to_bits(0x80000000, 32)
        result, carry = gate_barrel_shift(value, 1, 32, 0, True)
        assert bits_to_int(result) == 0

    def test_lsr_33(self) -> None:
        value = int_to_bits(1, 32)
        result, carry = gate_barrel_shift(value, 1, 33, 0, True)
        assert bits_to_int(result) == 0
        assert carry == 0

    def test_lsr_by_reg_0(self) -> None:
        value = int_to_bits(0x12345678, 32)
        result, carry = gate_barrel_shift(value, 1, 0, 1, True)
        assert bits_to_int(result) == 0x12345678

    def test_asr_1_negative(self) -> None:
        value = int_to_bits(0x80000000, 32)
        result, carry = gate_barrel_shift(value, 2, 1, 0, False)
        assert bits_to_int(result) == 0xC0000000

    def test_asr_0_encodes_32_negative(self) -> None:
        value = int_to_bits(0x80000000, 32)
        result, carry = gate_barrel_shift(value, 2, 0, 0, False)
        assert bits_to_int(result) == 0xFFFFFFFF
        assert carry == 1

    def test_asr_0_encodes_32_positive(self) -> None:
        value = int_to_bits(0x7FFFFFFF, 32)
        result, carry = gate_barrel_shift(value, 2, 0, 0, False)
        assert bits_to_int(result) == 0
        assert carry == 0

    def test_asr_by_reg_0(self) -> None:
        value = int_to_bits(0x12345678, 32)
        result, carry = gate_barrel_shift(value, 2, 0, 1, True)
        assert bits_to_int(result) == 0x12345678

    def test_asr_32(self) -> None:
        value = int_to_bits(0x80000000, 32)
        result, carry = gate_barrel_shift(value, 2, 32, 0, True)
        assert bits_to_int(result) == 0xFFFFFFFF

    def test_ror_by_reg_0(self) -> None:
        value = int_to_bits(0x12345678, 32)
        result, carry = gate_barrel_shift(value, 3, 0, 1, True)
        assert bits_to_int(result) == 0x12345678

    def test_ror_32(self) -> None:
        """ROR by 32 (mod 32 = 0): value unchanged, carry = bit 31."""
        value = int_to_bits(0x80000001, 32)
        result, carry = gate_barrel_shift(value, 3, 32, 0, True)
        assert bits_to_int(result) == 0x80000001
        assert carry == 1

    def test_by_register_amount_0(self) -> None:
        """When shifting by register with amount=0, pass through unchanged."""
        value = int_to_bits(0xABCDEF01, 32)
        result, carry = gate_barrel_shift(value, 0, 0, 1, True)
        assert bits_to_int(result) == 0xABCDEF01
        assert carry == 1

    def test_unknown_shift_type(self) -> None:
        value = int_to_bits(0xFF, 32)
        result, carry = gate_barrel_shift(value, 99, 4, 0, False)
        assert bits_to_int(result) == 0xFF


class TestGateLevelConditions:
    """Test gate-level condition evaluation indirectly through execution."""

    def test_all_conditions_via_cross_validate(self) -> None:
        """Test multiple conditions in a single program."""
        cross_validate("all_conditions", [
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),  # Sets Z, C
            encode_mov_imm(COND_NE, 3, 1),  # Should NOT execute (Z set)
            encode_mov_imm(COND_EQ, 4, 2),  # Should execute (Z set)
            encode_halt(),
        ])


class TestGateLevelSWI:
    """Test SWI through the gate-level simulator."""

    def test_swi(self) -> None:
        cpu = ARM1GateLevel(4096)
        cpu.write_word(0x08, encode_branch(COND_AL, False, 0xF0 - 0x08 - 8))
        cpu.write_word(0xF0, encode_mov_imm(COND_AL, 5, 99))
        cpu.write_word(0xF4, encode_halt())
        load_gate_program(cpu, [
            encode_mov_imm(COND_AL, 0, 1),
            (COND_AL << 28) | 0x0F000001,
            encode_halt(),
        ])
        cpu.write_word(0x08, encode_branch(COND_AL, False, 0xF0 - 0x08 - 8))
        cpu.run(20)
        assert cpu._read_reg(5) == 99
        assert cpu.mode == MODE_SVC


class TestGateLevelMemory:
    """Test gate-level memory operations."""

    def test_read_write_word(self) -> None:
        cpu = ARM1GateLevel(1024)
        cpu.write_word(0, 0xDEADBEEF)
        assert cpu.read_word(0) == 0xDEADBEEF

    def test_read_write_byte(self) -> None:
        cpu = ARM1GateLevel(1024)
        cpu.write_byte(0, 0xAB)
        assert cpu.read_byte(0) == 0xAB

    def test_out_of_bounds(self) -> None:
        cpu = ARM1GateLevel(64)
        assert cpu.read_word(0x100) == 0
        assert cpu.read_byte(0x100) == 0
        cpu.write_word(0x100, 0xDEAD)
        cpu.write_byte(0x100, 0xFF)

    def test_reset(self) -> None:
        cpu = ARM1GateLevel(1024)
        cpu._write_reg(0, 42)
        cpu.reset()
        assert cpu._read_reg(0) == 0
        assert cpu.mode == MODE_SVC
        assert cpu.pc == 0

    def test_flags_property(self) -> None:
        cpu = ARM1GateLevel(1024)
        f = cpu.flags
        assert not f.n and not f.z and not f.c and not f.v
