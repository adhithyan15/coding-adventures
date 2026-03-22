# frozen_string_literal: true

# Tests for the Intel 4004 gate-level simulator.
#
# These tests verify that every instruction works correctly when routed
# through real logic gates. The test structure mirrors the behavioral
# simulator's tests -- same programs, same expected results.

require "test_helper"

module CodingAdventures
  module Intel4004Gatelevel
    # ===================================================================
    # Basic instructions
    # ===================================================================

    class TestNOP < Minitest::Test
      def test_nop_does_nothing
        cpu = Intel4004GateLevel.new
        traces = cpu.run([0x00, 0x01])
        assert_equal 0, cpu.accumulator
        assert_equal "NOP", traces[0].mnemonic
      end

      def test_multiple_nops
        cpu = Intel4004GateLevel.new
        traces = cpu.run([0x00, 0x00, 0x00, 0x01])
        assert_equal 4, traces.length
      end
    end

    class TestHLT < Minitest::Test
      def test_hlt_stops
        cpu = Intel4004GateLevel.new
        traces = cpu.run([0x01])
        assert cpu.halted?
        assert_equal 1, traces.length
      end
    end

    class TestLDM < Minitest::Test
      def test_ldm_values
        16.times do |n|
          cpu = Intel4004GateLevel.new
          cpu.run([0xD0 | n, 0x01])
          assert_equal n, cpu.accumulator, "LDM #{n} failed"
        end
      end
    end

    class TestLD < Minitest::Test
      def test_ld_reads_register
        cpu = Intel4004GateLevel.new
        cpu.run([0xD7, 0xB0, 0xA0, 0x01]) # LDM 7, XCH R0, LD R0
        assert_equal 7, cpu.accumulator
      end
    end

    class TestXCH < Minitest::Test
      def test_xch_swaps
        cpu = Intel4004GateLevel.new
        cpu.run([0xD7, 0xB0, 0x01])
        assert_equal 7, cpu.registers[0]
        assert_equal 0, cpu.accumulator
      end
    end

    class TestINC < Minitest::Test
      def test_inc_wraps
        cpu = Intel4004GateLevel.new
        cpu.run([0xDF, 0xB0, 0x60, 0x01]) # LDM 15, XCH R0, INC R0
        assert_equal 0, cpu.registers[0]
      end

      def test_inc_no_carry
        cpu = Intel4004GateLevel.new
        # Set carry, then INC -- carry should stay
        cpu.run([0xDF, 0xB1, 0xDF, 0x81, 0x60, 0x01])
        assert cpu.carry
      end
    end

    # ===================================================================
    # Arithmetic
    # ===================================================================

    class TestADD < Minitest::Test
      def test_add_basic
        cpu = Intel4004GateLevel.new
        cpu.run([0xD3, 0xB0, 0xD2, 0x80, 0x01])
        assert_equal 5, cpu.accumulator
        refute cpu.carry
      end

      def test_add_overflow
        cpu = Intel4004GateLevel.new
        cpu.run([0xD1, 0xB0, 0xDF, 0x80, 0x01])
        assert_equal 0, cpu.accumulator
        assert cpu.carry
      end

      def test_add_carry_in
        cpu = Intel4004GateLevel.new
        cpu.run([
          0xDF, 0xB0, 0xDF, 0x80, # 15+15 -> carry=1
          0xD1, 0xB1, 0xD1, 0x81, # 1+1+carry = 3
          0x01
        ])
        assert_equal 3, cpu.accumulator
      end
    end

    class TestSUB < Minitest::Test
      def test_sub_basic
        cpu = Intel4004GateLevel.new
        cpu.run([0xD3, 0xB0, 0xD5, 0x90, 0x01])
        assert_equal 2, cpu.accumulator
        assert cpu.carry
      end

      def test_sub_underflow
        cpu = Intel4004GateLevel.new
        cpu.run([0xD1, 0xB0, 0xD0, 0x90, 0x01])
        assert_equal 15, cpu.accumulator
        refute cpu.carry
      end
    end

    # ===================================================================
    # Accumulator operations
    # ===================================================================

    class TestAccumOps < Minitest::Test
      def test_clb
        cpu = Intel4004GateLevel.new
        cpu.run([0xDF, 0xB0, 0xDF, 0x80, 0xF0, 0x01])
        assert_equal 0, cpu.accumulator
        refute cpu.carry
      end

      def test_clc
        cpu = Intel4004GateLevel.new
        cpu.run([0xDF, 0xB0, 0xDF, 0x80, 0xF1, 0x01])
        refute cpu.carry
      end

      def test_iac
        cpu = Intel4004GateLevel.new
        cpu.run([0xD5, 0xF2, 0x01])
        assert_equal 6, cpu.accumulator
      end

      def test_iac_overflow
        cpu = Intel4004GateLevel.new
        cpu.run([0xDF, 0xF2, 0x01])
        assert_equal 0, cpu.accumulator
        assert cpu.carry
      end

      def test_cmc
        cpu = Intel4004GateLevel.new
        cpu.run([0xF3, 0x01])
        assert cpu.carry
      end

      def test_cma
        cpu = Intel4004GateLevel.new
        cpu.run([0xD5, 0xF4, 0x01])
        assert_equal 10, cpu.accumulator
      end

      def test_ral
        cpu = Intel4004GateLevel.new
        cpu.run([0xD5, 0xF5, 0x01]) # 0101 -> 1010
        assert_equal 0b1010, cpu.accumulator
      end

      def test_rar
        cpu = Intel4004GateLevel.new
        cpu.run([0xD4, 0xF6, 0x01]) # 0100 -> 0010
        assert_equal 2, cpu.accumulator
      end

      def test_tcc
        cpu = Intel4004GateLevel.new
        cpu.run([0xFA, 0xF7, 0x01])
        assert_equal 1, cpu.accumulator
        refute cpu.carry
      end

      def test_dac
        cpu = Intel4004GateLevel.new
        cpu.run([0xD5, 0xF8, 0x01])
        assert_equal 4, cpu.accumulator
        assert cpu.carry
      end

      def test_dac_zero
        cpu = Intel4004GateLevel.new
        cpu.run([0xD0, 0xF8, 0x01])
        assert_equal 15, cpu.accumulator
        refute cpu.carry
      end

      def test_tcs
        cpu = Intel4004GateLevel.new
        cpu.run([0xFA, 0xF9, 0x01])
        assert_equal 10, cpu.accumulator
      end

      def test_stc
        cpu = Intel4004GateLevel.new
        cpu.run([0xFA, 0x01])
        assert cpu.carry
      end

      def test_daa
        cpu = Intel4004GateLevel.new
        cpu.run([0xDC, 0xFB, 0x01])
        assert_equal 2, cpu.accumulator
        assert cpu.carry
      end

      def test_kbp_all_values
        expected = {0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4, 3 => 15, 15 => 15}
        expected.each do |inp, out|
          cpu = Intel4004GateLevel.new
          cpu.run([0xD0 | inp, 0xFC, 0x01])
          assert_equal out, cpu.accumulator, "KBP(#{inp})=#{cpu.accumulator}, expected #{out}"
        end
      end

      def test_dcl
        cpu = Intel4004GateLevel.new
        cpu.run([0xD2, 0xFD, 0x01])
        assert_equal 2, cpu.ram_bank
      end
    end

    # ===================================================================
    # Jump instructions
    # ===================================================================

    class TestJumps < Minitest::Test
      def test_jun
        cpu = Intel4004GateLevel.new
        cpu.run([0x40, 0x04, 0xD5, 0x01, 0x01])
        assert_equal 0, cpu.accumulator # LDM 5 skipped
      end

      def test_jcn_zero
        cpu = Intel4004GateLevel.new
        cpu.run([0x14, 0x04, 0xD5, 0x01, 0x01])
        assert_equal 0, cpu.accumulator # A==0 -> jump
      end

      def test_jcn_nonzero_no_jump
        cpu = Intel4004GateLevel.new
        cpu.run([0xD3, 0x14, 0x06, 0xD5, 0x01, 0x01, 0x01])
        assert_equal 5, cpu.accumulator
      end

      def test_jcn_invert
        cpu = Intel4004GateLevel.new
        cpu.run([0xD3, 0x1C, 0x06, 0xD5, 0x01, 0x01, 0x01])
        assert_equal 3, cpu.accumulator # A!=0 -> jump (invert zero test)
      end

      def test_isz_loop
        cpu = Intel4004GateLevel.new
        cpu.run([0xDE, 0xB0, 0x70, 0x02, 0x01])
        assert_equal 0, cpu.registers[0]
      end
    end

    # ===================================================================
    # Subroutines
    # ===================================================================

    class TestSubroutines < Minitest::Test
      def test_jms_bbl
        cpu = Intel4004GateLevel.new
        cpu.run([
          0x50, 0x04, # JMS 0x004
          0x01,       # HLT (returned here)
          0x00,       # padding
          0xC5        # BBL 5
        ])
        assert_equal 5, cpu.accumulator
      end

      def test_nested
        cpu = Intel4004GateLevel.new
        cpu.run([
          0x50, 0x06, # JMS sub1
          0xB0, 0x01, # XCH R0, HLT
          0x00, 0x00, # padding
          0x50, 0x0C, # sub1: JMS sub2
          0xB1,       # XCH R1
          0xD9, 0xC0, # LDM 9, BBL 0
          0x00,       # padding
          0xC3        # sub2: BBL 3
        ])
        assert_equal 3, cpu.registers[1]
      end
    end

    # ===================================================================
    # Register pairs
    # ===================================================================

    class TestPairs < Minitest::Test
      def test_fim
        cpu = Intel4004GateLevel.new
        cpu.run([0x20, 0xAB, 0x01])
        assert_equal 0xA, cpu.registers[0]
        assert_equal 0xB, cpu.registers[1]
      end

      def test_src_wrm_rdm
        cpu = Intel4004GateLevel.new
        cpu.run([
          0x20, 0x00, 0x21, 0xD7, 0xE0, # SRC P0, LDM 7, WRM
          0xD0,                           # LDM 0
          0x20, 0x00, 0x21, 0xE9,         # SRC P0, RDM
          0x01
        ])
        assert_equal 7, cpu.accumulator
      end

      def test_jin
        cpu = Intel4004GateLevel.new
        cpu.run([0x22, 0x06, 0x33, 0xD5, 0x01, 0x00, 0x01])
        assert_equal 0, cpu.accumulator # LDM 5 skipped
      end
    end

    # ===================================================================
    # RAM I/O
    # ===================================================================

    class TestRAMIO < Minitest::Test
      def test_status_write_read
        cpu = Intel4004GateLevel.new
        cpu.run([
          0x20, 0x00, 0x21, # SRC P0
          0xD3, 0xE4,       # LDM 3, WR0
          0xD0,             # LDM 0
          0x20, 0x00, 0x21, # SRC P0
          0xEC,             # RD0
          0x01
        ])
        assert_equal 3, cpu.accumulator
      end

      def test_wrr_rdr
        cpu = Intel4004GateLevel.new
        cpu.run([0xDB, 0xE2, 0xD0, 0xEA, 0x01])
        assert_equal 11, cpu.accumulator
      end

      def test_ram_banking
        cpu = Intel4004GateLevel.new
        cpu.run([
          0xD0, 0xFD,       # DCL bank 0
          0x20, 0x00, 0x21, # SRC P0
          0xD5, 0xE0,       # LDM 5, WRM
          0xD1, 0xFD,       # DCL bank 1
          0x20, 0x00, 0x21,
          0xD9, 0xE0,       # LDM 9, WRM
          0xD0, 0xFD,       # DCL bank 0
          0x20, 0x00, 0x21,
          0xE9,             # RDM
          0x01
        ])
        assert_equal 5, cpu.accumulator
      end
    end

    # ===================================================================
    # End-to-end programs
    # ===================================================================

    class TestEndToEnd < Minitest::Test
      def test_x_equals_1_plus_2
        cpu = Intel4004GateLevel.new
        cpu.run([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])
        assert_equal 3, cpu.registers[1]
        assert cpu.halted?
      end

      def test_multiply_3x4
        cpu = Intel4004GateLevel.new
        cpu.run([
          0xD3, 0xB0, 0xDC, 0xB1,
          0xD0, 0x80, 0x71, 0x05,
          0xB2, 0x01
        ])
        assert_equal 12, cpu.registers[2]
      end

      def test_bcd_7_plus_8
        cpu = Intel4004GateLevel.new
        cpu.run([
          0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01
        ])
        assert_equal 5, cpu.accumulator
        assert cpu.carry
      end

      def test_countdown
        cpu = Intel4004GateLevel.new
        cpu.run([0xD5, 0xF8, 0x1C, 0x01, 0x01])
        assert_equal 0, cpu.accumulator
      end

      def test_max_steps
        cpu = Intel4004GateLevel.new
        traces = cpu.run([0x40, 0x00], max_steps: 10)
        assert_equal 10, traces.length
      end

      def test_gate_count
        cpu = Intel4004GateLevel.new
        count = cpu.gate_count
        assert count > 500 # Sanity check
      end
    end

    # ===================================================================
    # Component tests
    # ===================================================================

    class TestComponents < Minitest::Test
      def test_bits_roundtrip
        16.times do |val|
          assert_equal val, Bits.bits_to_int(Bits.int_to_bits(val, 4))
        end

        4096.times do |val|
          assert_equal val, Bits.bits_to_int(Bits.int_to_bits(val, 12))
        end
      end

      def test_alu_add
        alu = GateALU.new
        result, carry = alu.add(5, 3, 0)
        assert_equal 8, result
        refute carry
      end

      def test_alu_sub
        alu = GateALU.new
        result, carry = alu.subtract(5, 3, 1)
        assert_equal 2, result
        assert carry # no borrow
      end

      def test_register_file
        rf = RegisterFile.new
        rf.write(5, 11)
        assert_equal 11, rf.read(5)
        assert_equal 0, rf.read(0)
      end

      def test_pc_increment
        pc = ProgramCounter.new
        assert_equal 0, pc.read
        pc.increment
        assert_equal 1, pc.read
        pc.increment
        assert_equal 2, pc.read
      end

      def test_stack_push_pop
        stack = HardwareStack.new
        stack.push(0x100)
        stack.push(0x200)
        assert_equal 0x200, stack.pop
        assert_equal 0x100, stack.pop
      end

      def test_decoder
        d = Decoder.decode(0xD5)
        assert_equal 1, d.is_ldm
        assert_equal 5, d.immediate

        d = Decoder.decode(0x80)
        assert_equal 1, d.is_add
        assert_equal 0, d.reg_index
      end
    end
  end
end
