# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Intel4004Simulator
    class TestIntel4004 < Minitest::Test
      def test_x_equals_1_plus_2
        sim = Intel4004Sim.new
        # LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
        program = [0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01].pack("C*")
        traces = sim.run(program)

        assert_equal 6, traces.size
        assert_equal 3, sim.registers[1]
        assert sim.halted
      end

      def test_ldm
        sim = Intel4004Sim.new
        sim.load_program([0xD5, 0x01].pack("C*"))
        trace = sim.step
        assert_equal "LDM 5", trace.mnemonic
        assert_equal 0, trace.accumulator_before
        assert_equal 5, trace.accumulator_after
      end

      def test_xch
        sim = Intel4004Sim.new
        sim.load_program([0xD7, 0xB3, 0x01].pack("C*"))
        sim.step # LDM 7 -> A=7
        trace = sim.step # XCH R3 -> R3=7, A=0
        assert_equal "XCH R3", trace.mnemonic
        assert_equal 0, sim.accumulator
        assert_equal 7, sim.registers[3]
      end

      def test_add_no_carry
        sim = Intel4004Sim.new
        sim.load_program([0xD3, 0xB0, 0xD2, 0x80, 0x01].pack("C*"))
        sim.step # LDM 3 -> A=3
        sim.step # XCH R0 -> R0=3, A=0
        sim.step # LDM 2 -> A=2
        trace = sim.step # ADD R0 -> A=2+3=5
        assert_equal "ADD R0", trace.mnemonic
        assert_equal 5, sim.accumulator
        refute sim.carry
      end

      def test_add_with_carry
        sim = Intel4004Sim.new
        # 15 + 1 = 16 -> wraps to 0 with carry
        sim.load_program([0xDF, 0xB0, 0xD1, 0x80, 0x01].pack("C*"))
        sim.step # LDM 15
        sim.step # XCH R0 -> R0=15
        sim.step # LDM 1
        sim.step # ADD R0 -> 1+15=16, carry=true, A=0
        assert_equal 0, sim.accumulator
        assert sim.carry
      end

      def test_sub_no_borrow
        sim = Intel4004Sim.new
        sim.load_program([0xD2, 0xB0, 0xD5, 0x90, 0x01].pack("C*"))
        sim.step # LDM 2
        sim.step # XCH R0 -> R0=2
        sim.step # LDM 5
        sim.step # SUB R0 -> 5-2=3
        assert_equal 3, sim.accumulator
        refute sim.carry
      end

      def test_sub_with_borrow
        sim = Intel4004Sim.new
        sim.load_program([0xD3, 0xB0, 0xD1, 0x90, 0x01].pack("C*"))
        sim.step # LDM 3
        sim.step # XCH R0 -> R0=3
        sim.step # LDM 1
        sim.step # SUB R0 -> 1-3 = -2 & 0xF = 14, carry=true
        assert_equal 14, sim.accumulator
        assert sim.carry
      end

      def test_halted_raises
        sim = Intel4004Sim.new
        sim.run([0x01].pack("C*"))
        assert_raises(RuntimeError) { sim.step }
      end

      def test_trace_fields
        sim = Intel4004Sim.new
        sim.load_program([0xD1, 0x01].pack("C*"))
        trace = sim.step
        assert_equal 0, trace.address
        assert_equal 0xD1, trace.raw
        assert_equal "LDM 1", trace.mnemonic
        assert_equal false, trace.carry_before
        assert_equal false, trace.carry_after
      end

      def test_unknown_instruction
        sim = Intel4004Sim.new
        sim.load_program([0xF0, 0x01].pack("C*"))
        trace = sim.step
        assert_includes trace.mnemonic, "UNKNOWN"
      end

      def test_max_steps
        sim = Intel4004Sim.new
        # Program that never halts (all LDM 0 instructions)
        sim.load_program(([0xD0] * 100).pack("C*"))
        traces = sim.run(([0xD0] * 100).pack("C*"), max_steps: 5)
        assert_equal 5, traces.size
      end

      def test_4bit_masking
        sim = Intel4004Sim.new
        # LDM uses only lower 4 bits, so 0xDF loads 15
        sim.load_program([0xDF, 0x01].pack("C*"))
        sim.step
        assert_equal 15, sim.accumulator
      end
    end
  end
end
