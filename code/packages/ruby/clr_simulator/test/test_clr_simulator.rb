# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module ClrSimulator
    class TestEncoding < Minitest::Test
      def test_encode_ldc_i4_short
        assert_equal "\x16".b, ClrSimulator.encode_ldc_i4(0)
        assert_equal "\x1B".b, ClrSimulator.encode_ldc_i4(5)
        assert_equal "\x1E".b, ClrSimulator.encode_ldc_i4(8)
      end

      def test_encode_ldc_i4_s
        bytes = ClrSimulator.encode_ldc_i4(42)
        assert_equal 2, bytes.bytesize
        assert_equal LDC_I4_S, bytes.getbyte(0)
      end

      def test_encode_ldc_i4_s_negative
        bytes = ClrSimulator.encode_ldc_i4(-1)
        assert_equal 2, bytes.bytesize
        assert_equal LDC_I4_S, bytes.getbyte(0)
      end

      def test_encode_ldc_i4_large
        bytes = ClrSimulator.encode_ldc_i4(1000)
        assert_equal 5, bytes.bytesize
        assert_equal LDC_I4, bytes.getbyte(0)
      end

      def test_encode_stloc
        assert_equal "\x0A".b, ClrSimulator.encode_stloc(0)
        assert_equal "\x0D".b, ClrSimulator.encode_stloc(3)
        assert_equal "\x13\x0A".b, ClrSimulator.encode_stloc(10)
      end

      def test_encode_ldloc
        assert_equal "\x06".b, ClrSimulator.encode_ldloc(0)
        assert_equal "\x09".b, ClrSimulator.encode_ldloc(3)
        assert_equal "\x11\x0A".b, ClrSimulator.encode_ldloc(10)
      end

      def test_assemble_clr
        bytecode = ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1),
          [RET]
        )
        assert_equal "\x17\x2A".b, bytecode
      end

      def test_assemble_clr_raw_bytes
        bytecode = ClrSimulator.assemble_clr("\x17".b, [RET])
        assert_equal "\x17\x2A".b, bytecode
      end
    end

    class TestCLRSimulator < Minitest::Test
      def test_x_equals_1_plus_2
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1),
          ClrSimulator.encode_ldc_i4(2),
          [ADD],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        traces = sim.run
        assert_equal 5, traces.size
        assert_equal 3, sim.locals[0]
        assert sim.halted
      end

      def test_ldc_i4_all_short_forms
        sim = CLRSimulator.new
        (0..8).each do |n|
          sim.load(ClrSimulator.assemble_clr(
            ClrSimulator.encode_ldc_i4(n),
            ClrSimulator.encode_stloc(0),
            [RET]
          ))
          sim.run
          assert_equal n, sim.locals[0]
        end
      end

      def test_ldc_i4_s
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(42),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 42, sim.locals[0]
      end

      def test_ldc_i4_s_negative
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(-5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal(-5, sim.locals[0])
      end

      def test_ldc_i4_large
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1000),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 1000, sim.locals[0]
      end

      def test_ldloc_stloc_generic
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(7),
          ClrSimulator.encode_stloc(5),
          ClrSimulator.encode_ldloc(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 7, sim.locals[0]
        assert_equal 7, sim.locals[5]
      end

      def test_ldloc_uninitialized
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldloc(0),
          [RET]
        ))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_ldloc_s_uninitialized
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          [LDLOC_S, 5],
          [RET]
        ))
        assert_raises(RuntimeError) { sim.run }
      end

      def test_sub
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_ldc_i4(3),
          [SUB],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 2, sim.locals[0]
      end

      def test_mul
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(3),
          ClrSimulator.encode_ldc_i4(4),
          [MUL],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 12, sim.locals[0]
      end

      def test_div
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(10),
          ClrSimulator.encode_ldc_i4(3),
          [DIV],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 3, sim.locals[0]
      end

      def test_div_by_zero
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_ldc_i4(0),
          [DIV],
          [RET]
        ))
        assert_raises(ZeroDivisionError) { sim.run }
      end

      def test_nop
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr([NOP], [RET]))
        traces = sim.run
        assert_equal 2, traces.size
        assert_equal "nop", traces[0].opcode
      end

      def test_ldnull
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr([LDNULL], ClrSimulator.encode_stloc(0), [RET]))
        sim.run
        assert_nil sim.locals[0]
      end

      def test_br_s
        sim = CLRSimulator.new
        # br.s +2 at PC=0 -> next_pc=2, target=2+2=4 which is RET
        # Skip over ldc.i4.5 (1 byte) and stloc.0 (1 byte)
        sim.load(ClrSimulator.assemble_clr(
          [BR_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_nil sim.locals[0]
      end

      def test_brfalse_s_taken
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(0),
          [BRFALSE_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_nil sim.locals[0]
      end

      def test_brfalse_s_not_taken
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1),
          [BRFALSE_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 5, sim.locals[0]
      end

      def test_brtrue_s_taken
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1),
          [BRTRUE_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_nil sim.locals[0]
      end

      def test_brtrue_s_not_taken
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(0),
          [BRTRUE_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 5, sim.locals[0]
      end

      def test_brfalse_s_null
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          [LDNULL],
          [BRFALSE_S, 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        # null is treated as 0/false -> branch taken
        assert_nil sim.locals[0]
      end

      def test_ceq_equal
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(3),
          ClrSimulator.encode_ldc_i4(3),
          [PREFIX_FE, CEQ_BYTE],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 1, sim.locals[0]
      end

      def test_ceq_not_equal
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(3),
          ClrSimulator.encode_ldc_i4(5),
          [PREFIX_FE, CEQ_BYTE],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 0, sim.locals[0]
      end

      def test_cgt
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_ldc_i4(3),
          [PREFIX_FE, CGT_BYTE],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 1, sim.locals[0]
      end

      def test_clt
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(2),
          ClrSimulator.encode_ldc_i4(5),
          [PREFIX_FE, CLT_BYTE],
          ClrSimulator.encode_stloc(0),
          [RET]
        ))
        sim.run
        assert_equal 1, sim.locals[0]
      end

      def test_unknown_two_byte_opcode
        sim = CLRSimulator.new
        sim.load([PREFIX_FE, 0xFF, RET].pack("CCC"))
        assert_raises(ArgumentError) { sim.run }
      end

      def test_incomplete_two_byte_opcode
        sim = CLRSimulator.new
        sim.load([PREFIX_FE].pack("C"))
        assert_raises(ArgumentError) { sim.run }
      end

      def test_halted_raises
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr([RET]))
        sim.run
        assert_raises(RuntimeError) { sim.step }
      end

      def test_pc_past_end
        sim = CLRSimulator.new
        sim.load("".b)
        assert_raises(RuntimeError) { sim.step }
      end

      def test_unknown_opcode
        sim = CLRSimulator.new
        sim.load("\xFF".b)
        assert_raises(ArgumentError) { sim.step }
      end

      def test_trace_fields
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1),
          [RET]
        ))
        trace = sim.step
        assert_equal 0, trace.pc
        assert_equal "ldc.i4.1", trace.opcode
        assert_equal [], trace.stack_before
        assert_equal [1], trace.stack_after
      end

      def test_ldloc_stloc_all_shortcuts
        sim = CLRSimulator.new
        sim.load(ClrSimulator.assemble_clr(
          ClrSimulator.encode_ldc_i4(1), ClrSimulator.encode_stloc(0),
          ClrSimulator.encode_ldc_i4(2), ClrSimulator.encode_stloc(1),
          ClrSimulator.encode_ldc_i4(3), ClrSimulator.encode_stloc(2),
          ClrSimulator.encode_ldc_i4(4), ClrSimulator.encode_stloc(3),
          ClrSimulator.encode_ldloc(0),
          ClrSimulator.encode_ldloc(1),
          ClrSimulator.encode_ldloc(2),
          ClrSimulator.encode_ldloc(3),
          [RET]
        ))
        sim.run
        assert_equal [1, 2, 3, 4], sim.stack
      end
    end
  end
end
