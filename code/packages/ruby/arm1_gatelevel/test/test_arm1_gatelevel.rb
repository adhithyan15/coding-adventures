# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_arm1_gatelevel"

# ===========================================================================
# ARM1 Gate-Level Simulator Tests
# ===========================================================================
#
# These tests validate:
#   1. Bit conversion helpers (int_to_bits / bits_to_int)
#   2. Gate-level ALU (all operations via ripple-carry adders and gate trees)
#   3. Gate-level barrel shifter (mux tree implementation)
#   4. Cross-validation against the behavioral simulator
#
# The cross-validation tests are the ultimate correctness guarantee: we run
# the same program on both simulators and verify they produce identical
# register and flag state after every instruction.

module CodingAdventures
  module Arm1Gatelevel

    # Helper: load a program from uint32 instruction words
    module ProgramLoader
      def load_gate_program(cpu, instructions)
        code = instructions.flat_map do |inst|
          [inst & 0xFF, (inst >> 8) & 0xFF, (inst >> 16) & 0xFF, (inst >> 24) & 0xFF]
        end
        cpu.load_program(code, 0)
      end

      def load_behavioral_program(cpu, instructions)
        code = instructions.flat_map do |inst|
          [inst & 0xFF, (inst >> 8) & 0xFF, (inst >> 16) & 0xFF, (inst >> 24) & 0xFF]
        end
        cpu.load_program(code, 0)
      end
    end

    # =====================================================================
    # Bit Conversion
    # =====================================================================

    class TestBitConversion < Minitest::Test
      def test_int_to_bits_basic
        bits = Arm1Gatelevel.int_to_bits(5, 32)
        assert_equal 1, bits[0]  # LSB
        assert_equal 0, bits[1]
        assert_equal 1, bits[2]
        assert_equal 5, Arm1Gatelevel.bits_to_int(bits)
      end

      def test_roundtrip
        [0, 1, 42, 0xFF, 0xDEADBEEF, 0xFFFFFFFF].each do |v|
          bits = Arm1Gatelevel.int_to_bits(v, 32)
          got = Arm1Gatelevel.bits_to_int(bits)
          assert_equal v, got, "round-trip failed for #{v}"
        end
      end
    end

    # =====================================================================
    # Gate-level ALU
    # =====================================================================

    class TestGateALU < Minitest::Test
      def test_add
        a = Arm1Gatelevel.int_to_bits(1, 32)
        b = Arm1Gatelevel.int_to_bits(2, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_ADD, a, b, 0, 0, 0)
        assert_equal 3, Arm1Gatelevel.bits_to_int(r.result)
        assert_equal 0, r.n
        assert_equal 0, r.z
        assert_equal 0, r.c
        assert_equal 0, r.v
      end

      def test_sub_zero
        a = Arm1Gatelevel.int_to_bits(5, 32)
        b = Arm1Gatelevel.int_to_bits(5, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_SUB, a, b, 0, 0, 0)
        assert_equal 0, Arm1Gatelevel.bits_to_int(r.result)
        assert_equal 1, r.z, "Z should be set for 5-5=0"
        assert_equal 1, r.c, "C should be set (no borrow)"
      end

      def test_logical_and
        a = Arm1Gatelevel.int_to_bits(0xFF00FF00, 32)
        b = Arm1Gatelevel.int_to_bits(0x0FF00FF0, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_AND, a, b, 0, 0, 0)
        assert_equal 0x0F000F00, Arm1Gatelevel.bits_to_int(r.result)
      end

      def test_logical_eor
        a = Arm1Gatelevel.int_to_bits(0xFF00FF00, 32)
        b = Arm1Gatelevel.int_to_bits(0x0FF00FF0, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_EOR, a, b, 0, 0, 0)
        assert_equal 0xF0F0F0F0, Arm1Gatelevel.bits_to_int(r.result)
      end

      def test_logical_orr
        a = Arm1Gatelevel.int_to_bits(0xFF00FF00, 32)
        b = Arm1Gatelevel.int_to_bits(0x0FF00FF0, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_ORR, a, b, 0, 0, 0)
        assert_equal 0xFFF0FFF0, Arm1Gatelevel.bits_to_int(r.result)
      end

      def test_mov
        b = Arm1Gatelevel.int_to_bits(42, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_MOV, Array.new(32, 0), b, 0, 0, 0)
        assert_equal 42, Arm1Gatelevel.bits_to_int(r.result)
      end

      def test_mvn
        b = Arm1Gatelevel.int_to_bits(0, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_MVN, Array.new(32, 0), b, 0, 0, 0)
        assert_equal 0xFFFFFFFF, Arm1Gatelevel.bits_to_int(r.result)
      end

      def test_bic
        a = Arm1Gatelevel.int_to_bits(0xFFFFFFFF, 32)
        b = Arm1Gatelevel.int_to_bits(0x0000FF00, 32)
        r = Arm1Gatelevel.gate_alu_execute(Sim::OP_BIC, a, b, 0, 0, 0)
        assert_equal 0xFFFF00FF, Arm1Gatelevel.bits_to_int(r.result)
      end
    end

    # =====================================================================
    # Gate-level Barrel Shifter
    # =====================================================================

    class TestGateBarrelShifter < Minitest::Test
      def test_lsl_4
        value = Arm1Gatelevel.int_to_bits(0xFF, 32)
        result, _ = Arm1Gatelevel.gate_barrel_shift(value, 0, 4, 0, false)
        assert_equal 0xFF0, Arm1Gatelevel.bits_to_int(result)
      end

      def test_lsr_8
        value = Arm1Gatelevel.int_to_bits(0xFF00, 32)
        result, _ = Arm1Gatelevel.gate_barrel_shift(value, 1, 8, 0, false)
        assert_equal 0xFF, Arm1Gatelevel.bits_to_int(result)
      end

      def test_ror_4
        value = Arm1Gatelevel.int_to_bits(0x0000000F, 32)
        result, _ = Arm1Gatelevel.gate_barrel_shift(value, 3, 4, 0, false)
        assert_equal 0xF0000000, Arm1Gatelevel.bits_to_int(result)
      end

      def test_rrx
        value = Arm1Gatelevel.int_to_bits(0x00000001, 32)
        result, carry = Arm1Gatelevel.gate_barrel_shift(value, 3, 0, 1, false)
        assert_equal 0x80000000, Arm1Gatelevel.bits_to_int(result)
        assert_equal 1, carry, "carry should be 1 (old bit 0 was 1)"
      end

      def test_asr_negative
        value = Arm1Gatelevel.int_to_bits(0x80000000, 32)
        result, _ = Arm1Gatelevel.gate_barrel_shift(value, 2, 1, 0, false)
        assert_equal 0xC0000000, Arm1Gatelevel.bits_to_int(result)
      end
    end

    # =====================================================================
    # Gate-level CPU basics
    # =====================================================================

    class TestGateLevelCPU < Minitest::Test
      include ProgramLoader

      def test_new_and_reset
        cpu = ARM1GateLevel.new(1024)
        assert_equal Sim::MODE_SVC, cpu.mode
        assert_equal 0, cpu.pc
      end

      def test_halt
        cpu = ARM1GateLevel.new(1024)
        load_gate_program(cpu, [Sim.encode_halt])
        traces = cpu.run(10)
        assert cpu.halted?
        assert_equal 1, traces.length
      end

      def test_gate_ops_tracking
        cpu = ARM1GateLevel.new(1024)
        load_gate_program(cpu, [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 42),
          Sim.encode_halt
        ])
        cpu.run(10)
        assert cpu.gate_ops > 0, "gate ops should be non-zero after execution"
      end
    end

    # =====================================================================
    # Cross-validation: Gate-level vs Behavioral
    # =====================================================================
    #
    # This is the ultimate correctness guarantee. We run the same program
    # on both simulators and verify they produce identical results.

    class TestCrossValidation < Minitest::Test
      include ProgramLoader

      def cross_validate(name, instructions)
        behavioral = Sim::ARM1.new(4096)
        gate_level = ARM1GateLevel.new(4096)

        load_behavioral_program(behavioral, instructions)
        load_gate_program(gate_level, instructions)

        b_traces = behavioral.run(200)
        g_traces = gate_level.run(200)

        assert_equal b_traces.length, g_traces.length,
          "#{name}: trace count mismatch: behavioral=#{b_traces.length} gate=#{g_traces.length}"

        b_traces.each_with_index do |bt, i|
          gt = g_traces[i]

          assert_equal bt.address, gt.address,
            "#{name} step #{i}: address mismatch"
          assert_equal bt.condition_met, gt.condition_met,
            "#{name} step #{i}: condition mismatch"

          16.times do |r|
            assert_equal bt.regs_after[r], gt.regs_after[r],
              "#{name} step #{i}: R#{r} mismatch: B=0x#{bt.regs_after[r].to_s(16)} G=0x#{gt.regs_after[r].to_s(16)}"
          end

          assert_equal bt.flags_after, gt.flags_after,
            "#{name} step #{i}: flags mismatch"
        end
      end

      def test_one_plus_two
        cross_validate("1+2", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 1),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 2),
          Sim.encode_alu_reg(Sim::COND_AL, Sim::OP_ADD, 0, 2, 0, 1),
          Sim.encode_halt
        ])
      end

      def test_subs_with_flags
        cross_validate("SUBS", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 5),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 5),
          Sim.encode_alu_reg(Sim::COND_AL, Sim::OP_SUB, 1, 2, 0, 1),
          Sim.encode_halt
        ])
      end

      def test_conditional
        cross_validate("conditional", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 5),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 5),
          Sim.encode_alu_reg(Sim::COND_AL, Sim::OP_SUB, 1, 2, 0, 1),
          Sim.encode_mov_imm(Sim::COND_NE, 3, 99),
          Sim.encode_mov_imm(Sim::COND_EQ, 4, 42),
          Sim.encode_halt
        ])
      end

      def test_barrel_shifter
        # ADD R1, R0, R0, LSL #2 (multiply by 5)
        add_with_shift = (Sim::COND_AL << 28) |
                         (Sim::OP_ADD << 21) |
                         (0 << 16) | (1 << 12) | (2 << 7) |
                         (Sim::SHIFT_LSL << 5) | 0

        cross_validate("barrel_shifter", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 7),
          add_with_shift,
          Sim.encode_halt
        ])
      end

      def test_loop
        cross_validate("loop_sum_1_to_10", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 0),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 10),
          Sim.encode_alu_reg(Sim::COND_AL, Sim::OP_ADD, 0, 0, 0, 1),
          Sim.encode_data_processing(Sim::COND_AL, Sim::OP_SUB, 1, 1, 1, (1 << 25) | 1),
          Sim.encode_branch(Sim::COND_NE, false, -16),
          Sim.encode_halt
        ])
      end

      def test_ldr_str
        cross_validate("ldr_str", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 42),
          Sim.encode_data_processing(Sim::COND_AL, Sim::OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
          Sim.encode_str(Sim::COND_AL, 0, 1, 0, true),
          Sim.encode_mov_imm(Sim::COND_AL, 0, 0),
          Sim.encode_ldr(Sim::COND_AL, 0, 1, 0, true),
          Sim.encode_halt
        ])
      end

      def test_stm_ldm
        cross_validate("stm_ldm", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 10),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 20),
          Sim.encode_mov_imm(Sim::COND_AL, 2, 30),
          Sim.encode_mov_imm(Sim::COND_AL, 3, 40),
          Sim.encode_data_processing(Sim::COND_AL, Sim::OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
          Sim.encode_stm(Sim::COND_AL, 5, 0x000F, true, "IA"),
          Sim.encode_mov_imm(Sim::COND_AL, 0, 0),
          Sim.encode_mov_imm(Sim::COND_AL, 1, 0),
          Sim.encode_mov_imm(Sim::COND_AL, 2, 0),
          Sim.encode_mov_imm(Sim::COND_AL, 3, 0),
          Sim.encode_data_processing(Sim::COND_AL, Sim::OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
          Sim.encode_ldm(Sim::COND_AL, 5, 0x000F, true, "IA"),
          Sim.encode_halt
        ])
      end

      def test_branch_and_link
        cross_validate("branch_and_link", [
          Sim.encode_mov_imm(Sim::COND_AL, 0, 7),
          Sim.encode_branch(Sim::COND_AL, true, 4),
          Sim.encode_halt,
          0,
          Sim.encode_alu_reg(Sim::COND_AL, Sim::OP_ADD, 0, 0, 0, 0),
          Sim.encode_data_processing(Sim::COND_AL, Sim::OP_MOV, 1, 0, 15, 14)
        ])
      end
    end
  end
end
