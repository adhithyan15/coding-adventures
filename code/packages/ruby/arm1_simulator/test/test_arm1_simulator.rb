# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_arm1_simulator"

# ===========================================================================
# ARM1 Behavioral Simulator Tests
# ===========================================================================
#
# These tests cover every major subsystem of the ARM1 simulator:
#   1. Constants and type helpers
#   2. Condition code evaluator
#   3. Barrel shifter (all shift types + edge cases)
#   4. ALU (all 16 operations, flag computation)
#   5. Instruction decoder
#   6. CPU execution (data processing, load/store, block transfer, branch, SWI)
#   7. Complete programs (loops, subroutines)

module CodingAdventures
  module Arm1Simulator
    class TestConstants < Minitest::Test
      def test_version_exists
        refute_nil VERSION
      end

      def test_mode_names
        assert_equal "USR", MODE_NAMES[MODE_USR]
        assert_equal "FIQ", MODE_NAMES[MODE_FIQ]
        assert_equal "IRQ", MODE_NAMES[MODE_IRQ]
        assert_equal "SVC", MODE_NAMES[MODE_SVC]
      end

      def test_op_names
        assert_equal "ADD", OP_NAMES[OP_ADD]
        assert_equal "MOV", OP_NAMES[OP_MOV]
        assert_equal "MVN", OP_NAMES[OP_MVN]
        assert_equal "AND", OP_NAMES[OP_AND]
      end

      def test_test_op?
        assert Arm1Simulator.test_op?(OP_TST)
        assert Arm1Simulator.test_op?(OP_TEQ)
        assert Arm1Simulator.test_op?(OP_CMP)
        assert Arm1Simulator.test_op?(OP_CMN)
        refute Arm1Simulator.test_op?(OP_ADD)
        refute Arm1Simulator.test_op?(OP_MOV)
      end

      def test_logical_op?
        assert Arm1Simulator.logical_op?(OP_AND)
        assert Arm1Simulator.logical_op?(OP_EOR)
        assert Arm1Simulator.logical_op?(OP_MOV)
        assert Arm1Simulator.logical_op?(OP_MVN)
        assert Arm1Simulator.logical_op?(OP_BIC)
        assert Arm1Simulator.logical_op?(OP_TST)
        assert Arm1Simulator.logical_op?(OP_TEQ)
        assert Arm1Simulator.logical_op?(OP_ORR)
        refute Arm1Simulator.logical_op?(OP_ADD)
        refute Arm1Simulator.logical_op?(OP_SUB)
      end
    end

    # =======================================================================
    # Condition Evaluator
    # =======================================================================

    class TestConditionEvaluator < Minitest::Test
      def test_eq_when_z_set
        assert Arm1Simulator.evaluate_condition(COND_EQ, Flags.new(z: true))
      end

      def test_eq_when_z_clear
        refute Arm1Simulator.evaluate_condition(COND_EQ, Flags.new)
      end

      def test_ne_when_z_clear
        assert Arm1Simulator.evaluate_condition(COND_NE, Flags.new)
      end

      def test_ne_when_z_set
        refute Arm1Simulator.evaluate_condition(COND_NE, Flags.new(z: true))
      end

      def test_cs_when_c_set
        assert Arm1Simulator.evaluate_condition(COND_CS, Flags.new(c: true))
      end

      def test_cc_when_c_clear
        assert Arm1Simulator.evaluate_condition(COND_CC, Flags.new)
      end

      def test_mi_when_n_set
        assert Arm1Simulator.evaluate_condition(COND_MI, Flags.new(n: true))
      end

      def test_pl_when_n_clear
        assert Arm1Simulator.evaluate_condition(COND_PL, Flags.new)
      end

      def test_vs_when_v_set
        assert Arm1Simulator.evaluate_condition(COND_VS, Flags.new(v: true))
      end

      def test_vc_when_v_clear
        assert Arm1Simulator.evaluate_condition(COND_VC, Flags.new)
      end

      def test_hi_when_c_set_z_clear
        assert Arm1Simulator.evaluate_condition(COND_HI, Flags.new(c: true))
      end

      def test_hi_when_c_set_z_set
        refute Arm1Simulator.evaluate_condition(COND_HI, Flags.new(c: true, z: true))
      end

      def test_ls_when_c_clear
        assert Arm1Simulator.evaluate_condition(COND_LS, Flags.new)
      end

      def test_ls_when_z_set
        assert Arm1Simulator.evaluate_condition(COND_LS, Flags.new(c: true, z: true))
      end

      def test_ge_when_n_eq_v_both_false
        assert Arm1Simulator.evaluate_condition(COND_GE, Flags.new)
      end

      def test_ge_when_n_eq_v_both_true
        assert Arm1Simulator.evaluate_condition(COND_GE, Flags.new(n: true, v: true))
      end

      def test_ge_when_n_ne_v
        refute Arm1Simulator.evaluate_condition(COND_GE, Flags.new(n: true))
      end

      def test_lt_when_n_ne_v
        assert Arm1Simulator.evaluate_condition(COND_LT, Flags.new(n: true))
      end

      def test_lt_when_n_eq_v
        refute Arm1Simulator.evaluate_condition(COND_LT, Flags.new)
      end

      def test_gt_when_z_clear_n_eq_v
        assert Arm1Simulator.evaluate_condition(COND_GT, Flags.new)
      end

      def test_gt_when_z_set
        refute Arm1Simulator.evaluate_condition(COND_GT, Flags.new(z: true))
      end

      def test_le_when_z_set
        assert Arm1Simulator.evaluate_condition(COND_LE, Flags.new(z: true))
      end

      def test_le_when_n_ne_v
        assert Arm1Simulator.evaluate_condition(COND_LE, Flags.new(n: true))
      end

      def test_al_always
        assert Arm1Simulator.evaluate_condition(COND_AL, Flags.new)
      end

      def test_nv_never
        refute Arm1Simulator.evaluate_condition(COND_NV, Flags.new)
      end
    end

    # =======================================================================
    # Barrel Shifter
    # =======================================================================

    class TestBarrelShifter < Minitest::Test
      def test_lsl_no_shift
        val, c = Arm1Simulator.barrel_shift(0xFF, SHIFT_LSL, 0, false, false)
        assert_equal 0xFF, val
        refute c
      end

      def test_lsl_by_1
        val, c = Arm1Simulator.barrel_shift(0xFF, SHIFT_LSL, 1, false, false)
        assert_equal 0x1FE, val
        refute c
      end

      def test_lsl_by_4
        val, c = Arm1Simulator.barrel_shift(0xFF, SHIFT_LSL, 4, false, false)
        assert_equal 0xFF0, val
        refute c
      end

      def test_lsl_by_31
        val, c = Arm1Simulator.barrel_shift(1, SHIFT_LSL, 31, false, false)
        assert_equal 0x80000000, val
        refute c
      end

      def test_lsl_by_32
        val, c = Arm1Simulator.barrel_shift(1, SHIFT_LSL, 32, false, false)
        assert_equal 0, val
        assert c
      end

      def test_lsl_by_33
        val, c = Arm1Simulator.barrel_shift(1, SHIFT_LSL, 33, false, false)
        assert_equal 0, val
        refute c
      end

      def test_lsr_by_1
        val, c = Arm1Simulator.barrel_shift(0xFF, SHIFT_LSR, 1, false, false)
        assert_equal 0x7F, val
        assert c
      end

      def test_lsr_by_8
        val, c = Arm1Simulator.barrel_shift(0xFF00, SHIFT_LSR, 8, false, false)
        assert_equal 0xFF, val
        refute c
      end

      def test_lsr_imm0_encodes_32
        val, c = Arm1Simulator.barrel_shift(0x80000000, SHIFT_LSR, 0, false, false)
        assert_equal 0, val
        assert c
      end

      def test_lsr_32_by_register
        val, c = Arm1Simulator.barrel_shift(0x80000000, SHIFT_LSR, 32, true, true)
        assert_equal 0, val
        assert c
      end

      def test_asr_positive
        val, c = Arm1Simulator.barrel_shift(0x7FFFFFFE, SHIFT_ASR, 1, false, false)
        assert_equal 0x3FFFFFFF, val
        refute c
      end

      def test_asr_negative
        val, c = Arm1Simulator.barrel_shift(0x80000000, SHIFT_ASR, 1, false, false)
        assert_equal 0xC0000000, val
        refute c
      end

      def test_asr_imm0_negative
        val, c = Arm1Simulator.barrel_shift(0x80000000, SHIFT_ASR, 0, false, false)
        assert_equal 0xFFFFFFFF, val
        assert c
      end

      def test_asr_imm0_positive
        val, c = Arm1Simulator.barrel_shift(0x7FFFFFFF, SHIFT_ASR, 0, false, false)
        assert_equal 0, val
        refute c
      end

      def test_ror_4
        val, c = Arm1Simulator.barrel_shift(0x0000000F, SHIFT_ROR, 4, false, false)
        assert_equal 0xF0000000, val
        assert c
      end

      def test_ror_8
        val, c = Arm1Simulator.barrel_shift(0x000000FF, SHIFT_ROR, 8, false, false)
        assert_equal 0xFF000000, val
        assert c
      end

      def test_ror_16
        val, c = Arm1Simulator.barrel_shift(0x0000FFFF, SHIFT_ROR, 16, false, false)
        assert_equal 0xFFFF0000, val
        assert c
      end

      def test_rrx_carry_in_true_bit0_set
        val, c = Arm1Simulator.barrel_shift(0x00000001, SHIFT_ROR, 0, true, false)
        assert_equal 0x80000000, val
        assert c
      end

      def test_rrx_carry_in_true_bit0_clear
        val, c = Arm1Simulator.barrel_shift(0x00000000, SHIFT_ROR, 0, true, false)
        assert_equal 0x80000000, val
        refute c
      end

      def test_by_register_amount_zero_passthrough
        val, c = Arm1Simulator.barrel_shift(0xDEADBEEF, SHIFT_LSL, 0, true, true)
        assert_equal 0xDEADBEEF, val
        assert c
      end

      def test_decode_immediate_no_rotation
        val, _ = Arm1Simulator.decode_immediate(0xFF, 0)
        assert_equal 0xFF, val
      end

      def test_decode_immediate_ror_2
        val, _ = Arm1Simulator.decode_immediate(0x01, 1)
        assert_equal 0x40000000, val
      end

      def test_decode_immediate_ror_8
        val, _ = Arm1Simulator.decode_immediate(0xFF, 4)
        assert_equal 0xFF000000, val
      end
    end

    # =======================================================================
    # ALU
    # =======================================================================

    class TestALU < Minitest::Test
      def test_add_basic
        r = Arm1Simulator.alu_execute(OP_ADD, 1, 2, false, false, false)
        assert_equal 3, r.result
        refute r.n
        refute r.z
        refute r.c
        refute r.v
      end

      def test_add_overflow
        r = Arm1Simulator.alu_execute(OP_ADD, 0x7FFFFFFF, 1, false, false, false)
        assert_equal 0x80000000, r.result
        assert r.n
        assert r.v
      end

      def test_add_carry
        r = Arm1Simulator.alu_execute(OP_ADD, 0xFFFFFFFF, 1, false, false, false)
        assert_equal 0, r.result
        assert r.c
        assert r.z
      end

      def test_sub_basic
        r = Arm1Simulator.alu_execute(OP_SUB, 5, 3, false, false, false)
        assert_equal 2, r.result
        assert r.c
      end

      def test_sub_borrow
        r = Arm1Simulator.alu_execute(OP_SUB, 3, 5, false, false, false)
        assert_equal 0xFFFFFFFE, r.result
        refute r.c
        assert r.n
      end

      def test_rsb
        r = Arm1Simulator.alu_execute(OP_RSB, 3, 5, false, false, false)
        assert_equal 2, r.result
      end

      def test_adc
        r = Arm1Simulator.alu_execute(OP_ADC, 1, 2, true, false, false)
        assert_equal 4, r.result
      end

      def test_sbc
        r = Arm1Simulator.alu_execute(OP_SBC, 5, 3, true, false, false)
        assert_equal 2, r.result
      end

      def test_rsc
        r = Arm1Simulator.alu_execute(OP_RSC, 3, 10, true, false, false)
        assert_equal 7, r.result
      end

      def test_and
        r = Arm1Simulator.alu_execute(OP_AND, 0xFF00FF00, 0x0FF00FF0, false, false, false)
        assert_equal 0x0F000F00, r.result
      end

      def test_eor
        r = Arm1Simulator.alu_execute(OP_EOR, 0xFF00FF00, 0x0FF00FF0, false, false, false)
        assert_equal 0xF0F0F0F0, r.result
      end

      def test_orr
        r = Arm1Simulator.alu_execute(OP_ORR, 0xFF00FF00, 0x0FF00FF0, false, false, false)
        assert_equal 0xFFF0FFF0, r.result
      end

      def test_bic
        r = Arm1Simulator.alu_execute(OP_BIC, 0xFFFFFFFF, 0x0000FF00, false, false, false)
        assert_equal 0xFFFF00FF, r.result
      end

      def test_mov
        r = Arm1Simulator.alu_execute(OP_MOV, 0, 42, false, false, false)
        assert_equal 42, r.result
      end

      def test_mvn
        r = Arm1Simulator.alu_execute(OP_MVN, 0, 0, false, false, false)
        assert_equal 0xFFFFFFFF, r.result
      end

      def test_tst_flags_only
        r = Arm1Simulator.alu_execute(OP_TST, 0xFF, 0x00, false, false, false)
        refute r.write_result
        assert r.z
      end

      def test_cmp_equal
        r = Arm1Simulator.alu_execute(OP_CMP, 5, 5, false, false, false)
        refute r.write_result
        assert r.z
        assert r.c
      end

      def test_teq
        r = Arm1Simulator.alu_execute(OP_TEQ, 42, 42, false, false, false)
        refute r.write_result
        assert r.z
      end

      def test_cmn
        r = Arm1Simulator.alu_execute(OP_CMN, 0xFFFFFFFF, 1, false, false, false)
        refute r.write_result
        assert r.z
        assert r.c
      end

      def test_logical_ops_preserve_v
        r = Arm1Simulator.alu_execute(OP_AND, 0xFF, 0xFF, false, false, true)
        assert r.v
      end

      def test_logical_ops_use_shifter_carry
        r = Arm1Simulator.alu_execute(OP_AND, 0xFF, 0xFF, false, true, false)
        assert r.c
      end
    end

    # =======================================================================
    # Instruction Decoder
    # =======================================================================

    class TestDecoder < Minitest::Test
      def test_decode_add_register
        d = Arm1Simulator.decode(0xE0802001)
        assert_equal INST_DATA_PROCESSING, d.type
        assert_equal COND_AL, d.cond
        assert_equal OP_ADD, d.opcode
        refute d.s
        assert_equal 0, d.rn
        assert_equal 2, d.rd
        assert_equal 1, d.rm
      end

      def test_decode_mov_immediate
        d = Arm1Simulator.decode(0xE3A0002A)
        assert_equal INST_DATA_PROCESSING, d.type
        assert_equal OP_MOV, d.opcode
        assert d.immediate
        assert_equal 0, d.rd
        assert_equal 42, d.imm8
      end

      def test_decode_branch
        d = Arm1Simulator.decode(0xEA000002)
        assert_equal INST_BRANCH, d.type
        refute d.link
        assert_equal 8, d.branch_offset
      end

      def test_decode_branch_link
        d = Arm1Simulator.decode(0xEBFFFFFE)
        assert_equal INST_BRANCH, d.type
        assert d.link
        assert_equal(-8, d.branch_offset)
      end

      def test_decode_swi
        d = Arm1Simulator.decode(0xEF123456)
        assert_equal INST_SWI, d.type
        assert_equal 0x123456, d.swi_comment
      end

      def test_decode_ldr
        inst = Arm1Simulator.encode_ldr(COND_AL, 0, 1, 0, true)
        d = Arm1Simulator.decode(inst)
        assert_equal INST_LOAD_STORE, d.type
        assert d.load
        assert d.pre_index
        assert d.up
        assert_equal 0, d.rd
        assert_equal 1, d.rn
      end

      def test_decode_str
        inst = Arm1Simulator.encode_str(COND_AL, 0, 1, 4, true)
        d = Arm1Simulator.decode(inst)
        assert_equal INST_LOAD_STORE, d.type
        refute d.load
        assert d.pre_index
      end

      def test_decode_ldm
        inst = Arm1Simulator.encode_ldm(COND_AL, 13, 0x000F, true, "IA")
        d = Arm1Simulator.decode(inst)
        assert_equal INST_BLOCK_TRANSFER, d.type
        assert d.load
        assert d.write_back
        assert_equal 13, d.rn
        assert_equal 0x000F, d.register_list
      end

      def test_decode_stm
        inst = Arm1Simulator.encode_stm(COND_AL, 13, 0x000F, true, "DB")
        d = Arm1Simulator.decode(inst)
        assert_equal INST_BLOCK_TRANSFER, d.type
        refute d.load
        assert d.pre_index
        refute d.up
      end
    end

    # =======================================================================
    # Disassembly
    # =======================================================================

    class TestDisassembly < Minitest::Test
      def test_disassemble_mov_imm
        d = Arm1Simulator.decode(0xE3A0002A)
        assert_equal "MOV R0, #42", Arm1Simulator.disassemble(d)
      end

      def test_disassemble_add_reg
        d = Arm1Simulator.decode(0xE0802001)
        assert_equal "ADD R2, R0, R1", Arm1Simulator.disassemble(d)
      end

      def test_disassemble_adds_reg
        d = Arm1Simulator.decode(0xE0912001)
        assert_equal "ADDS R2, R1, R1", Arm1Simulator.disassemble(d)
      end

      def test_disassemble_conditional
        d = Arm1Simulator.decode(0x10802001)
        assert_equal "ADDNE R2, R0, R1", Arm1Simulator.disassemble(d)
      end

      def test_disassemble_halt
        d = Arm1Simulator.decode(0xEF123456)
        assert_equal "HLT", Arm1Simulator.disassemble(d)
      end
    end

    # =======================================================================
    # CPU — Helper to load programs
    # =======================================================================

    module ProgramLoader
      def load_program(instructions)
        code = instructions.flat_map do |inst|
          [inst & 0xFF, (inst >> 8) & 0xFF, (inst >> 16) & 0xFF, (inst >> 24) & 0xFF]
        end
        @cpu.load_program(code, 0)
      end
    end

    # =======================================================================
    # CPU — Power-on state
    # =======================================================================

    class TestCPUInit < Minitest::Test
      def test_power_on_state
        cpu = ARM1.new(1024)
        assert_equal MODE_SVC, cpu.mode
        assert_equal 0, cpu.pc
        f = cpu.flags
        refute f.n
        refute f.z
        refute f.c
        refute f.v
      end

      def test_reset
        cpu = ARM1.new(1024)
        cpu.write_register(0, 42)
        cpu.reset
        assert_equal 0, cpu.read_register(0)
        assert_equal MODE_SVC, cpu.mode
        refute cpu.halted?
      end
    end

    # =======================================================================
    # CPU — Basic programs
    # =======================================================================

    class TestCPUBasic < Minitest::Test
      include ProgramLoader

      def setup
        @cpu = ARM1.new(4096)
      end

      def test_mov_immediate
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 42),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(10)
        assert_equal 42, @cpu.read_register(0)
      end

      def test_one_plus_two
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 1),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 2),
          Arm1Simulator.encode_alu_reg(COND_AL, OP_ADD, 0, 2, 0, 1),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(10)
        assert_equal 1, @cpu.read_register(0)
        assert_equal 2, @cpu.read_register(1)
        assert_equal 3, @cpu.read_register(2)
      end

      def test_subs_with_flags
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 5),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 5),
          Arm1Simulator.encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(10)
        assert_equal 0, @cpu.read_register(2)
        f = @cpu.flags
        assert f.z, "Z should be set (5 - 5 = 0)"
        assert f.c, "C should be set (no borrow)"
      end

      def test_conditional_execution
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 5),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 5),
          Arm1Simulator.encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
          Arm1Simulator.encode_mov_imm(COND_NE, 3, 99),
          Arm1Simulator.encode_mov_imm(COND_EQ, 4, 42),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(20)
        assert_equal 0, @cpu.read_register(3)
        assert_equal 42, @cpu.read_register(4)
      end

      def test_barrel_shifter_in_instruction
        add_with_shift = (COND_AL << 28) |
                         (OP_ADD << 21) |
                         (0 << 16) | (1 << 12) | (2 << 7) |
                         (SHIFT_LSL << 5) | 0
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 7),
          add_with_shift,
          Arm1Simulator.encode_halt
        ])
        @cpu.run(10)
        assert_equal 35, @cpu.read_register(1)
      end

      def test_loop_sum_one_to_ten
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 0),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 10),
          Arm1Simulator.encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 1),
          Arm1Simulator.encode_data_processing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
          Arm1Simulator.encode_branch(COND_NE, false, -16),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(100)
        assert_equal 55, @cpu.read_register(0)
        assert_equal 0, @cpu.read_register(1)
      end

      def test_halt
        load_program([Arm1Simulator.encode_halt])
        traces = @cpu.run(10)
        assert @cpu.halted?
        assert_equal 1, traces.length
      end

      def test_trace_captures_state
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 42),
          Arm1Simulator.encode_halt
        ])
        traces = @cpu.run(10)
        trace = traces[0]
        assert_equal 0, trace.address
        assert_equal "MOV R0, #42", trace.mnemonic
        assert trace.condition_met
        assert_equal 0, trace.regs_before[0]
        assert_equal 42, trace.regs_after[0]
      end
    end

    # =======================================================================
    # CPU — Load/Store
    # =======================================================================

    class TestCPULoadStore < Minitest::Test
      include ProgramLoader

      def setup
        @cpu = ARM1.new(4096)
      end

      def test_ldr_str
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 42),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 0),
          Arm1Simulator.encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
          Arm1Simulator.encode_str(COND_AL, 0, 1, 0, true),
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 0),
          Arm1Simulator.encode_ldr(COND_AL, 0, 1, 0, true),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(20)
        assert_equal 42, @cpu.read_register(0)
      end

      def test_ldr_byte
        @cpu.write_word(0x100, 0xDEADBEEF)
        load_program([
          Arm1Simulator.encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
          (COND_AL << 28) | 0x05D00000 | (1 << 16) | (0 << 12) | 0,
          Arm1Simulator.encode_halt
        ])
        @cpu.run(10)
        assert_equal 0xEF, @cpu.read_register(0)
      end

      def test_memory_read_write_word
        @cpu.write_word(0x100, 0xDEADBEEF)
        assert_equal 0xDEADBEEF, @cpu.read_word(0x100)
      end

      def test_memory_read_write_byte
        @cpu.write_byte(0x100, 0xAB)
        assert_equal 0xAB, @cpu.read_byte(0x100)
      end

      def test_memory_bounds
        assert_equal 0, @cpu.read_word(0xFFFFFFF0)
        assert_equal 0, @cpu.read_byte(0xFFFFFFF0)
      end
    end

    # =======================================================================
    # CPU — Block Transfer (LDM/STM)
    # =======================================================================

    class TestCPUBlockTransfer < Minitest::Test
      include ProgramLoader

      def setup
        @cpu = ARM1.new(4096)
      end

      def test_stm_ldm
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 10),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 20),
          Arm1Simulator.encode_mov_imm(COND_AL, 2, 30),
          Arm1Simulator.encode_mov_imm(COND_AL, 3, 40),
          Arm1Simulator.encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
          Arm1Simulator.encode_stm(COND_AL, 5, 0x000F, true, "IA"),
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 0),
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 0),
          Arm1Simulator.encode_mov_imm(COND_AL, 2, 0),
          Arm1Simulator.encode_mov_imm(COND_AL, 3, 0),
          Arm1Simulator.encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
          Arm1Simulator.encode_ldm(COND_AL, 5, 0x000F, true, "IA"),
          Arm1Simulator.encode_halt
        ])
        @cpu.run(50)
        assert_equal 10, @cpu.read_register(0)
        assert_equal 20, @cpu.read_register(1)
        assert_equal 30, @cpu.read_register(2)
        assert_equal 40, @cpu.read_register(3)
      end
    end

    # =======================================================================
    # CPU — Branch and Link
    # =======================================================================

    class TestCPUBranch < Minitest::Test
      include ProgramLoader

      def setup
        @cpu = ARM1.new(4096)
      end

      def test_branch_and_link
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 7),
          Arm1Simulator.encode_branch(COND_AL, true, 4),
          Arm1Simulator.encode_halt,
          0,
          Arm1Simulator.encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 0),
          Arm1Simulator.encode_data_processing(COND_AL, OP_MOV, 1, 0, 15, 14)
        ])
        @cpu.run(20)
        assert_equal 14, @cpu.read_register(0)
      end

      def test_simple_branch
        # B at 0x04, PC+8=0x0C. We want target=0x0C (skip the MOV R0,#99).
        # Offset from PC+8 = 0x0C - 0x0C = 0.
        load_program([
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 1),      # 0x00
          Arm1Simulator.encode_branch(COND_AL, false, 0),    # 0x04: B to 0x0C
          Arm1Simulator.encode_mov_imm(COND_AL, 0, 99),     # 0x08: skipped
          Arm1Simulator.encode_mov_imm(COND_AL, 1, 42),     # 0x0C: land here
          Arm1Simulator.encode_halt                           # 0x10
        ])
        @cpu.run(20)
        assert_equal 1, @cpu.read_register(0)
        assert_equal 42, @cpu.read_register(1)
      end
    end

    # =======================================================================
    # Encoding helpers
    # =======================================================================

    class TestEncodingHelpers < Minitest::Test
      def test_encode_halt
        inst = Arm1Simulator.encode_halt
        d = Arm1Simulator.decode(inst)
        assert_equal INST_SWI, d.type
        assert_equal HALT_SWI, d.swi_comment
      end

      def test_encode_mov_imm_roundtrip
        inst = Arm1Simulator.encode_mov_imm(COND_AL, 3, 99)
        d = Arm1Simulator.decode(inst)
        assert_equal OP_MOV, d.opcode
        assert_equal 3, d.rd
        assert_equal 99, d.imm8
      end

      def test_encode_branch_roundtrip
        inst = Arm1Simulator.encode_branch(COND_AL, true, 16)
        d = Arm1Simulator.decode(inst)
        assert_equal INST_BRANCH, d.type
        assert d.link
        assert_equal 16, d.branch_offset
      end

      def test_encode_branch_negative_offset
        inst = Arm1Simulator.encode_branch(COND_NE, false, -8)
        d = Arm1Simulator.decode(inst)
        assert_equal INST_BRANCH, d.type
        refute d.link
        assert_equal(-8, d.branch_offset)
      end
    end
  end
end
