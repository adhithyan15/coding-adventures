# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Tests for opcodes and instruction construction.
    # =========================================================================
    #
    # Opcodes are the "verbs" of GPU programs. Each opcode tells the core
    # what operation to perform. These tests verify the opcode set and the
    # Instruction data structure.

    class TestOpcode < Minitest::Test
      # Verify all 16 opcodes are defined.
      def test_all_opcodes_exist
        assert_equal 16, ALL_OPCODES.length
      end

      # Opcode values are lowercase symbols.
      def test_opcode_values
        assert_includes ALL_OPCODES, :fadd
        assert_includes ALL_OPCODES, :halt
      end
    end

    class TestInstruction < Minitest::Test
      # Instructions are immutable (Data.define objects are frozen).
      def test_frozen
        inst = GpuCore.fadd(0, 1, 2)
        assert inst.frozen?
      end

      # Default fields are provided by helper constructors.
      def test_defaults
        inst = GpuCore.nop
        assert_equal 0, inst.rd
        assert_equal 0, inst.rs1
        assert_equal 0, inst.rs2
        assert_equal 0, inst.rs3
        assert_equal 0.0, inst.immediate
      end

      # FADD to_s shows assembly-like syntax.
      def test_to_s_fadd
        inst = GpuCore.fadd(2, 0, 1)
        assert_equal "FADD R2, R0, R1", inst.to_s
      end

      # FFMA to_s shows all four register operands.
      def test_to_s_ffma
        inst = GpuCore.ffma(3, 0, 1, 2)
        assert_equal "FFMA R3, R0, R1, R2", inst.to_s
      end

      # LIMM to_s shows the immediate value.
      def test_to_s_limm
        inst = GpuCore.limm(0, 3.14)
        assert_includes inst.to_s, "3.14"
      end

      # LOAD to_s shows memory access syntax.
      def test_to_s_load
        inst = GpuCore.load(0, 1, 4.0)
        assert_includes inst.to_s, "LOAD"
        assert_includes inst.to_s, "[R1+"
      end

      # STORE to_s shows memory access syntax.
      def test_to_s_store
        inst = GpuCore.store(1, 2, 8.0)
        assert_includes inst.to_s, "STORE"
      end

      # BEQ to_s shows branch offset.
      def test_to_s_beq
        inst = GpuCore.beq(0, 1, 3)
        assert_includes inst.to_s, "BEQ"
        assert_includes inst.to_s, "+3"
      end

      # BEQ with negative offset shows minus sign.
      def test_to_s_beq_negative
        inst = GpuCore.beq(0, 1, -2)
        assert_includes inst.to_s, "-2"
      end

      # HALT to_s is simple.
      def test_to_s_halt
        assert_equal "HALT", GpuCore.halt.to_s
      end

      # NOP to_s is simple.
      def test_to_s_nop
        assert_equal "NOP", GpuCore.nop.to_s
      end

      # JMP to_s shows target.
      def test_to_s_jmp
        inst = GpuCore.jmp(5)
        assert_includes inst.to_s, "JMP"
        assert_includes inst.to_s, "5"
      end
    end

    class TestHelperConstructors < Minitest::Test
      def test_fadd
        inst = GpuCore.fadd(2, 0, 1)
        assert_equal :fadd, inst.opcode
        assert_equal 2, inst.rd
        assert_equal 0, inst.rs1
        assert_equal 1, inst.rs2
      end

      def test_fsub
        inst = GpuCore.fsub(2, 0, 1)
        assert_equal :fsub, inst.opcode
      end

      def test_fmul
        inst = GpuCore.fmul(2, 0, 1)
        assert_equal :fmul, inst.opcode
      end

      def test_ffma
        inst = GpuCore.ffma(3, 0, 1, 2)
        assert_equal :ffma, inst.opcode
        assert_equal 2, inst.rs3
      end

      def test_fneg
        inst = GpuCore.fneg(1, 0)
        assert_equal :fneg, inst.opcode
        assert_equal 1, inst.rd
        assert_equal 0, inst.rs1
      end

      def test_fabs
        inst = GpuCore.fabs(1, 0)
        assert_equal :fabs, inst.opcode
      end

      def test_load
        inst = GpuCore.load(0, 1, 4.0)
        assert_equal :load, inst.opcode
        assert_equal 0, inst.rd
        assert_equal 1, inst.rs1
        assert_equal 4.0, inst.immediate
      end

      def test_load_default_offset
        inst = GpuCore.load(0, 1)
        assert_equal 0.0, inst.immediate
      end

      def test_store
        inst = GpuCore.store(1, 2, 8.0)
        assert_equal :store, inst.opcode
        assert_equal 1, inst.rs1
        assert_equal 2, inst.rs2
        assert_equal 8.0, inst.immediate
      end

      def test_mov
        inst = GpuCore.mov(1, 0)
        assert_equal :mov, inst.opcode
      end

      def test_limm
        inst = GpuCore.limm(0, 3.14)
        assert_equal :limm, inst.opcode
        assert_equal 3.14, inst.immediate
      end

      def test_beq
        inst = GpuCore.beq(0, 1, 3)
        assert_equal :beq, inst.opcode
        assert_equal 3.0, inst.immediate
      end

      def test_blt
        inst = GpuCore.blt(0, 1, -2)
        assert_equal :blt, inst.opcode
        assert_equal(-2.0, inst.immediate)
      end

      def test_bne
        inst = GpuCore.bne(0, 1, 5)
        assert_equal :bne, inst.opcode
      end

      def test_jmp
        inst = GpuCore.jmp(10)
        assert_equal :jmp, inst.opcode
        assert_equal 10.0, inst.immediate
      end

      def test_nop
        assert_equal :nop, GpuCore.nop.opcode
      end

      def test_halt
        assert_equal :halt, GpuCore.halt.opcode
      end
    end
  end
end
