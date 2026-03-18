# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module JvmSimulator
    class TestEncoding < Minitest::Test
      def test_encode_iconst_small
        assert_equal "\x03".b, JvmSimulator.encode_iconst(0)
        assert_equal "\x04".b, JvmSimulator.encode_iconst(1)
        assert_equal "\x08".b, JvmSimulator.encode_iconst(5)
      end

      def test_encode_iconst_bipush
        assert_equal "\x10\x2A".b, JvmSimulator.encode_iconst(42)
      end

      def test_encode_iconst_negative
        bytes = JvmSimulator.encode_iconst(-1)
        assert_equal 2, bytes.bytesize
        assert_equal BIPUSH, bytes.getbyte(0)
      end

      def test_encode_iconst_out_of_range
        assert_raises(ArgumentError) { JvmSimulator.encode_iconst(200) }
      end

      def test_encode_istore
        assert_equal "\x3B".b, JvmSimulator.encode_istore(0)
        assert_equal "\x3E".b, JvmSimulator.encode_istore(3)
        assert_equal "\x36\x05".b, JvmSimulator.encode_istore(5)
      end

      def test_encode_iload
        assert_equal "\x1A".b, JvmSimulator.encode_iload(0)
        assert_equal "\x1D".b, JvmSimulator.encode_iload(3)
        assert_equal "\x15\x05".b, JvmSimulator.encode_iload(5)
      end

      def test_assemble_jvm
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_1], [ICONST_2], [IADD], [ISTORE_0], [RETURN]
        )
        assert_equal "\x04\x05\x60\x3B\xB1".b, bytecode
      end

      def test_assemble_two_byte
        bytecode = JvmSimulator.assemble_jvm([BIPUSH, 42])
        assert_equal "\x10\x2A".b, bytecode
      end

      def test_assemble_three_byte
        bytecode = JvmSimulator.assemble_jvm([GOTO, 5])
        assert_equal 3, bytecode.bytesize
      end

      def test_assemble_missing_operand
        assert_raises(ArgumentError) { JvmSimulator.assemble_jvm([BIPUSH]) }
        assert_raises(ArgumentError) { JvmSimulator.assemble_jvm([GOTO]) }
      end

      def test_assemble_unknown
        assert_raises(ArgumentError) { JvmSimulator.assemble_jvm([0xFF]) }
      end
    end

    class TestJVMSimulator < Minitest::Test
      def test_x_equals_1_plus_2
        sim = JVMSimulator.new
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_1], [ICONST_2], [IADD], [ISTORE_0], [RETURN]
        )
        sim.load(bytecode)
        traces = sim.run
        assert_equal 5, traces.size
        assert_equal 3, sim.locals[0]
        assert sim.halted
      end

      def test_iconst_all_values
        sim = JVMSimulator.new
        (0..5).each do |n|
          sim.load([ICONST_0 + n, RETURN].pack("CC"))
          sim.run
          # Just verify it doesn't crash
        end
      end

      def test_bipush
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([BIPUSH, 42], [ISTORE_0], [RETURN]))
        sim.run
        assert_equal 42, sim.locals[0]
      end

      def test_bipush_negative
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([BIPUSH, -1], [ISTORE_0], [RETURN]))
        sim.run
        assert_equal(-1, sim.locals[0])
      end

      def test_ldc
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([LDC, 0], [ISTORE_0], [RETURN]),
          constants: [999])
        sim.run
        assert_equal 999, sim.locals[0]
      end

      def test_ldc_out_of_range
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([LDC, 5], [RETURN]), constants: [1])
        assert_raises(RuntimeError) { sim.run }
      end

      def test_ldc_not_integer
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([LDC, 0], [RETURN]), constants: ["hello"])
        assert_raises(RuntimeError) { sim.run }
      end

      def test_iload_and_istore
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_5], [ISTORE_0], [ILOAD_0], [ISTORE_1], [RETURN]
        ))
        sim.run
        assert_equal 5, sim.locals[0]
        assert_equal 5, sim.locals[1]
      end

      def test_iload_generic
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_3], [ISTORE, 5], [ILOAD, 5], [ISTORE_0], [RETURN]
        ))
        sim.run
        assert_equal 3, sim.locals[0]
        assert_equal 3, sim.locals[5]
      end

      def test_iload_uninitialized
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ILOAD_0], [RETURN]))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_isub
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_5], [ICONST_3], [ISUB], [ISTORE_0], [RETURN]
        ))
        sim.run
        assert_equal 2, sim.locals[0]
      end

      def test_imul
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_3], [ICONST_4], [IMUL], [ISTORE_0], [RETURN]
        ))
        sim.run
        assert_equal 12, sim.locals[0]
      end

      def test_idiv
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [BIPUSH, 10], [ICONST_3], [IDIV], [ISTORE_0], [RETURN]
        ))
        sim.run
        assert_equal 3, sim.locals[0]
      end

      def test_idiv_by_zero
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_5], [ICONST_0], [IDIV], [RETURN]
        ))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_goto
        sim = JVMSimulator.new
        # goto +3 skips to RETURN (goto is 3 bytes, so +3 lands right after it)
        sim.load(JvmSimulator.assemble_jvm(
          [GOTO, 5], [ICONST_1], [ISTORE_0], [RETURN]
        ))
        # goto at PC=0 with offset 5 -> jumps to PC=5 which is RETURN
        sim.run
        assert_nil sim.locals[0] # iconst_1 and istore_0 were skipped
      end

      def test_if_icmpeq_taken
        sim = JVMSimulator.new
        # Push 1, 1, if_icmpeq jumps over iconst_5
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_1], [ICONST_1], [IF_ICMPEQ, 5], [ICONST_5], [ISTORE_0], [RETURN]
        )
        sim.load(bytecode)
        sim.run
        # if_icmpeq at PC=2 with offset 5 -> target = 2+5 = 7 which is RETURN
        assert_nil sim.locals[0]
      end

      def test_if_icmpeq_not_taken
        sim = JVMSimulator.new
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_1], [ICONST_2], [IF_ICMPEQ, 5], [ICONST_5], [ISTORE_0], [RETURN]
        )
        sim.load(bytecode)
        sim.run
        assert_equal 5, sim.locals[0]
      end

      def test_if_icmpgt_taken
        sim = JVMSimulator.new
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_5], [ICONST_1], [IF_ICMPGT, 5], [ICONST_0], [ISTORE_0], [RETURN]
        )
        sim.load(bytecode)
        sim.run
        assert_nil sim.locals[0]
      end

      def test_if_icmpgt_not_taken
        sim = JVMSimulator.new
        bytecode = JvmSimulator.assemble_jvm(
          [ICONST_1], [ICONST_5], [IF_ICMPGT, 5], [ICONST_3], [ISTORE_0], [RETURN]
        )
        sim.load(bytecode)
        sim.run
        assert_equal 3, sim.locals[0]
      end

      def test_ireturn
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ICONST_5], [IRETURN]))
        sim.run
        assert_equal 5, sim.return_value
        assert sim.halted
      end

      def test_ireturn_empty_stack
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([IRETURN]))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_halted_raises
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([RETURN]))
        sim.run
        assert_raises(RuntimeError) { sim.step }
      end

      def test_pc_past_end
        sim = JVMSimulator.new
        sim.load("".b)
        assert_raises(RuntimeError) { sim.step }
      end

      def test_unknown_opcode
        sim = JVMSimulator.new
        sim.load("\xFF".b)
        assert_raises(RuntimeError) { sim.step }
      end

      def test_stack_underflow_binary_op
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ICONST_1], [IADD]))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_stack_underflow_istore
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ISTORE_0]))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_stack_underflow_if_icmp
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ICONST_1], [IF_ICMPEQ, 3]))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_i32_overflow
        sim = JVMSimulator.new
        # 2^31 - 1 + 1 should wrap to -2^31
        sim.load(JvmSimulator.assemble_jvm(
          [LDC, 0], [ICONST_1], [IADD], [IRETURN]
        ), constants: [2_147_483_647])
        sim.run
        assert_equal(-2_147_483_648, sim.return_value)
      end

      def test_trace_fields
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ICONST_1], [RETURN]))
        trace = sim.step
        assert_equal 0, trace.pc
        assert_equal "iconst_1", trace.opcode
        assert_equal [], trace.stack_before
        assert_equal [1], trace.stack_after
      end

      def test_iload_all_shortcuts
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm(
          [ICONST_1], [ISTORE_0],
          [ICONST_2], [ISTORE_1],
          [ICONST_3], [ISTORE_2],
          [ICONST_4], [ISTORE_3],
          [ILOAD_0], [ILOAD_1], [ILOAD_2], [ILOAD_3],
          [RETURN]
        ))
        sim.run
        assert_equal [1, 2, 3, 4], sim.stack
      end

      def test_idiv_stack_underflow
        sim = JVMSimulator.new
        sim.load(JvmSimulator.assemble_jvm([ICONST_1], [IDIV]))
        assert_raises(RuntimeError) { sim.run }
      end
    end
  end
end
