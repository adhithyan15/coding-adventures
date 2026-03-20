# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Tests for the GenericISA instruction set implementation.
    # =========================================================================
    #
    # The GenericISA is the default pluggable ISA -- a vendor-neutral set of
    # 16 opcodes that can express any floating-point program.

    class TestGenericISAProtocol < Minitest::Test
      # GenericISA responds to the expected duck-type interface.
      def test_responds_to_name
        isa = GenericISA.new
        assert_respond_to isa, :name
      end

      def test_responds_to_execute
        isa = GenericISA.new
        assert_respond_to isa, :execute
      end

      def test_has_name
        assert_equal "Generic", GenericISA.new.name
      end
    end

    class TestISAArithmetic < Minitest::Test
      def setup
        @isa = GenericISA.new
        @regs = FPRegisterFile.new
        @mem = LocalMemory.new
      end

      def test_fadd
        @regs.write_float(0, 1.0)
        @regs.write_float(1, 2.0)
        result = @isa.execute(GpuCore.fadd(2, 0, 1), @regs, @mem)
        assert_equal 3.0, @regs.read_float(2)
        assert_equal({"R2" => 3.0}, result.registers_changed)
      end

      def test_fadd_negative
        @regs.write_float(0, 1.0)
        @regs.write_float(1, -3.0)
        @isa.execute(GpuCore.fadd(2, 0, 1), @regs, @mem)
        assert_equal(-2.0, @regs.read_float(2))
      end

      def test_fsub
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 3.0)
        @isa.execute(GpuCore.fsub(2, 0, 1), @regs, @mem)
        assert_equal 2.0, @regs.read_float(2)
      end

      def test_fmul
        @regs.write_float(0, 3.0)
        @regs.write_float(1, 4.0)
        @isa.execute(GpuCore.fmul(2, 0, 1), @regs, @mem)
        assert_equal 12.0, @regs.read_float(2)
      end

      def test_fmul_by_zero
        @regs.write_float(0, 42.0)
        @regs.write_float(1, 0.0)
        @isa.execute(GpuCore.fmul(2, 0, 1), @regs, @mem)
        assert_equal 0.0, @regs.read_float(2)
      end

      # FMA: Rd = Rs1 * Rs2 + Rs3 = 2.0 * 3.0 + 1.0 = 7.0.
      def test_ffma
        @regs.write_float(0, 2.0)
        @regs.write_float(1, 3.0)
        @regs.write_float(2, 1.0)
        result = @isa.execute(GpuCore.ffma(3, 0, 1, 2), @regs, @mem)
        assert_equal 7.0, @regs.read_float(3)
        assert_includes result.registers_changed, "R3"
      end

      def test_fneg
        @regs.write_float(0, 5.0)
        @isa.execute(GpuCore.fneg(1, 0), @regs, @mem)
        assert_equal(-5.0, @regs.read_float(1))
      end

      # Negating twice returns to original.
      def test_fneg_double
        @regs.write_float(0, 3.0)
        @isa.execute(GpuCore.fneg(1, 0), @regs, @mem)
        @isa.execute(GpuCore.fneg(2, 1), @regs, @mem)
        assert_equal 3.0, @regs.read_float(2)
      end

      def test_fabs_positive
        @regs.write_float(0, 5.0)
        @isa.execute(GpuCore.fabs(1, 0), @regs, @mem)
        assert_equal 5.0, @regs.read_float(1)
      end

      def test_fabs_negative
        @regs.write_float(0, -5.0)
        @isa.execute(GpuCore.fabs(1, 0), @regs, @mem)
        assert_equal 5.0, @regs.read_float(1)
      end
    end

    class TestISAMemory < Minitest::Test
      def setup
        @isa = GenericISA.new
        @regs = FPRegisterFile.new
        @mem = LocalMemory.new
      end

      # Store a value then load it back.
      def test_store_and_load
        @regs.write_float(0, 0.0) # base address = 0
        @regs.write_float(1, 3.14) # value to store
        @isa.execute(GpuCore.store(0, 1, 0.0), @regs, @mem)
        @isa.execute(GpuCore.load(2, 0, 0.0), @regs, @mem)
        assert_in_delta 3.14, @regs.read_float(2), 1e-5
      end

      # Store with a non-zero offset.
      def test_store_with_offset
        @regs.write_float(0, 0.0) # base = 0
        @regs.write_float(1, 42.0)
        @isa.execute(GpuCore.store(0, 1, 8.0), @regs, @mem) # store at address 8
        @isa.execute(GpuCore.load(2, 0, 8.0), @regs, @mem)  # load from address 8
        assert_equal 42.0, @regs.read_float(2)
      end

      # Store returns memory_changed in result.
      def test_store_result_description
        @regs.write_float(0, 0.0)
        @regs.write_float(1, 5.0)
        result = @isa.execute(GpuCore.store(0, 1, 0.0), @regs, @mem)
        refute_nil result.memory_changed
        assert_includes result.memory_changed, 0
      end

      # Load returns registers_changed in result.
      def test_load_result_description
        @mem.store_ruby_float(0, 7.0)
        @regs.write_float(0, 0.0)
        result = @isa.execute(GpuCore.load(1, 0, 0.0), @regs, @mem)
        refute_nil result.registers_changed
        assert_includes result.registers_changed, "R1"
      end
    end

    class TestISADataMovement < Minitest::Test
      def setup
        @isa = GenericISA.new
        @regs = FPRegisterFile.new
        @mem = LocalMemory.new
      end

      def test_mov
        @regs.write_float(0, 42.0)
        @isa.execute(GpuCore.mov(1, 0), @regs, @mem)
        assert_equal 42.0, @regs.read_float(1)
      end

      def test_limm
        @isa.execute(GpuCore.limm(0, 3.14), @regs, @mem)
        assert_in_delta 3.14, @regs.read_float(0), 0.01
      end

      def test_limm_negative
        @isa.execute(GpuCore.limm(0, -99.0), @regs, @mem)
        assert_equal(-99.0, @regs.read_float(0))
      end

      def test_limm_zero
        @isa.execute(GpuCore.limm(0, 0.0), @regs, @mem)
        assert_equal 0.0, @regs.read_float(0)
      end
    end

    class TestISAControlFlow < Minitest::Test
      def setup
        @isa = GenericISA.new
        @regs = FPRegisterFile.new
        @mem = LocalMemory.new
      end

      # BEQ branches when registers are equal.
      def test_beq_taken
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 5.0)
        result = @isa.execute(GpuCore.beq(0, 1, 3), @regs, @mem)
        assert_equal 3, result.next_pc_offset
      end

      # BEQ falls through when registers differ.
      def test_beq_not_taken
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 3.0)
        result = @isa.execute(GpuCore.beq(0, 1, 3), @regs, @mem)
        assert_equal 1, result.next_pc_offset
      end

      # BLT branches when Rs1 < Rs2.
      def test_blt_taken
        @regs.write_float(0, 2.0)
        @regs.write_float(1, 5.0)
        result = @isa.execute(GpuCore.blt(0, 1, 4), @regs, @mem)
        assert_equal 4, result.next_pc_offset
      end

      # BLT falls through when Rs1 >= Rs2.
      def test_blt_not_taken
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 2.0)
        result = @isa.execute(GpuCore.blt(0, 1, 4), @regs, @mem)
        assert_equal 1, result.next_pc_offset
      end

      # BNE branches when registers differ.
      def test_bne_taken
        @regs.write_float(0, 1.0)
        @regs.write_float(1, 2.0)
        result = @isa.execute(GpuCore.bne(0, 1, 2), @regs, @mem)
        assert_equal 2, result.next_pc_offset
      end

      # BNE falls through when registers are equal.
      def test_bne_not_taken
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 5.0)
        result = @isa.execute(GpuCore.bne(0, 1, 2), @regs, @mem)
        assert_equal 1, result.next_pc_offset
      end

      # JMP sets absolute PC.
      def test_jmp
        result = @isa.execute(GpuCore.jmp(10), @regs, @mem)
        assert_equal 10, result.next_pc_offset
        assert result.absolute_jump
      end

      # NOP does nothing but advance PC.
      def test_nop
        result = @isa.execute(GpuCore.nop, @regs, @mem)
        assert_equal 1, result.next_pc_offset
        refute result.halted
      end

      # HALT sets the halted flag.
      def test_halt
        result = @isa.execute(GpuCore.halt, @regs, @mem)
        assert result.halted
      end
    end

    class TestISADescriptions < Minitest::Test
      def setup
        @isa = GenericISA.new
        @regs = FPRegisterFile.new
        @mem = LocalMemory.new
      end

      def test_fadd_description
        @regs.write_float(0, 1.0)
        @regs.write_float(1, 2.0)
        result = @isa.execute(GpuCore.fadd(2, 0, 1), @regs, @mem)
        assert_includes result.description, "1.0"
        assert_includes result.description, "2.0"
        assert_includes result.description, "3.0"
      end

      def test_ffma_description
        @regs.write_float(0, 2.0)
        @regs.write_float(1, 3.0)
        @regs.write_float(2, 1.0)
        result = @isa.execute(GpuCore.ffma(3, 0, 1, 2), @regs, @mem)
        assert_includes result.description, "7.0"
      end

      def test_branch_description_taken
        @regs.write_float(0, 5.0)
        @regs.write_float(1, 5.0)
        result = @isa.execute(GpuCore.beq(0, 1, 3), @regs, @mem)
        assert_includes result.description.downcase, "branch"
      end

      def test_branch_description_not_taken
        @regs.write_float(0, 1.0)
        @regs.write_float(1, 2.0)
        result = @isa.execute(GpuCore.beq(0, 1, 3), @regs, @mem)
        assert_includes result.description.downcase, "fall through"
      end
    end
  end
end
