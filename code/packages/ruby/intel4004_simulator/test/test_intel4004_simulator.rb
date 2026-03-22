# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Intel4004Simulator
    class TestIntel4004 < Minitest::Test
      # ---------------------------------------------------------------
      # Helper: build a binary program from an array of byte values
      # ---------------------------------------------------------------
      def prog(*bytes)
        bytes.flatten.pack("C*")
      end

      def setup
        @sim = Intel4004Sim.new
      end

      # =================================================================
      # NOP (0x00)
      # =================================================================

      def test_nop
        @sim.load_program(prog(0x00, 0x01))
        trace = @sim.step
        assert_equal "NOP", trace.mnemonic
        assert_equal 0, @sim.accumulator
        assert_equal 1, @sim.pc
      end

      # =================================================================
      # HLT (0x01) -- simulator-only halt
      # =================================================================

      def test_hlt
        @sim.load_program(prog(0x01))
        trace = @sim.step
        assert_equal "HLT", trace.mnemonic
        assert @sim.halted?
      end

      def test_halted_raises_on_step
        @sim.run(prog(0x01))
        assert_raises(RuntimeError) { @sim.step }
      end

      # =================================================================
      # LDM N (0xDN) -- load immediate
      # =================================================================

      def test_ldm_loads_nibble
        @sim.load_program(prog(0xD5, 0x01))
        trace = @sim.step
        assert_equal "LDM 5", trace.mnemonic
        assert_equal 0, trace.accumulator_before
        assert_equal 5, trace.accumulator_after
        assert_equal 5, @sim.accumulator
      end

      def test_ldm_all_values
        (0..15).each do |n|
          sim = Intel4004Sim.new
          sim.load_program(prog(0xD0 | n, 0x01))
          sim.step
          assert_equal n, sim.accumulator, "LDM #{n} should load #{n}"
        end
      end

      # =================================================================
      # LD Rn (0xAR) -- load register into accumulator
      # =================================================================

      def test_ld
        @sim.load_program(prog(0xD7, 0xB3, 0xA3, 0x01))
        @sim.step # LDM 7
        @sim.step # XCH R3 -> R3=7, A=0
        trace = @sim.step # LD R3 -> A=7
        assert_equal "LD R3", trace.mnemonic
        assert_equal 7, @sim.accumulator
      end

      # =================================================================
      # XCH Rn (0xBR) -- exchange accumulator and register
      # =================================================================

      def test_xch
        @sim.load_program(prog(0xD7, 0xB3, 0x01))
        @sim.step # LDM 7 -> A=7
        trace = @sim.step # XCH R3 -> R3=7, A=0
        assert_equal "XCH R3", trace.mnemonic
        assert_equal 0, @sim.accumulator
        assert_equal 7, @sim.registers[3]
      end

      # =================================================================
      # INC Rn (0x6R) -- increment register (no carry effect)
      # =================================================================

      def test_inc
        @sim.load_program(prog(0xD5, 0xB0, 0x60, 0x01))
        @sim.step # LDM 5
        @sim.step # XCH R0 -> R0=5
        trace = @sim.step # INC R0 -> R0=6
        assert_equal "INC R0", trace.mnemonic
        assert_equal 6, @sim.registers[0]
      end

      def test_inc_wraps_at_16
        @sim.load_program(prog(0xDF, 0xB0, 0x60, 0x01))
        @sim.step # LDM 15
        @sim.step # XCH R0 -> R0=15
        @sim.step # INC R0 -> R0=0 (wraps)
        assert_equal 0, @sim.registers[0]
      end

      def test_inc_does_not_affect_carry
        @sim.load_program(prog(0xDF, 0xB0, 0x60, 0x01))
        @sim.step # LDM 15
        @sim.step # XCH R0 -> R0=15
        @sim.step # INC R0 -> R0=0 (wraps, but no carry)
        refute @sim.carry
      end

      # =================================================================
      # ADD Rn (0x8R) -- add register to accumulator with carry
      # =================================================================

      def test_add_no_carry
        @sim.load_program(prog(0xD3, 0xB0, 0xD2, 0x80, 0x01))
        @sim.step # LDM 3
        @sim.step # XCH R0 -> R0=3
        @sim.step # LDM 2
        trace = @sim.step # ADD R0 -> A=2+3+0=5
        assert_equal "ADD R0", trace.mnemonic
        assert_equal 5, @sim.accumulator
        refute @sim.carry
      end

      def test_add_with_overflow
        @sim.load_program(prog(0xDF, 0xB0, 0xD1, 0x80, 0x01))
        @sim.step # LDM 15
        @sim.step # XCH R0 -> R0=15
        @sim.step # LDM 1
        @sim.step # ADD R0 -> 1+15+0=16, carry=true, A=0
        assert_equal 0, @sim.accumulator
        assert @sim.carry
      end

      def test_add_includes_carry_in
        # Set carry first via STC, then add: A = 2 + 3 + 1(carry) = 6
        @sim.load_program(prog(
          0xD3, 0xB0, # R0=3
          0xD2,       # A=2
          0xFA,       # STC -> carry=true
          0x80,       # ADD R0 -> 2+3+1=6
          0x01
        ))
        @sim.step # LDM 3
        @sim.step # XCH R0
        @sim.step # LDM 2
        @sim.step # STC
        @sim.step # ADD R0
        assert_equal 6, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # SUB Rn (0x9R) -- subtract register (complement-add)
      # =================================================================
      # carry=true means NO borrow, carry=false means borrow

      def test_sub_no_borrow
        # 5 - 2: complement(2)=13, borrow_in=1(carry=false), 5+13+1=19>15
        # -> carry=true (no borrow), A=3
        @sim.load_program(prog(0xD2, 0xB0, 0xD5, 0x90, 0x01))
        @sim.step # LDM 2
        @sim.step # XCH R0 -> R0=2
        @sim.step # LDM 5
        @sim.step # SUB R0
        assert_equal 3, @sim.accumulator
        assert @sim.carry # no borrow
      end

      def test_sub_with_borrow
        # 1 - 3: complement(3)=12, borrow_in=1(carry=false), 1+12+1=14<=15
        # -> carry=false (borrow), A=14
        @sim.load_program(prog(0xD3, 0xB0, 0xD1, 0x90, 0x01))
        @sim.step # LDM 3
        @sim.step # XCH R0 -> R0=3
        @sim.step # LDM 1
        @sim.step # SUB R0
        assert_equal 14, @sim.accumulator
        refute @sim.carry # borrow occurred
      end

      def test_sub_equal_values
        # 5-5: complement(5)=10, borrow_in=1, 5+10+1=16>15 -> carry=true, A=0
        @sim.load_program(prog(0xD5, 0xB0, 0xD5, 0x90, 0x01))
        3.times { @sim.step }
        @sim.step # SUB R0
        assert_equal 0, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # JUN addr (0x4H LL) -- unconditional jump
      # =================================================================

      def test_jun
        @sim.load_program(prog(0x40, 0x04, 0x00, 0x00, 0xD5, 0x01))
        trace = @sim.step # JUN 0x004
        assert_equal "JUN 0x004", trace.mnemonic
        assert_equal 4, @sim.pc
        @sim.step # LDM 5
        assert_equal 5, @sim.accumulator
      end

      def test_jun_12bit_address
        @sim.load_program(prog(0x41, 0x23, 0x01))
        @sim.step
        assert_equal 0x123, @sim.pc
      end

      # =================================================================
      # JCN cond,addr (0x1C AA) -- conditional jump
      # =================================================================

      def test_jcn_jump_if_accumulator_zero
        @sim.load_program(prog(0x14, 0x04, 0x00, 0x00, 0xD5, 0x01))
        @sim.step # JCN 4,04 -> A==0 -> jump to 0x04
        assert_equal 4, @sim.pc
      end

      def test_jcn_no_jump_when_nonzero
        # LDM 5 at 0x00 (1 byte), JCN at 0x01-0x02 (2 bytes), HLT at 0x03
        @sim.load_program(prog(0xD5, 0x14, 0x06, 0x01))
        @sim.step # LDM 5
        @sim.step # JCN 4,06 -> A!=0 -> fall through
        assert_equal 3, @sim.pc # fell through past the 2-byte JCN
      end

      def test_jcn_invert_test
        # cond=0xC (invert + test_zero), A=5 -> ~(false) = true -> jump
        @sim.load_program(prog(
          0xD5, 0x1C, 0x06,
          0x00, 0x00, 0x00,
          0xD9, 0x01
        ))
        @sim.step # LDM 5 (A=5)
        @sim.step # JCN 0xC,0x06 -> A!=0, invert->true -> jump to 0x06
        assert_equal 6, @sim.pc
      end

      def test_jcn_test_carry
        @sim.load_program(prog(
          0xFA,       # STC (set carry)
          0x12, 0x05, # JCN 2,05 -> carry set -> jump
          0x00, 0x00,
          0xD5, 0x01
        ))
        @sim.step # STC
        @sim.step # JCN
        assert_equal 5, @sim.pc
      end

      def test_jcn_pin_test_never_true
        @sim.load_program(prog(0x11, 0x05, 0xD3, 0x01))
        @sim.step # JCN 1,05 -> pin=0 -> no jump
        assert_equal 2, @sim.pc
      end

      def test_jcn_inverted_pin_always_true
        @sim.load_program(prog(
          0x19, 0x05,
          0xD3, 0x01,
          0x00,
          0xD7, 0x01
        ))
        @sim.step # JCN 9,05 -> inverted pin -> jump to 0x05
        assert_equal 5, @sim.pc
      end

      # =================================================================
      # FIM Pp,data (0x2P_even data) -- fetch immediate to pair
      # =================================================================

      def test_fim
        @sim.load_program(prog(0x20, 0xAB, 0x01))
        trace = @sim.step # FIM P0,0xAB
        assert_equal "FIM P0,0xAB", trace.mnemonic
        assert_equal 0xA, @sim.registers[0]
        assert_equal 0xB, @sim.registers[1]
      end

      def test_fim_pair3
        @sim.load_program(prog(0x26, 0x42, 0x01))
        @sim.step # FIM P3,0x42
        assert_equal 4, @sim.registers[6]
        assert_equal 2, @sim.registers[7]
      end

      def test_fim_all_pairs
        (0..7).each do |p|
          sim = Intel4004Sim.new
          opcode = 0x20 | (p << 1)
          sim.load_program(prog(opcode, 0xAB, 0x01))
          sim.step
          assert_equal 0xA, sim.registers[p * 2], "FIM P#{p} high"
          assert_equal 0xB, sim.registers[p * 2 + 1], "FIM P#{p} low"
        end
      end

      # =================================================================
      # SRC Pp (0x2P_odd) -- send register control
      # =================================================================

      def test_src
        @sim.load_program(prog(0x20, 0x35, 0x21, 0x01))
        @sim.step # FIM P0,0x35 -> R0=3, R1=5
        trace = @sim.step # SRC P0 -> ram_register=3, ram_character=5
        assert_equal "SRC P0", trace.mnemonic
        assert_equal 3, @sim.ram_register
        assert_equal 5, @sim.ram_character
      end

      def test_src_all_pairs
        (0..7).each do |p|
          sim = Intel4004Sim.new
          fim_opcode = 0x20 | (p << 1)
          src_opcode = 0x21 | (p << 1)
          sim.load_program(prog(fim_opcode, 0x35, src_opcode, 0x01))
          sim.step # FIM
          sim.step # SRC
          assert_equal 3, sim.ram_register
          assert_equal 5, sim.ram_character
        end
      end

      # =================================================================
      # FIN Pp (0x3P_even) -- fetch indirect from ROM
      # =================================================================

      def test_fin
        rom = Array.new(256, 0)
        rom[0] = 0x20   # FIM P0,0x0A
        rom[1] = 0x0A
        rom[2] = 0x34   # FIN P2
        rom[3] = 0x01   # HLT
        rom[0x0A] = 0xCD
        @sim.load_program(rom.pack("C*"))
        @sim.step # FIM P0,0x0A
        trace = @sim.step # FIN P2
        assert_equal "FIN P2", trace.mnemonic
        assert_equal 0xC, @sim.registers[4]
        assert_equal 0xD, @sim.registers[5]
      end

      # =================================================================
      # JIN Pp (0x3P_odd) -- jump indirect
      # =================================================================

      def test_jin
        @sim.load_program(prog(
          0x20, 0x08, # FIM P0,0x08
          0x31,       # JIN P0 -> PC = page(0) | 0x08 = 0x08
          0x00, 0x00, 0x00, 0x00, 0x00,
          0xD5, 0x01
        ))
        @sim.step # FIM P0,0x08
        trace = @sim.step # JIN P0
        assert_equal "JIN P0", trace.mnemonic
        assert_equal 8, @sim.pc
      end

      # =================================================================
      # JMS addr (0x5H LL) -- jump to subroutine
      # =================================================================

      def test_jms_pushes_return_address
        @sim.load_program(prog(0x50, 0x04, 0x00, 0x00, 0xD5, 0x01))
        @sim.step # JMS 0x004 -> push 0x002, jump to 0x004
        assert_equal 4, @sim.pc
        assert_equal 2, @sim.hw_stack[0]
      end

      # =================================================================
      # BBL N (0xCN) -- branch back and load
      # =================================================================

      def test_bbl_pops_and_loads
        @sim.load_program(prog(
          0x50, 0x04, # JMS 0x004
          0xD0, 0x01, # (return here)
          0xC7        # BBL 7 -> A=7, pop -> return to 0x002
        ))
        @sim.step # JMS 0x004
        @sim.step # BBL 7
        assert_equal 7, @sim.accumulator
        assert_equal 2, @sim.pc
      end

      # =================================================================
      # ISZ Rn,addr (0x7R AA) -- increment and skip if zero
      # =================================================================

      def test_isz_loops_until_zero
        @sim.load_program(prog(
          0xDE, 0xB0, # LDM 14, XCH R0 -> R0=14
          0x70, 0x02, # ISZ R0,0x02 -> inc R0
          0x01        # HLT
        ))
        @sim.step # LDM 14
        @sim.step # XCH R0
        @sim.step # ISZ R0 -> R0=15, !=0 -> jump to 0x02
        assert_equal 15, @sim.registers[0]
        assert_equal 2, @sim.pc
        @sim.step # ISZ R0 -> R0=0, ==0 -> fall through
        assert_equal 0, @sim.registers[0]
        assert_equal 4, @sim.pc
      end

      def test_isz_immediate_zero
        @sim.load_program(prog(0xDF, 0xB0, 0x70, 0x00, 0x01))
        @sim.step # LDM 15
        @sim.step # XCH R0
        @sim.step # ISZ R0,0x00 -> R0=0 -> fall through
        assert_equal 0, @sim.registers[0]
        assert_equal 4, @sim.pc
      end

      # =================================================================
      # I/O: WRM, RDM -- write/read RAM main
      # =================================================================

      def test_wrm_rdm
        @sim.load_program(prog(
          0x20, 0x13, # FIM P0,0x13 -> R0=1, R1=3
          0x21,       # SRC P0 -> reg=1, char=3
          0xD9,       # LDM 9
          0xE0,       # WRM -> ram[0][1][3] = 9
          0xD0,       # LDM 0 -> clear A
          0xE9,       # RDM -> A = ram[0][1][3] = 9
          0x01
        ))
        7.times { @sim.step }
        assert_equal 9, @sim.accumulator
        assert_equal 9, @sim.ram[0][1][3]
      end

      # =================================================================
      # I/O: WMP -- write to RAM output port
      # =================================================================

      def test_wmp
        @sim.load_program(prog(0xD7, 0xE1, 0x01))
        @sim.step # LDM 7
        @sim.step # WMP
        assert_equal 7, @sim.ram_output[0]
      end

      # =================================================================
      # I/O: WRR, RDR -- write/read ROM port
      # =================================================================

      def test_wrr_rdr
        @sim.load_program(prog(0xDA, 0xE2, 0xD0, 0xEA, 0x01))
        @sim.step # LDM 10
        @sim.step # WRR
        assert_equal 10, @sim.rom_port
        @sim.step # LDM 0
        @sim.step # RDR
        assert_equal 10, @sim.accumulator
      end

      # =================================================================
      # I/O: WPM -- write program memory (NOP in sim)
      # =================================================================

      def test_wpm_nop
        @sim.load_program(prog(0xE3, 0x01))
        trace = @sim.step
        assert_equal "WPM", trace.mnemonic
      end

      # =================================================================
      # I/O: WR0-WR3, RD0-RD3 -- status characters
      # =================================================================

      def test_wr0_rd0
        @sim.load_program(prog(
          0x20, 0x00, 0x21, # FIM P0,0; SRC P0
          0xD5, 0xE4,       # LDM 5; WR0
          0xD0, 0xEC,       # LDM 0; RD0
          0x01
        ))
        7.times { @sim.step }
        assert_equal 5, @sim.accumulator
        assert_equal 5, @sim.ram_status[0][0][0]
      end

      def test_wr1_rd1
        @sim.load_program(prog(
          0x20, 0x00, 0x21,
          0xD3, 0xE5,
          0xD0, 0xED,
          0x01
        ))
        7.times { @sim.step }
        assert_equal 3, @sim.accumulator
      end

      def test_wr2_rd2
        @sim.load_program(prog(
          0x20, 0x00, 0x21,
          0xD8, 0xE6,
          0xD0, 0xEE,
          0x01
        ))
        7.times { @sim.step }
        assert_equal 8, @sim.accumulator
      end

      def test_wr3_rd3
        @sim.load_program(prog(
          0x20, 0x00, 0x21,
          0xD1, 0xE7,
          0xD0, 0xEF,
          0x01
        ))
        7.times { @sim.step }
        assert_equal 1, @sim.accumulator
      end

      # =================================================================
      # I/O: SBM -- subtract RAM from accumulator
      # =================================================================

      def test_sbm
        @sim.load_program(prog(
          0x20, 0x00, 0x21, # FIM P0,0; SRC P0
          0xD3, 0xE0,       # LDM 3; WRM -> ram=3
          0xD7,             # LDM 7
          0xE8,             # SBM -> 7-3 with complement-add
          0x01
        ))
        7.times { @sim.step }
        # complement(3)=12, borrow_in=1(carry=false), 7+12+1=20>15
        # -> carry=true, A=4
        assert_equal 4, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # I/O: ADM -- add RAM to accumulator with carry
      # =================================================================

      def test_adm
        @sim.load_program(prog(
          0x20, 0x00, 0x21, # FIM P0,0; SRC P0
          0xD5, 0xE0,       # LDM 5; WRM -> ram=5
          0xD3,             # LDM 3
          0xEB,             # ADM -> A=3+5+0=8
          0x01
        ))
        7.times { @sim.step }
        assert_equal 8, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # Accumulator ops: CLB (0xF0)
      # =================================================================

      def test_clb
        @sim.load_program(prog(0xD5, 0xFA, 0xF0, 0x01))
        @sim.step # LDM 5
        @sim.step # STC
        trace = @sim.step # CLB
        assert_equal "CLB", trace.mnemonic
        assert_equal 0, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # CLC (0xF1)
      # =================================================================

      def test_clc
        @sim.load_program(prog(0xFA, 0xF1, 0x01))
        @sim.step # STC
        assert @sim.carry
        @sim.step # CLC
        refute @sim.carry
      end

      # =================================================================
      # IAC (0xF2) -- increment accumulator
      # =================================================================

      def test_iac
        @sim.load_program(prog(0xD5, 0xF2, 0x01))
        @sim.step # LDM 5
        trace = @sim.step # IAC -> A=6
        assert_equal "IAC", trace.mnemonic
        assert_equal 6, @sim.accumulator
        refute @sim.carry
      end

      def test_iac_overflow
        @sim.load_program(prog(0xDF, 0xF2, 0x01))
        @sim.step # LDM 15
        @sim.step # IAC -> A=0, carry=true
        assert_equal 0, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # CMC (0xF3) -- complement carry
      # =================================================================

      def test_cmc
        @sim.load_program(prog(0xF3, 0xF3, 0x01))
        @sim.step # CMC -> carry=true
        assert @sim.carry
        @sim.step # CMC -> carry=false
        refute @sim.carry
      end

      # =================================================================
      # CMA (0xF4) -- complement accumulator
      # =================================================================

      def test_cma
        @sim.load_program(prog(0xD5, 0xF4, 0x01))
        @sim.step # LDM 5 (0101)
        trace = @sim.step # CMA -> ~5 & 0xF = 10 (1010)
        assert_equal "CMA", trace.mnemonic
        assert_equal 10, @sim.accumulator
      end

      # =================================================================
      # RAL (0xF5) -- rotate left through carry
      # =================================================================

      def test_ral
        @sim.load_program(prog(0xD5, 0xF5, 0x01))
        @sim.step # LDM 5 (0101)
        trace = @sim.step # RAL
        assert_equal "RAL", trace.mnemonic
        assert_equal 0b1010, @sim.accumulator
        refute @sim.carry
      end

      def test_ral_with_carry
        @sim.load_program(prog(0xD9, 0xFA, 0xF5, 0x01))
        @sim.step # LDM 9 (1001)
        @sim.step # STC
        @sim.step # RAL -> carry=bit3(1)=true, A=(0010|1)=0011=3
        assert_equal 3, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # RAR (0xF6) -- rotate right through carry
      # =================================================================

      def test_rar
        @sim.load_program(prog(0xD6, 0xF6, 0x01))
        @sim.step # LDM 6 (0110)
        trace = @sim.step # RAR
        assert_equal "RAR", trace.mnemonic
        assert_equal 3, @sim.accumulator
        refute @sim.carry
      end

      def test_rar_with_carry
        @sim.load_program(prog(0xD5, 0xFA, 0xF6, 0x01))
        @sim.step # LDM 5 (0101)
        @sim.step # STC
        @sim.step # RAR -> carry=bit0(1)=true, A=(0010|1000)=1010=10
        assert_equal 10, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # TCC (0xF7) -- transfer carry to accumulator
      # =================================================================

      def test_tcc_carry_set
        @sim.load_program(prog(0xFA, 0xF7, 0x01))
        @sim.step # STC
        trace = @sim.step # TCC -> A=1, carry=false
        assert_equal "TCC", trace.mnemonic
        assert_equal 1, @sim.accumulator
        refute @sim.carry
      end

      def test_tcc_carry_clear
        @sim.load_program(prog(0xF7, 0x01))
        @sim.step # TCC -> A=0, carry=false
        assert_equal 0, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # DAC (0xF8) -- decrement accumulator
      # =================================================================

      def test_dac
        @sim.load_program(prog(0xD5, 0xF8, 0x01))
        @sim.step # LDM 5
        trace = @sim.step # DAC -> A=4, carry=true (no borrow)
        assert_equal "DAC", trace.mnemonic
        assert_equal 4, @sim.accumulator
        assert @sim.carry
      end

      def test_dac_underflow
        @sim.load_program(prog(0xD0, 0xF8, 0x01))
        @sim.step # LDM 0
        @sim.step # DAC -> A=15, carry=false (borrow)
        assert_equal 15, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # TCS (0xF9) -- transfer carry subtract
      # =================================================================

      def test_tcs_carry_set
        @sim.load_program(prog(0xFA, 0xF9, 0x01))
        @sim.step # STC
        trace = @sim.step # TCS -> A=10, carry=false
        assert_equal "TCS", trace.mnemonic
        assert_equal 10, @sim.accumulator
        refute @sim.carry
      end

      def test_tcs_carry_clear
        @sim.load_program(prog(0xF9, 0x01))
        @sim.step # TCS -> A=9, carry=false
        assert_equal 9, @sim.accumulator
        refute @sim.carry
      end

      # =================================================================
      # STC (0xFA) -- set carry
      # =================================================================

      def test_stc
        @sim.load_program(prog(0xFA, 0x01))
        trace = @sim.step
        assert_equal "STC", trace.mnemonic
        assert @sim.carry
      end

      # =================================================================
      # DAA (0xFB) -- decimal adjust accumulator
      # =================================================================

      def test_daa_no_adjustment
        @sim.load_program(prog(0xD5, 0xFB, 0x01))
        @sim.step # LDM 5
        @sim.step # DAA -> A<=9, no carry -> no change
        assert_equal 5, @sim.accumulator
        refute @sim.carry
      end

      def test_daa_adjustment_needed
        @sim.load_program(prog(0xDC, 0xFB, 0x01))
        @sim.step # LDM 12
        @sim.step # DAA -> 12+6=18 -> A=2, carry=true
        assert_equal 2, @sim.accumulator
        assert @sim.carry
      end

      def test_daa_carry_set
        @sim.load_program(prog(0xD3, 0xFA, 0xFB, 0x01))
        @sim.step # LDM 3
        @sim.step # STC
        @sim.step # DAA -> carry set -> 3+6=9, carry=true
        assert_equal 9, @sim.accumulator
        assert @sim.carry
      end

      def test_daa_bcd_addition
        # 7 + 8 = 15 -> DAA -> 15+6=21 -> A=5, carry=1
        @sim.load_program(prog(
          0xD8, 0xB0, # R0=8
          0xD7,       # A=7
          0xF1,       # CLC
          0x80,       # ADD R0 -> 7+8=15
          0xFB,       # DAA -> 15>9 -> A=5, carry=true
          0x01
        ))
        6.times { @sim.step }
        assert_equal 5, @sim.accumulator
        assert @sim.carry
      end

      # =================================================================
      # KBP (0xFC) -- keyboard process
      # =================================================================

      def test_kbp_truth_table
        {0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4}.each do |input, expected|
          sim = Intel4004Sim.new
          sim.load_program(prog(0xD0 | input, 0xFC, 0x01))
          sim.step # LDM
          sim.step # KBP
          assert_equal expected, sim.accumulator,
            "KBP(#{input}) should be #{expected}"
        end
      end

      def test_kbp_error_values
        [3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15].each do |input|
          sim = Intel4004Sim.new
          sim.load_program(prog(0xD0 | input, 0xFC, 0x01))
          sim.step
          sim.step
          assert_equal 15, sim.accumulator,
            "KBP(#{input}) should be 15 (error)"
        end
      end

      # =================================================================
      # DCL (0xFD) -- designate command line (select RAM bank)
      # =================================================================

      def test_dcl
        @sim.load_program(prog(0xD2, 0xFD, 0x01))
        @sim.step # LDM 2
        trace = @sim.step # DCL -> ram_bank = 2
        assert_equal "DCL", trace.mnemonic
        assert_equal 2, @sim.ram_bank
      end

      def test_dcl_clamps_to_3
        @sim.load_program(prog(0xD7, 0xFD, 0x01))
        @sim.step # LDM 7
        @sim.step # DCL -> 7 & 7 = 7 -> clamped to 3
        assert_equal 3, @sim.ram_bank
      end

      # =================================================================
      # RAM bank selection with DCL
      # =================================================================

      def test_ram_bank_selection
        @sim.load_program(prog(
          0xD1, 0xFD,       # DCL bank 1
          0x20, 0x00, 0x21, # FIM P0,0; SRC P0
          0xD7, 0xE0,       # LDM 7; WRM -> ram[1][0][0]=7
          0xD0, 0xFD,       # LDM 0; DCL bank 0
          0x20, 0x00, 0x21, # FIM P0,0; SRC P0
          0xE9,             # RDM -> ram[0][0][0]=0
          0x01
        ))
        13.times { @sim.step }
        assert_equal 0, @sim.accumulator
        assert_equal 7, @sim.ram[1][0][0]
      end

      # =================================================================
      # 2-byte instruction detection
      # =================================================================

      def test_two_byte_detection
        @sim.load_program(prog(
          0x40, 0x04, # JUN 0x004
          0x00, 0x00,
          0xD5, 0x01  # LDM 5, HLT
        ))
        @sim.step # JUN
        @sim.step # LDM 5
        assert_equal 5, @sim.accumulator
      end

      # =================================================================
      # Trace fields
      # =================================================================

      def test_trace_raw2_for_two_byte
        @sim.load_program(prog(0x40, 0x10, 0x01))
        trace = @sim.step
        assert_equal 0x40, trace.raw
        assert_equal 0x10, trace.raw2
      end

      def test_trace_raw2_nil_for_one_byte
        @sim.load_program(prog(0xD5, 0x01))
        trace = @sim.step
        assert_nil trace.raw2
      end

      def test_trace_address
        @sim.load_program(prog(0xD1, 0xD2, 0x01))
        trace1 = @sim.step
        trace2 = @sim.step
        assert_equal 0, trace1.address
        assert_equal 1, trace2.address
      end

      # =================================================================
      # Stack operations -- nested subroutine calls
      # =================================================================

      def test_nested_calls
        rom = Array.new(256, 0)
        rom[0] = 0x50
        rom[1] = 0x10  # JMS 0x010
        rom[2] = 0x01                  # HLT

        rom[0x10] = 0x50
        rom[0x11] = 0x20 # JMS 0x020
        rom[0x12] = 0xC0                    # BBL 0

        rom[0x20] = 0xC3 # BBL 3
        @sim.load_program(rom.pack("C*"))
        @sim.step # JMS 0x010
        @sim.step # JMS 0x020
        @sim.step # BBL 3 -> A=3, pop -> 0x012
        assert_equal 3, @sim.accumulator
        assert_equal 0x12, @sim.pc
        @sim.step # BBL 0 -> A=0, pop -> 0x002
        assert_equal 0, @sim.accumulator
        assert_equal 2, @sim.pc
      end

      def test_stack_wraps_on_overflow
        rom = Array.new(256, 0)
        rom[0] = 0x50
        rom[1] = 0x10
        rom[0x10] = 0x50
        rom[0x11] = 0x20
        rom[0x20] = 0x50
        rom[0x21] = 0x30
        rom[0x30] = 0x50
        rom[0x31] = 0x40
        rom[0x40] = 0xC0
        @sim.load_program(rom.pack("C*"))
        4.times { @sim.step }
        # Stack pointer wraps: 0->1->2->0->1
        assert_equal 1, @sim.stack_pointer
      end

      # =================================================================
      # Reset method
      # =================================================================

      def test_reset_clears_state
        @sim.load_program(prog(0xD5, 0xFA, 0x01))
        @sim.step # LDM 5
        @sim.step # STC
        @sim.reset
        assert_equal 0, @sim.accumulator
        refute @sim.carry
        assert_equal 0, @sim.pc
        refute @sim.halted?
      end

      def test_run_resets_before_loading
        @sim.run(prog(0xD5, 0x01))
        assert_equal 5, @sim.accumulator
        @sim.run(prog(0xD3, 0x01))
        assert_equal 3, @sim.accumulator
      end

      # =================================================================
      # End-to-end: x = 1 + 2
      # =================================================================

      def test_e2e_add_1_plus_2
        traces = @sim.run(prog(0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01))
        assert_equal 6, traces.size
        assert_equal 3, @sim.registers[1]
        assert @sim.halted?
      end

      # =================================================================
      # End-to-end: multiply 3 x 4
      # =================================================================

      def test_e2e_multiply_3_x_4
        program = [
          0xD3, 0xB0,  # R0 = 3
          0xDC, 0xB2,  # R2 = 12 (= -4 mod 16)
          0xD0, 0xB1,  # R1 = 0 (result)
          # Loop at 0x06:
          0xA1,        # LD R1 -> A = result
          0xF1,        # CLC
          0x80,        # ADD R0 -> A = result + 3
          0xB1,        # XCH R1 -> R1 = new result
          0x72, 0x06,  # ISZ R2,0x06 -> loop
          0x01         # HLT
        ]
        @sim.run(prog(*program))
        assert_equal 12, @sim.registers[1] # 3*4=12
        assert @sim.halted?
      end

      # =================================================================
      # End-to-end: countdown from 5 to 0
      # =================================================================

      def test_e2e_countdown
        program = [
          0xD5,       # LDM 5
          0xF8,       # DAC -> A=A-1
          0x1C, 0x01, # JCN 0xC,0x01 -> if A!=0 jump to 0x01
          0x01        # HLT
        ]
        @sim.run(prog(*program))
        assert_equal 0, @sim.accumulator
        assert @sim.halted?
      end

      # =================================================================
      # Edge cases
      # =================================================================

      def test_max_steps_limit
        traces = @sim.run(prog(*([0xD0] * 100)), max_steps: 5)
        assert_equal 5, traces.size
      end

      def test_unknown_instruction_0x0f
        @sim.load_program(prog(0x0F, 0x01))
        trace = @sim.step
        assert_includes trace.mnemonic, "UNKNOWN"
      end

      def test_unknown_instruction_0xfe
        @sim.load_program(prog(0xFE, 0x01))
        trace = @sim.step
        assert_includes trace.mnemonic, "UNKNOWN"
      end

      # =================================================================
      # Carry chain: multi-digit addition
      # =================================================================

      def test_carry_chain
        prog2 = prog(
          0xDF, 0xB0, # R0 = 15
          0xD0, 0xB2, # R2 = 0
          0xD1,       # A = 1
          0xF1,       # CLC
          0x80,       # ADD R0 -> 1+15+0=16 -> A=0, carry=true
          0xB1,       # XCH R1 -> R1=0 (low result)
          0xD0,       # LDM 0
          0x82,       # ADD R2 -> 0+0+1(carry)=1
          0xB3,       # XCH R3 -> R3=1 (high result)
          0x01
        )
        @sim.run(prog2)
        assert_equal 0, @sim.registers[1] # low digit
        assert_equal 1, @sim.registers[3] # carry propagated
      end

      # =================================================================
      # 4-bit masking
      # =================================================================

      def test_4bit_masking
        @sim.load_program(prog(0xDF, 0x01))
        @sim.step
        assert_equal 15, @sim.accumulator
      end
    end
  end
end
