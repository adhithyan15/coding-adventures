"""Tests for the Intel 8008 gate-level simulator.

These tests verify:
1. Each component (ALU, registers, decoder, stack) works correctly
2. The full CPU pipeline produces correct results
3. Cross-validation: gate-level matches behavioral for the same programs

The gate-level simulator must produce IDENTICAL results to the behavioral
simulator for any program. This is verified via cross-validation tests that
run the same program on both and compare instruction-by-instruction.
"""

from __future__ import annotations

import pytest

from intel8008_gatelevel import Intel8008GateLevel
from intel8008_gatelevel.alu import GateALU8, Intel8008FlagBits
from intel8008_gatelevel.bits import bits_to_int, compute_parity, int_to_bits
from intel8008_gatelevel.decoder import decode
from intel8008_gatelevel.registers import REG_A, REG_B, REG_H, REG_L, RegisterFile
from intel8008_gatelevel.stack import PushDownStack
from intel8008_simulator import Intel8008Simulator


# ---------------------------------------------------------------------------
# bits.py tests
# ---------------------------------------------------------------------------


class TestBits:
    """Tests for bit conversion helpers."""

    def test_int_to_bits_zero(self) -> None:
        assert int_to_bits(0, 8) == [0, 0, 0, 0, 0, 0, 0, 0]

    def test_int_to_bits_one(self) -> None:
        assert int_to_bits(1, 8) == [1, 0, 0, 0, 0, 0, 0, 0]

    def test_int_to_bits_0xff(self) -> None:
        assert int_to_bits(0xFF, 8) == [1, 1, 1, 1, 1, 1, 1, 1]

    def test_int_to_bits_5(self) -> None:
        # 5 = 0b00000101: bit0=1, bit1=0, bit2=1
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_int_to_bits_14bit(self) -> None:
        # 14-bit value for PC
        assert len(int_to_bits(0x3FFF, 14)) == 14
        assert int_to_bits(0x3FFF, 14) == [1] * 14

    def test_bits_to_int_zero(self) -> None:
        assert bits_to_int([0, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_bits_to_int_one(self) -> None:
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_bits_to_int_roundtrip(self) -> None:
        for val in [0, 1, 5, 127, 128, 255]:
            assert bits_to_int(int_to_bits(val, 8)) == val

    def test_compute_parity_zero(self) -> None:
        # 0 ones → even parity → 1
        assert compute_parity([0, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_compute_parity_one_bit(self) -> None:
        # 1 one → odd parity → 0
        assert compute_parity([1, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_compute_parity_two_bits(self) -> None:
        # 2 ones → even parity → 1
        assert compute_parity([1, 1, 0, 0, 0, 0, 0, 0]) == 1

    def test_compute_parity_all_ones(self) -> None:
        # 8 ones → even parity → 1
        assert compute_parity([1, 1, 1, 1, 1, 1, 1, 1]) == 1

    def test_compute_parity_empty(self) -> None:
        # Empty → even (0 ones) → 1
        assert compute_parity([]) == 1


# ---------------------------------------------------------------------------
# GateALU8 tests
# ---------------------------------------------------------------------------


class TestGateALU8:
    """Tests for the 8-bit gate-level ALU."""

    def setup_method(self) -> None:
        self.alu = GateALU8()

    def test_add_basic(self) -> None:
        result, carry = self.alu.add(3, 4)
        assert result == 7
        assert carry is False

    def test_add_overflow(self) -> None:
        result, carry = self.alu.add(0xFF, 1)
        assert result == 0
        assert carry is True

    def test_add_with_carry_in(self) -> None:
        result, carry = self.alu.add(1, 1, carry_in=True)
        assert result == 3

    def test_subtract_basic(self) -> None:
        result, borrow = self.alu.subtract(5, 3)
        assert result == 2
        assert borrow is False

    def test_subtract_borrow(self) -> None:
        result, borrow = self.alu.subtract(1, 2)
        # 1 - 2 = -1 = 0xFF with borrow
        assert result == 0xFF
        assert borrow is True

    def test_subtract_self(self) -> None:
        result, borrow = self.alu.subtract(42, 42)
        assert result == 0
        assert borrow is False

    def test_bitwise_and(self) -> None:
        assert self.alu.bitwise_and(0xFF, 0x0F) == 0x0F

    def test_bitwise_or(self) -> None:
        assert self.alu.bitwise_or(0x0F, 0xF0) == 0xFF

    def test_bitwise_xor(self) -> None:
        assert self.alu.bitwise_xor(0xFF, 0xFF) == 0x00

    def test_bitwise_xor_partial(self) -> None:
        assert self.alu.bitwise_xor(0xAA, 0x55) == 0xFF

    def test_increment(self) -> None:
        result, carry = self.alu.increment(5)
        assert result == 6
        assert carry is False

    def test_increment_wraps(self) -> None:
        result, carry = self.alu.increment(0xFF)
        assert result == 0
        assert carry is True

    def test_decrement(self) -> None:
        result, borrow = self.alu.decrement(5)
        assert result == 4
        assert borrow is False

    def test_decrement_wraps(self) -> None:
        result, borrow = self.alu.decrement(0)
        assert result == 0xFF
        assert borrow is True

    def test_rotate_left_circular(self) -> None:
        result, carry = self.alu.rotate_left_circular(0x01)
        assert result == 0x02
        assert carry is False

    def test_rotate_left_circular_wraps(self) -> None:
        result, carry = self.alu.rotate_left_circular(0x80)
        assert result == 0x01
        assert carry is True

    def test_rotate_right_circular(self) -> None:
        result, carry = self.alu.rotate_right_circular(0x02)
        assert result == 0x01
        assert carry is False

    def test_rotate_right_circular_wraps(self) -> None:
        result, carry = self.alu.rotate_right_circular(0x01)
        assert result == 0x80
        assert carry is True

    def test_rotate_left_carry(self) -> None:
        # With carry_in=True: bit goes into LSB
        result, carry = self.alu.rotate_left_carry(0x01, True)
        assert result == 0x03  # 00000001 << 1 | 1 = 00000011
        assert carry is False

    def test_rotate_right_carry(self) -> None:
        result, carry = self.alu.rotate_right_carry(0x02, True)
        assert result == 0x81  # 1 << 7 | 00000001 = 10000001
        assert carry is False

    def test_compute_flags_zero_result(self) -> None:
        flags = self.alu.compute_flags(0, False)
        assert flags.zero == 1
        assert flags.sign == 0
        assert flags.parity == 1  # 0 ones = even parity
        assert flags.carry == 0

    def test_compute_flags_sign_bit(self) -> None:
        flags = self.alu.compute_flags(0x80, False)
        assert flags.sign == 1
        assert flags.zero == 0

    def test_compute_flags_carry(self) -> None:
        flags = self.alu.compute_flags(5, True)
        assert flags.carry == 1

    def test_compute_flags_parity_even(self) -> None:
        # 0xFF has 8 ones → even parity → P=1
        flags = self.alu.compute_flags(0xFF, False)
        assert flags.parity == 1

    def test_compare_equal(self) -> None:
        flags = self.alu.compare(5, 5)
        assert flags.zero == 1
        assert flags.carry == 0

    def test_compare_less(self) -> None:
        flags = self.alu.compare(1, 5)
        assert flags.zero == 0
        assert flags.carry == 1  # borrow occurred


# ---------------------------------------------------------------------------
# RegisterFile tests
# ---------------------------------------------------------------------------


class TestRegisterFile:
    """Tests for the 7-register file."""

    def setup_method(self) -> None:
        self.rf = RegisterFile()

    def test_initial_all_zero(self) -> None:
        for i in range(8):
            if i == 6:
                continue  # skip M
            assert self.rf.read(i) == 0

    def test_write_read_a(self) -> None:
        self.rf.write(REG_A, 42)
        assert self.rf.read(REG_A) == 42

    def test_write_read_b(self) -> None:
        self.rf.write(REG_B, 0xFF)
        assert self.rf.read(REG_B) == 0xFF

    def test_write_masks_to_8_bits(self) -> None:
        self.rf.write(REG_A, 0x1FF)  # 9-bit value
        assert self.rf.read(REG_A) == 0xFF

    def test_read_m_raises(self) -> None:
        with pytest.raises(ValueError):
            self.rf.read(6)

    def test_write_m_raises(self) -> None:
        with pytest.raises(ValueError):
            self.rf.write(6, 0)

    def test_hl_address(self) -> None:
        self.rf.write(REG_H, 0x10)
        self.rf.write(REG_L, 0x20)
        # address = (0x10 & 0x3F) << 8 | 0x20 = 0x1020
        assert self.rf.hl_address == 0x1020

    def test_hl_address_masks_h(self) -> None:
        self.rf.write(REG_H, 0xFF)  # only low 6 bits used
        self.rf.write(REG_L, 0x00)
        assert self.rf.hl_address == (0x3F << 8)  # = 0x3F00

    def test_read_bits_returns_list(self) -> None:
        self.rf.write(REG_A, 5)
        bits = self.rf.read_bits(REG_A)
        assert isinstance(bits, list)
        assert len(bits) == 8
        assert bits_to_int(bits) == 5

    def test_reset_clears_all(self) -> None:
        self.rf.write(REG_A, 0xFF)
        self.rf.reset()
        assert self.rf.read(REG_A) == 0


# ---------------------------------------------------------------------------
# Decoder tests
# ---------------------------------------------------------------------------


class TestDecoder:
    """Tests for the combinational instruction decoder."""

    def test_decode_hlt_76(self) -> None:
        d = decode(0x76)
        assert d.is_halt == 1

    def test_decode_hlt_ff(self) -> None:
        d = decode(0xFF)
        assert d.is_halt == 1

    def test_decode_mov_a_b(self) -> None:
        # MOV A,B = 0x78 = 01 111 000
        d = decode(0x78)
        assert d.is_mov == 1
        assert d.reg_dst == 7  # A
        assert d.reg_src == 0  # B

    def test_decode_mvi_a(self) -> None:
        # MVI A = 0x3E = 00 111 110
        d = decode(0x3E)
        assert d.is_mvi == 1
        assert d.reg_dst == 7
        assert d.instruction_bytes == 2

    def test_decode_mvi_b(self) -> None:
        # MVI B = 0x06 = 00 000 110
        d = decode(0x06)
        assert d.is_mvi == 1
        assert d.reg_dst == 0

    def test_decode_inr_b(self) -> None:
        # INR B = 0x00 = 00 000 000
        d = decode(0x00)
        assert d.is_inr == 1

    def test_decode_dcr_b(self) -> None:
        # DCR B = 0x01 = 00 000 001
        d = decode(0x01)
        assert d.is_dcr == 1

    def test_decode_add_b(self) -> None:
        # ADD B = 0x80 = 10 000 000
        d = decode(0x80)
        assert d.is_alu_reg == 1
        assert d.alu_op == 0  # ADD
        assert d.reg_src == 0  # B

    def test_decode_sub_a(self) -> None:
        # SUB A = 0x97 = 10 010 111
        d = decode(0x97)
        assert d.is_alu_reg == 1
        assert d.alu_op == 2  # SUB
        assert d.reg_src == 7  # A

    def test_decode_adi(self) -> None:
        # ADI = 0xC4 = 11 000 100
        d = decode(0xC4)
        assert d.is_alu_imm == 1
        assert d.alu_op == 0  # ADD
        assert d.instruction_bytes == 2

    def test_decode_rlc(self) -> None:
        # RLC = 0x02 = 00 000 010
        d = decode(0x02)
        assert d.is_rotate == 1
        assert d.rotate_type == 0

    def test_decode_rrc(self) -> None:
        # RRC = 0x0A = 00 001 010
        d = decode(0x0A)
        assert d.is_rotate == 1
        assert d.rotate_type == 1

    def test_decode_jmp(self) -> None:
        # JMP = 0x7C = 01 111 100
        d = decode(0x7C)
        assert d.is_jump == 1
        assert d.unconditional == 1
        assert d.instruction_bytes == 3

    def test_decode_cal(self) -> None:
        # CAL = 0x7E = 01 111 110
        d = decode(0x7E)
        assert d.is_call == 1
        assert d.unconditional == 1
        assert d.instruction_bytes == 3

    def test_decode_jfc(self) -> None:
        # JFC = 0x40 = 01 000 000
        d = decode(0x40)
        assert d.is_jump == 1
        assert d.cond_code == 0  # CY
        assert d.cond_sense == 0  # if false/clear

    def test_decode_jtc(self) -> None:
        # JTC = 0x44 = 01 000 100
        d = decode(0x44)
        assert d.is_jump == 1
        assert d.cond_code == 0  # CY
        assert d.cond_sense == 1  # if true/set

    def test_decode_ret(self) -> None:
        # RET = 0x3F = 00 111 111
        d = decode(0x3F)
        assert d.is_ret == 1
        assert d.unconditional == 1

    def test_decode_rst_1(self) -> None:
        # RST 1 = 0x0D = 00 001 101
        d = decode(0x0D)
        assert d.is_rst == 1
        assert d.port_or_rst == 1  # RST 1 → target = 8

    def test_decode_in_0(self) -> None:
        # IN 0 = 0x41 = 01 000 001
        d = decode(0x41)
        assert d.is_in == 1
        assert d.port_or_rst == 0  # port 0

    def test_decode_in_3(self) -> None:
        # IN 3 = 0x59 = 01 011 001
        d = decode(0x59)
        assert d.is_in == 1
        assert d.port_or_rst == 3  # port 3


# ---------------------------------------------------------------------------
# PushDownStack tests
# ---------------------------------------------------------------------------


class TestPushDownStack:
    """Tests for the 8-level push-down stack."""

    def setup_method(self) -> None:
        self.stack = PushDownStack()

    def test_initial_pc_zero(self) -> None:
        assert self.stack.current_pc() == 0

    def test_load_sets_pc(self) -> None:
        self.stack.load(0x100)
        assert self.stack.current_pc() == 0x100

    def test_increment_advances_pc(self) -> None:
        self.stack.load(0x100)
        self.stack.increment()
        assert self.stack.current_pc() == 0x101

    def test_increment_wraps(self) -> None:
        self.stack.load(0x3FFF)
        self.stack.increment()
        assert self.stack.current_pc() == 0

    def test_push_and_jump_sets_target(self) -> None:
        self.stack.load(0x103)  # return address is current PC
        self.stack.push_and_jump(0x103, 0x200)
        assert self.stack.current_pc() == 0x200

    def test_push_saves_return_addr(self) -> None:
        self.stack.load(0x103)
        self.stack.push_and_jump(0x103, 0x200)
        assert self.stack.entries[1] == 0x103

    def test_pop_restores_pc(self) -> None:
        self.stack.load(0x103)
        self.stack.push_and_jump(0x103, 0x200)
        self.stack.pop()
        assert self.stack.current_pc() == 0x103

    def test_depth_increments_on_push(self) -> None:
        self.stack.push_and_jump(0, 0x100)
        assert self.stack.depth == 1

    def test_depth_decrements_on_pop(self) -> None:
        self.stack.push_and_jump(0, 0x100)
        self.stack.pop()
        assert self.stack.depth == 0

    def test_nested_calls(self) -> None:
        self.stack.load(0x10)
        self.stack.push_and_jump(0x10, 0x100)  # call level 1
        self.stack.increment()   # advance past some instructions
        self.stack.push_and_jump(self.stack.current_pc(), 0x200)  # call level 2
        assert self.stack.depth == 2
        self.stack.pop()
        assert self.stack.current_pc() == 0x101  # returned from level 2
        self.stack.pop()
        assert self.stack.current_pc() == 0x10   # returned from level 1

    def test_reset_clears(self) -> None:
        self.stack.load(0x100)
        self.stack.push_and_jump(0, 0x200)
        self.stack.reset()
        assert self.stack.current_pc() == 0
        assert self.stack.depth == 0


# ---------------------------------------------------------------------------
# Full CPU tests
# ---------------------------------------------------------------------------


class TestCPU:
    """End-to-end tests for the gate-level CPU."""

    def test_mvi_and_add(self) -> None:
        # MVI B,1; MVI A,2; ADD B; HLT
        program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 3
        assert cpu.flags.carry is False
        assert cpu.flags.parity is True

    def test_sub_self(self) -> None:
        # MVI A,42; SUB A; HLT → A=0, Z=1
        program = bytes([0x3E, 0x2A, 0x97, 0x76])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 0
        assert cpu.flags.zero is True

    def test_flags_zero_after_xra_self(self) -> None:
        # XRA A (0xAF): A=0, CY=0, Z=1
        program = bytes([0x3E, 0x42, 0xAF, 0x76])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 0
        assert cpu.flags.zero is True
        assert cpu.flags.carry is False

    def test_inr_dcr(self) -> None:
        # MVI B,5; DCR B; HLT → B=4
        program = bytes([0x06, 0x05, 0x01, 0x76])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.b == 4

    def test_jmp_unconditional(self) -> None:
        program = bytes([
            0x7C, 0x04, 0x00,   # JMP 0x0004
            0x76,               # HLT (skipped)
            0x3E, 0x42,         # MVI A,0x42
            0x76,               # HLT
        ])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 0x42

    def test_call_ret(self) -> None:
        program = bytes([
            0x7E, 0x06, 0x00,   # CAL 0x0006
            0x76,               # HLT
            0x00, 0x00,
            0x3E, 0x55,         # MVI A,0x55 at 0x0006
            0x3F,               # RET
        ])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 0x55

    def test_rotate_rlc(self) -> None:
        program = bytes([0x3E, 0x80, 0x02, 0x76])  # MVI A,0x80; RLC
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 0x01
        assert cpu.flags.carry is True

    def test_memory_mov(self) -> None:
        program = bytes([
            0x26, 0x00,   # MVI H,0
            0x2E, 0x20,   # MVI L,0x20
            0x3E, 0x42,   # MVI A,0x42
            0x77,         # MOV M,A
            0x76,         # HLT
        ])
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.memory[0x20] == 0x42

    def test_rst(self) -> None:
        program = bytearray(64)
        program[0] = 0x0D     # RST 1 → call 0x0008
        program[1] = 0x76     # HLT
        program[0x08] = 0x3E  # MVI A,7 at 0x0008
        program[0x09] = 0x07
        program[0x0A] = 0x3F  # RET
        cpu = Intel8008GateLevel()
        cpu.run(bytes(program))
        assert cpu.a == 7

    def test_adi_immediate(self) -> None:
        program = bytes([0x3E, 0x05, 0xC4, 0x03, 0x76])  # MVI A,5; ADI 3; HLT
        cpu = Intel8008GateLevel()
        cpu.run(program)
        assert cpu.a == 8

    def test_in_out(self) -> None:
        cpu = Intel8008GateLevel()
        cpu.set_input_port(0, 0xAB)
        program = bytes([0x41, 0x76])  # IN 0; HLT
        cpu.run(program)
        assert cpu.a == 0xAB

    def test_halted_raises(self) -> None:
        cpu = Intel8008GateLevel()
        cpu.run(bytes([0x76]))
        with pytest.raises(RuntimeError):
            cpu.step()

    def test_gate_count(self) -> None:
        cpu = Intel8008GateLevel()
        counts = cpu.gate_count()
        assert "alu" in counts
        assert "total" in counts
        assert counts["total"] > 100

    def test_reset_clears_state(self) -> None:
        cpu = Intel8008GateLevel()
        cpu.run(bytes([0x3E, 0xFF, 0x76]))
        cpu.reset()
        assert cpu.a == 0
        assert cpu.pc == 0

    def test_max_steps(self) -> None:
        program = bytes([0x7C, 0x00, 0x00])  # infinite JMP 0
        cpu = Intel8008GateLevel()
        traces = cpu.run(program, max_steps=5)
        assert len(traces) == 5
        assert not cpu.halted


# ---------------------------------------------------------------------------
# Cross-validation tests
# ---------------------------------------------------------------------------


class TestCrossValidation:
    """Verify gate-level produces identical results to behavioral simulator."""

    def _cross_validate(self, program: bytes, max_steps: int = 100) -> None:
        """Run program on both simulators and compare every trace."""
        bsim = Intel8008Simulator()
        gsim = Intel8008GateLevel()

        b_traces = bsim.run(program, max_steps=max_steps)
        g_traces = gsim.run(program, max_steps=max_steps)

        assert len(b_traces) == len(g_traces), (
            f"Different trace lengths: behavioral={len(b_traces)}, "
            f"gate-level={len(g_traces)}"
        )

        for i, (bt, gt) in enumerate(zip(b_traces, g_traces)):
            assert bt.a_after == gt.a_after, (
                f"Step {i} ({bt.mnemonic}): A mismatch: "
                f"behavioral={bt.a_after}, gate={gt.a_after}"
            )
            assert bt.flags_after == gt.flags_after, (
                f"Step {i} ({bt.mnemonic}): flags mismatch: "
                f"behavioral={bt.flags_after}, gate={gt.flags_after}"
            )

    def test_basic_addition(self) -> None:
        """x = 1 + 2"""
        program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
        self._cross_validate(program)

    def test_alu_operations(self) -> None:
        """Multiple ALU operations in sequence."""
        program = bytes([
            0x3E, 0x10,   # MVI A, 0x10
            0x06, 0x05,   # MVI B, 5
            0x80,         # ADD B  → A = 0x15
            0x97,         # SUB A  → A = 0
            0x3E, 0xFF,   # MVI A, 0xFF
            0xAF,         # XRA A  → A = 0 (also clears carry)
            0x76,         # HLT
        ])
        self._cross_validate(program)

    def test_flag_computation(self) -> None:
        """Verify all 4 flags match between simulators."""
        program = bytes([
            0x3E, 0xFF,   # MVI A, 0xFF
            0xC4, 0x01,   # ADI 1   → A=0, Z=1, CY=1
            0xC4, 0x00,   # ADI 0   → A=0, Z=1
            0xC4, 0x01,   # ADI 1   → A=1, Z=0
            0x76,
        ])
        self._cross_validate(program)

    def test_carry_propagation(self) -> None:
        """ADC uses carry from previous ADD."""
        program = bytes([
            0x3E, 0xFF,   # MVI A, 0xFF
            0x80,         # ADD B (B=0) → A=0xFF, no carry
            0xC4, 0x01,   # ADI 1 → A=0, CY=1
            0xCC, 0x00,   # ACI 0 → A=1 (0 + CY=1), CY=0
            0x76,
        ])
        self._cross_validate(program)

    def test_subtraction_borrow(self) -> None:
        """SUB with borrow."""
        program = bytes([
            0x3E, 0x01,   # MVI A, 1
            0xD4, 0x02,   # SUI 2 → A=0xFF, CY=1 (borrow)
            0x76,
        ])
        self._cross_validate(program)

    def test_rotate_instructions(self) -> None:
        """RLC, RRC, RAL, RAR."""
        program = bytes([
            0x3E, 0x01,   # MVI A, 0x01
            0x02,         # RLC → A=0x02, CY=0
            0x02,         # RLC → A=0x04
            0x0A,         # RRC → A=0x02
            0x12,         # RAL → A=0x04, CY=0 (uses old CY=0 as new bit0)
            0x1A,         # RAR
            0x76,
        ])
        self._cross_validate(program)

    def test_inr_dcr_no_carry(self) -> None:
        """INR and DCR preserve carry."""
        program = bytes([
            0x3E, 0xFF,   # MVI A, 0xFF
            0xC4, 0x01,   # ADI 1 → CY=1
            0x06, 0x00,   # MVI B, 0
            0x00,         # INR B → B=1, CY preserved
            0x01,         # DCR B → B=0, CY preserved
            0x76,
        ])
        self._cross_validate(program)

    def test_jump_conditional(self) -> None:
        """Conditional jump: JFZ jumps when Z=0."""
        program = bytes([
            0x06, 0x03,         # MVI B, 3 (loop count)
            0x01,               # DCR B (at 0x0002)
            0x48, 0x02, 0x00,   # JFZ 0x0002 (loop while Z=0)
            0x76,               # HLT
        ])
        self._cross_validate(program, max_steps=50)

    def test_call_and_return(self) -> None:
        """CAL and RET: subroutine doubles A."""
        program = bytes([
            0x3E, 0x05,         # MVI A, 5
            0x7E, 0x08, 0x00,   # CAL 0x0008 (subroutine: double A)
            0x76,               # HLT
            0x00,               # padding
            0x80,               # ADD A (double A) at 0x0008
            0x3F,               # RET
        ])
        self._cross_validate(program)

    def test_inr_m_memory(self) -> None:
        """INR M increments memory via H:L."""
        program = bytes([
            0x26, 0x00,   # MVI H, 0
            0x2E, 0x30,   # MVI L, 0x30
            0x36, 0x04,   # MVI M, 4 → mem[0x30] = 4
            0x30,         # INR M → mem[0x30] = 5
            0x76,
        ])
        self._cross_validate(program)
