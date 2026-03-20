# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Integration tests -- multi-instruction GPU programs.
    # =========================================================================
    #
    # These tests verify that the GPU core correctly executes complete programs,
    # not just individual instructions. They serve as both tests and examples
    # of what GPU programs look like at the core level.

    class TestSAXPY < Minitest::Test
      # SAXPY: y = a * x + y -- the "hello world" of GPU programming.
      #
      # In real GPU code, SAXPY runs across thousands of threads, each computing
      # one element. Here we simulate what a single thread does: one FMA.

      # y = 2.0 * 3.0 + 1.0 = 7.0.
      def test_saxpy_single_element
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 2.0),     # R0 = a = 2.0
          GpuCore.limm(1, 3.0),     # R1 = x = 3.0
          GpuCore.limm(2, 1.0),     # R2 = y = 1.0
          GpuCore.ffma(3, 0, 1, 2), # R3 = a * x + y = 7.0
          GpuCore.halt
        ])
        traces = core.run
        assert_equal 7.0, core.registers.read_float(3)
        assert_equal 5, traces.length
      end

      # y = 0.0 * x + y = y (alpha=0 means just copy y).
      def test_saxpy_zero_alpha
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 0.0),     # a = 0
          GpuCore.limm(1, 99.0),    # x = 99 (doesn't matter)
          GpuCore.limm(2, 5.0),     # y = 5
          GpuCore.ffma(3, 0, 1, 2), # R3 = 0*99 + 5 = 5
          GpuCore.halt
        ])
        core.run
        assert_equal 5.0, core.registers.read_float(3)
      end
    end

    class TestDotProduct < Minitest::Test
      # Dot product: sum of element-wise products.
      #
      # dot(A, B) = A[0]*B[0] + A[1]*B[1] + A[2]*B[2]
      #
      # This is the fundamental operation in neural networks -- every neuron
      # computes a dot product of its inputs and weights.

      # dot([1,2,3], [4,5,6]) = 4 + 10 + 18 = 32.
      def test_dot_product_3d
        core = GPUCore.new
        core.load_program([
          # Load vector A
          GpuCore.limm(0, 1.0),       # A[0]
          GpuCore.limm(1, 2.0),       # A[1]
          GpuCore.limm(2, 3.0),       # A[2]
          # Load vector B
          GpuCore.limm(3, 4.0),       # B[0]
          GpuCore.limm(4, 5.0),       # B[1]
          GpuCore.limm(5, 6.0),       # B[2]
          # Accumulate with FMA
          GpuCore.limm(6, 0.0),       # acc = 0
          GpuCore.ffma(6, 0, 3, 6),   # acc = 1*4 + 0 = 4
          GpuCore.ffma(6, 1, 4, 6),   # acc = 2*5 + 4 = 14
          GpuCore.ffma(6, 2, 5, 6),   # acc = 3*6 + 14 = 32
          GpuCore.halt
        ])
        core.run
        assert_equal 32.0, core.registers.read_float(6)
      end
    end

    class TestLoop < Minitest::Test
      # Test programs with loops (branches).

      # Sum of 1 + 2 + 3 + 4 = 10 using a loop.
      #
      # Program:
      #     R0 = sum = 0
      #     R1 = i = 1
      #     R2 = increment = 1
      #     R3 = limit = 5
      # loop:
      #     sum += i           (PC=4)
      #     i += increment     (PC=5)
      #     if i < limit: goto loop  (PC=6, branch offset = -2 -> PC=4)
      #     halt               (PC=7)
      def test_sum_1_to_4
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 0.0),   # R0 = sum
          GpuCore.limm(1, 1.0),   # R1 = i
          GpuCore.limm(2, 1.0),   # R2 = 1 (increment)
          GpuCore.limm(3, 5.0),   # R3 = limit
          GpuCore.fadd(0, 0, 1),  # sum += i        (PC=4)
          GpuCore.fadd(1, 1, 2),  # i += 1          (PC=5)
          GpuCore.blt(1, 3, -2),  # if i < 5: back  (PC=6)
          GpuCore.halt            #                  (PC=7)
        ])
        core.run
        assert_equal 10.0, core.registers.read_float(0)
      end

      # Count down from 3 to 0.
      #
      # R0 = counter = 3
      # R1 = decrement = 1
      # R2 = zero = 0
      # loop: counter -= decrement; if counter != zero: loop
      def test_countdown
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 3.0),   # counter
          GpuCore.limm(1, 1.0),   # decrement
          GpuCore.limm(2, 0.0),   # zero
          GpuCore.fsub(0, 0, 1),  # counter -= 1   (PC=3)
          GpuCore.bne(0, 2, -1),  # if counter != 0: back (PC=4)
          GpuCore.halt            #                (PC=5)
        ])
        core.run
        assert_equal 0.0, core.registers.read_float(0)
      end
    end

    class TestMemoryPrograms < Minitest::Test
      # Test programs that use load/store.

      # Store 3 values to memory, load them back, sum them.
      #
      # This simulates a GPU thread loading input data from memory,
      # computing on it, and writing the result back.
      def test_store_and_load_array
        core = GPUCore.new
        # Pre-store some values in memory
        core.memory.store_ruby_float(0, 10.0)
        core.memory.store_ruby_float(4, 20.0)
        core.memory.store_ruby_float(8, 30.0)

        core.load_program([
          GpuCore.limm(10, 0.0),      # R10 = base address
          GpuCore.load(0, 10, 0.0),   # R0 = Mem[0] = 10.0
          GpuCore.load(1, 10, 4.0),   # R1 = Mem[4] = 20.0
          GpuCore.load(2, 10, 8.0),   # R2 = Mem[8] = 30.0
          GpuCore.fadd(3, 0, 1),      # R3 = 10 + 20 = 30
          GpuCore.fadd(3, 3, 2),      # R3 = 30 + 30 = 60
          GpuCore.store(10, 3, 12.0), # Mem[12] = 60.0
          GpuCore.halt
        ])
        core.run
        assert_equal 60.0, core.registers.read_float(3)
        assert_equal 60.0, core.memory.load_float_as_ruby(12)
      end
    end

    class TestConditional < Minitest::Test
      # Test conditional execution patterns.

      # Compute max(a, b) using a branch.
      #
      # if a < b:
      #     result = b
      # else:
      #     result = a
      def test_max_of_two
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 3.0),   # R0 = a
          GpuCore.limm(1, 7.0),   # R1 = b
          GpuCore.blt(0, 1, 2),   # if a < b: skip to "result = b"  (PC=2)
          GpuCore.mov(2, 0),      # result = a                       (PC=3)
          GpuCore.jmp(5),         # skip "result = b"                (PC=4)
          GpuCore.mov(2, 1),      # result = b                       (PC=5)
          GpuCore.halt            #                                  (PC=6)
        ])
        core.run
        assert_equal 7.0, core.registers.read_float(2)
      end

      # max(7, 3) = 7 -- takes the else branch.
      def test_max_reversed
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 7.0),
          GpuCore.limm(1, 3.0),
          GpuCore.blt(0, 1, 2),   # 7 < 3? No -> fall through
          GpuCore.mov(2, 0),      # result = a = 7
          GpuCore.jmp(6),         # skip "result = b"
          GpuCore.mov(2, 1),      # skipped
          GpuCore.halt            # PC=6
        ])
        core.run
        assert_equal 7.0, core.registers.read_float(2)
      end
    end

    class TestPrecisionModes < Minitest::Test
      # Test with different floating-point formats.

      # Run a program in FP16 mode.
      def test_fp16_execution
        core = GPUCore.new(fmt: FpArithmetic::FP16)
        core.load_program([
          GpuCore.limm(0, 1.0),
          GpuCore.limm(1, 2.0),
          GpuCore.fadd(2, 0, 1),
          GpuCore.halt
        ])
        core.run
        assert_equal 3.0, core.registers.read_float(2)
      end

      # Run a program in BF16 mode.
      def test_bf16_execution
        core = GPUCore.new(fmt: FpArithmetic::BF16)
        core.load_program([
          GpuCore.limm(0, 4.0),
          GpuCore.limm(1, 5.0),
          GpuCore.fmul(2, 0, 1),
          GpuCore.halt
        ])
        core.run
        assert_equal 20.0, core.registers.read_float(2)
      end
    end

    class TestEdgeCases < Minitest::Test
      # Test edge cases and error conditions.

      # A program of only NOPs and HALT.
      def test_nop_program
        core = GPUCore.new
        core.load_program([GpuCore.nop, GpuCore.nop, GpuCore.nop, GpuCore.halt])
        traces = core.run
        assert_equal 4, traces.length
        assert core.halted?
      end

      # An instruction can read and write the same register.
      def test_self_modifying_register
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 5.0),
          GpuCore.fadd(0, 0, 0), # R0 = R0 + R0 = 10.0
          GpuCore.halt
        ])
        core.run
        assert_equal 10.0, core.registers.read_float(0)
      end

      # Use high-numbered registers (NVIDIA-scale).
      def test_large_register_index
        core = GPUCore.new(num_registers: 256)
        core.load_program([
          GpuCore.limm(200, 42.0),
          GpuCore.limm(255, 1.0),
          GpuCore.fadd(254, 200, 255),
          GpuCore.halt
        ])
        core.run
        assert_equal 43.0, core.registers.read_float(254)
      end

      # BEQ with offset 0 creates an infinite loop (caught by max_steps).
      def test_beq_with_zero_offset
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 1.0),
          GpuCore.limm(1, 1.0),
          GpuCore.beq(0, 1, 0), # infinite: jump to self
          GpuCore.halt
        ])
        err = assert_raises(RuntimeError) { core.run(max_steps: 50) }
        assert_match(/Execution limit/, err.message)
      end
    end
  end
end
