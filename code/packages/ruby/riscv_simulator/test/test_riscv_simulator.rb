# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module RiscvSimulator
    # Helper methods
    def self.run_program(instructions)
      sim = RiscVSimulator.new
      program = assemble(instructions)
      sim.run(program)
      sim
    end

    class TestITypeArithmetic < Minitest::Test
      def test_addi
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 42),
          RiscvSimulator.encode_addi(2, 1, 10),
          RiscvSimulator.encode_addi(3, 0, -5),
          RiscvSimulator.encode_addi(4, 3, 3),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 42, sim.cpu.registers.read(1)
        assert_equal 52, sim.cpu.registers.read(2)
        assert_equal 0xFFFFFFFB, sim.cpu.registers.read(3) # -5 unsigned
        assert_equal 0xFFFFFFFE, sim.cpu.registers.read(4) # -2 unsigned
      end

      def test_slti
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_slti(2, 1, 10),
          RiscvSimulator.encode_slti(3, 1, 3),
          RiscvSimulator.encode_slti(4, 1, 5),
          RiscvSimulator.encode_addi(5, 0, -1),
          RiscvSimulator.encode_slti(6, 5, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 1, sim.cpu.registers.read(2)
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 0, sim.cpu.registers.read(4)
        assert_equal 1, sim.cpu.registers.read(6)
      end

      def test_sltiu
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_sltiu(2, 1, 10),
          RiscvSimulator.encode_sltiu(3, 1, 3),
          RiscvSimulator.encode_addi(4, 0, -1),
          RiscvSimulator.encode_sltiu(5, 4, 1),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 1, sim.cpu.registers.read(2)
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 0, sim.cpu.registers.read(5)
      end

      def test_xori
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0xFF),
          RiscvSimulator.encode_xori(2, 1, 0x0F),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xF0, sim.cpu.registers.read(2)
      end

      def test_ori
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x50),
          RiscvSimulator.encode_ori(2, 1, 0x0F),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x5F, sim.cpu.registers.read(2)
      end

      def test_andi
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0xFF),
          RiscvSimulator.encode_andi(2, 1, 0x0F),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x0F, sim.cpu.registers.read(2)
      end

      def test_slli
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_slli(2, 1, 4),
          RiscvSimulator.encode_slli(3, 1, 31),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 16, sim.cpu.registers.read(2)
        assert_equal 0x80000000, sim.cpu.registers.read(3)
      end

      def test_srli
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -1),
          RiscvSimulator.encode_srli(2, 1, 4),
          RiscvSimulator.encode_srli(3, 1, 31),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x0FFFFFFF, sim.cpu.registers.read(2)
        assert_equal 1, sim.cpu.registers.read(3)
      end

      def test_srai
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -16),
          RiscvSimulator.encode_srai(2, 1, 2),
          RiscvSimulator.encode_addi(3, 0, 16),
          RiscvSimulator.encode_srai(4, 3, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xFFFFFFFC, sim.cpu.registers.read(2) # -4
        assert_equal 4, sim.cpu.registers.read(4)
      end
    end

    class TestRTypeArithmetic < Minitest::Test
      def test_add_sub
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 10),
          RiscvSimulator.encode_addi(2, 0, 20),
          RiscvSimulator.encode_add(3, 1, 2),
          RiscvSimulator.encode_sub(4, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 30, sim.cpu.registers.read(3)
        assert_equal 0xFFFFFFF6, sim.cpu.registers.read(4) # -10
      end

      def test_sll
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_addi(2, 0, 8),
          RiscvSimulator.encode_sll(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 256, sim.cpu.registers.read(3)
      end

      def test_slt
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -5),
          RiscvSimulator.encode_addi(2, 0, 3),
          RiscvSimulator.encode_slt(3, 1, 2),
          RiscvSimulator.encode_slt(4, 2, 1),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 1, sim.cpu.registers.read(3)
        assert_equal 0, sim.cpu.registers.read(4)
      end

      def test_sltu
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -1),
          RiscvSimulator.encode_addi(2, 0, 1),
          RiscvSimulator.encode_sltu(3, 2, 1),
          RiscvSimulator.encode_sltu(4, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 1, sim.cpu.registers.read(3)
        assert_equal 0, sim.cpu.registers.read(4)
      end

      def test_xor
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0xFF),
          RiscvSimulator.encode_addi(2, 0, 0x0F),
          RiscvSimulator.encode_xor(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xF0, sim.cpu.registers.read(3)
      end

      def test_srl
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -1),
          RiscvSimulator.encode_addi(2, 0, 4),
          RiscvSimulator.encode_srl(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x0FFFFFFF, sim.cpu.registers.read(3)
      end

      def test_sra
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -16),
          RiscvSimulator.encode_addi(2, 0, 2),
          RiscvSimulator.encode_sra(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xFFFFFFFC, sim.cpu.registers.read(3)
      end

      def test_or
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x50),
          RiscvSimulator.encode_addi(2, 0, 0x0F),
          RiscvSimulator.encode_or(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x5F, sim.cpu.registers.read(3)
      end

      def test_and
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0xFF),
          RiscvSimulator.encode_addi(2, 0, 0x0F),
          RiscvSimulator.encode_and(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x0F, sim.cpu.registers.read(3)
      end
    end

    class TestLoadStore < Minitest::Test
      def test_store_word_load_word
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x100),
          RiscvSimulator.encode_addi(2, 0, 0x42),
          RiscvSimulator.encode_sw(2, 1, 0),
          RiscvSimulator.encode_lw(3, 1, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x42, sim.cpu.registers.read(3)
      end

      def test_store_byte_load_byte
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_addi(2, 0, 0xAB),
          RiscvSimulator.encode_sb(2, 1, 0),
          RiscvSimulator.encode_lbu(3, 1, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xAB, sim.cpu.registers.read(3)
      end

      def test_load_byte_sign_extend
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_addi(2, 0, 0xFF),
          RiscvSimulator.encode_sb(2, 1, 0),
          RiscvSimulator.encode_lb(3, 1, 0),
          RiscvSimulator.encode_lbu(4, 1, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xFFFFFFFF, sim.cpu.registers.read(3) # -1
        assert_equal 0xFF, sim.cpu.registers.read(4)
      end

      def test_store_half_load_half
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_lui(2, 0),
          RiscvSimulator.encode_addi(2, 0, 0x1FF),
          RiscvSimulator.encode_sh(2, 1, 0),
          RiscvSimulator.encode_lhu(3, 1, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x1FF, sim.cpu.registers.read(3)
      end

      def test_load_half_sign_extend
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_addi(2, 0, -1),
          RiscvSimulator.encode_sh(2, 1, 0),
          RiscvSimulator.encode_lh(3, 1, 0),
          RiscvSimulator.encode_lhu(4, 1, 0),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xFFFFFFFF, sim.cpu.registers.read(3) # -1
        assert_equal 0xFFFF, sim.cpu.registers.read(4)
      end

      def test_store_load_with_offset
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_addi(2, 0, 99),
          RiscvSimulator.encode_sw(2, 1, 4),
          RiscvSimulator.encode_lw(3, 1, 4),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 99, sim.cpu.registers.read(3)
      end
    end

    class TestBranches < Minitest::Test
      def test_beq_taken
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_addi(2, 0, 5),
          RiscvSimulator.encode_beq(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_beq_not_taken
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_addi(2, 0, 10),
          RiscvSimulator.encode_beq(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 42, sim.cpu.registers.read(3)
      end

      def test_bne
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_addi(2, 0, 10),
          RiscvSimulator.encode_bne(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_blt
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -5),
          RiscvSimulator.encode_addi(2, 0, 3),
          RiscvSimulator.encode_blt(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_bge
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_addi(2, 0, 5),
          RiscvSimulator.encode_bge(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_bltu
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_addi(2, 0, -1),
          RiscvSimulator.encode_bltu(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_bgeu
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -1),
          RiscvSimulator.encode_addi(2, 0, 1),
          RiscvSimulator.encode_bgeu(1, 2, 8),
          RiscvSimulator.encode_addi(3, 0, 999),
          RiscvSimulator.encode_addi(4, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(3)
        assert_equal 42, sim.cpu.registers.read(4)
      end

      def test_branch_backward
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0),
          RiscvSimulator.encode_addi(2, 0, 3),
          RiscvSimulator.encode_addi(1, 1, 1),
          RiscvSimulator.encode_bne(1, 2, -4),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 3, sim.cpu.registers.read(1)
      end
    end

    class TestJumps < Minitest::Test
      def test_jal
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_jal(1, 8),
          RiscvSimulator.encode_addi(2, 0, 999),
          RiscvSimulator.encode_addi(3, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 4, sim.cpu.registers.read(1)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 42, sim.cpu.registers.read(3)
      end

      def test_jalr
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(5, 0, 12),
          RiscvSimulator.encode_jalr(1, 5, 0),
          RiscvSimulator.encode_addi(2, 0, 999),
          RiscvSimulator.encode_addi(3, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 8, sim.cpu.registers.read(1)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 42, sim.cpu.registers.read(3)
      end

      def test_jalr_with_offset
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(5, 0, 8),
          RiscvSimulator.encode_jalr(1, 5, 4),
          RiscvSimulator.encode_addi(2, 0, 999),
          RiscvSimulator.encode_addi(3, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 8, sim.cpu.registers.read(1)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 42, sim.cpu.registers.read(3)
      end

      def test_call_and_return
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_jal(1, 12),
          RiscvSimulator.encode_addi(11, 0, 99),
          RiscvSimulator.encode_ecall,
          RiscvSimulator.encode_addi(10, 0, 42),
          RiscvSimulator.encode_jalr(0, 1, 0)
        ])
        assert_equal 4, sim.cpu.registers.read(1)
        assert_equal 42, sim.cpu.registers.read(10)
        assert_equal 99, sim.cpu.registers.read(11)
      end
    end

    class TestUpperImmediate < Minitest::Test
      def test_lui
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_lui(1, 0x12345),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x12345000, sim.cpu.registers.read(1)
      end

      def test_lui_plus_addi
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_lui(1, 0x12345),
          RiscvSimulator.encode_addi(1, 1, 0x678),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x12345678, sim.cpu.registers.read(1)
      end

      def test_auipc
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_auipc(1, 1),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x1000, sim.cpu.registers.read(1)
      end

      def test_auipc_non_zero_pc
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(0, 0, 0),
          RiscvSimulator.encode_auipc(1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0x2004, sim.cpu.registers.read(1)
      end
    end

    class TestRegisterZero < Minitest::Test
      def test_register_zero_hardwired
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(0, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(0)
      end

      def test_register_zero_on_r_type
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_addi(2, 0, 10),
          RiscvSimulator.encode_add(0, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0, sim.cpu.registers.read(0)
      end
    end

    class TestCSROperations < Minitest::Test
      def test_csrrw
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 0x100),
          RiscvSimulator.encode_csrrw(2, CSR_MSCRATCH, 1),
          RiscvSimulator.encode_csrrw(3, CSR_MSCRATCH, 0),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 0x100, sim.cpu.registers.read(3)
      end

      def test_csrrs
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 8),
          RiscvSimulator.encode_csrrs(2, CSR_MSTATUS, 1),
          RiscvSimulator.encode_csrrs(3, CSR_MSTATUS, 0),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 8, sim.cpu.registers.read(3)
      end

      def test_csrrc
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 0xFF),
          RiscvSimulator.encode_csrrw(0, CSR_MSCRATCH, 1),
          RiscvSimulator.encode_addi(2, 0, 0x0F),
          RiscvSimulator.encode_csrrc(3, CSR_MSCRATCH, 2),
          RiscvSimulator.encode_csrrs(4, CSR_MSCRATCH, 0),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)
        assert_equal 0xFF, sim.cpu.registers.read(3)
        assert_equal 0xF0, sim.cpu.registers.read(4)
      end
    end

    class TestEcallTrap < Minitest::Test
      def test_ecall_halt_when_no_trap_handler
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        assert sim.cpu.halted
        assert_equal 42, sim.cpu.registers.read(1)
      end

      def test_ecall_trap_with_handler
        sim = RiscVSimulator.new

        main_code = [
          RiscvSimulator.encode_addi(1, 0, 0x100),
          RiscvSimulator.encode_csrrw(0, CSR_MTVEC, 1),
          RiscvSimulator.encode_ecall,
          RiscvSimulator.encode_addi(11, 0, 77),
          RiscvSimulator.encode_csrrw(0, CSR_MTVEC, 0),
          RiscvSimulator.encode_ecall
        ]

        pad_count = (0x100 / 4) - main_code.size
        padded = main_code + Array.new(pad_count) { RiscvSimulator.encode_addi(0, 0, 0) }

        trap_handler = [
          RiscvSimulator.encode_addi(10, 0, 99),
          RiscvSimulator.encode_csrrs(20, CSR_MEPC, 0),
          RiscvSimulator.encode_addi(20, 20, 4),
          RiscvSimulator.encode_csrrw(0, CSR_MEPC, 20),
          RiscvSimulator.encode_mret
        ]
        padded += trap_handler

        program = RiscvSimulator.assemble(padded)
        sim.run(program)

        assert_equal 99, sim.cpu.registers.read(10)
        assert_equal 77, sim.cpu.registers.read(11)
        assert sim.cpu.halted
      end

      def test_ecall_sets_csrs
        sim = RiscVSimulator.new

        main_code = [
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_csrrw(0, CSR_MTVEC, 1),
          RiscvSimulator.encode_addi(2, 0, 8),
          RiscvSimulator.encode_csrrs(0, CSR_MSTATUS, 2),
          RiscvSimulator.encode_ecall
        ]

        pad_count = (0x200 / 4) - main_code.size
        padded = main_code + Array.new(pad_count) { RiscvSimulator.encode_addi(0, 0, 0) }

        trap_handler = [
          RiscvSimulator.encode_csrrs(20, CSR_MEPC, 0),
          RiscvSimulator.encode_csrrs(21, CSR_MCAUSE, 0),
          RiscvSimulator.encode_csrrs(22, CSR_MSTATUS, 0),
          RiscvSimulator.encode_csrrw(0, CSR_MTVEC, 0),
          RiscvSimulator.encode_ecall
        ]
        padded += trap_handler

        program = RiscvSimulator.assemble(padded)
        sim.run(program)

        assert_equal 16, sim.cpu.registers.read(20) # mepc
        assert_equal CAUSE_ECALL_M_MODE, sim.cpu.registers.read(21) # mcause
        assert_equal 0, sim.cpu.registers.read(22) & MIE # MIE cleared
      end
    end

    class TestMret < Minitest::Test
      def test_mret
        sim = RiscVSimulator.new
        sim.csr.write(CSR_MEPC, 12)

        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_mret,
          RiscvSimulator.encode_addi(1, 0, 999),
          RiscvSimulator.encode_addi(2, 0, 999),
          RiscvSimulator.encode_addi(3, 0, 42),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)

        assert_equal 0, sim.cpu.registers.read(1)
        assert_equal 0, sim.cpu.registers.read(2)
        assert_equal 42, sim.cpu.registers.read(3)
      end

      def test_mret_reenables_interrupts
        sim = RiscVSimulator.new
        sim.csr.write(CSR_MSTATUS, 0)
        sim.csr.write(CSR_MEPC, 4)

        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_mret,
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)

        refute_equal 0, sim.csr.read(CSR_MSTATUS) & MIE
      end
    end

    class TestMisc < Minitest::Test
      def test_unknown_instruction
        sim = RiscvSimulator.run_program([0xFFFFFFFF, RiscvSimulator.encode_ecall])
        assert_equal 0, sim.cpu.registers.read(1)
      end

      def test_negative_immediate
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, -5),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 0xFFFFFFFB, sim.cpu.registers.read(1)
      end
    end

    class TestIntegration < Minitest::Test
      def test_fibonacci
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 0),
          RiscvSimulator.encode_addi(2, 0, 1),
          RiscvSimulator.encode_addi(4, 0, 2),
          RiscvSimulator.encode_addi(5, 0, 11),
          RiscvSimulator.encode_add(3, 1, 2),
          RiscvSimulator.encode_addi(1, 2, 0),
          RiscvSimulator.encode_addi(2, 3, 0),
          RiscvSimulator.encode_addi(4, 4, 1),
          RiscvSimulator.encode_bne(4, 5, -16),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 55, sim.cpu.registers.read(2)
      end

      def test_memcpy
        sim = RiscVSimulator.new
        sim.cpu.memory.write_byte(0x200, 0xDE)
        sim.cpu.memory.write_byte(0x201, 0xAD)
        sim.cpu.memory.write_byte(0x202, 0xBE)
        sim.cpu.memory.write_byte(0x203, 0xEF)

        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 0x200),
          RiscvSimulator.encode_addi(2, 0, 0x300),
          RiscvSimulator.encode_lw(3, 1, 0),
          RiscvSimulator.encode_sw(3, 2, 0),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)

        4.times do |i|
          assert_equal sim.cpu.memory.read_byte(0x200 + i), sim.cpu.memory.read_byte(0x300 + i)
        end
      end

      def test_stack_operations
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(2, 0, 0x400),
          RiscvSimulator.encode_addi(10, 0, 42),
          RiscvSimulator.encode_addi(11, 0, 99),
          RiscvSimulator.encode_addi(2, 2, -4),
          RiscvSimulator.encode_sw(10, 2, 0),
          RiscvSimulator.encode_addi(2, 2, -4),
          RiscvSimulator.encode_sw(11, 2, 0),
          RiscvSimulator.encode_lw(12, 2, 0),
          RiscvSimulator.encode_addi(2, 2, 4),
          RiscvSimulator.encode_lw(13, 2, 0),
          RiscvSimulator.encode_addi(2, 2, 4),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 99, sim.cpu.registers.read(12)
        assert_equal 42, sim.cpu.registers.read(13)
        assert_equal 0x400, sim.cpu.registers.read(2)
      end
    end

    class TestStepExecution < Minitest::Test
      def test_step
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_addi(2, 0, 2),
          RiscvSimulator.encode_ecall
        ])
        sim.cpu.load_program(program)

        trace1 = sim.step
        assert_equal "addi", trace1.decode.mnemonic
        assert_equal 1, sim.cpu.registers.read(1)

        trace2 = sim.step
        assert_equal "addi", trace2.decode.mnemonic
        assert_equal 2, sim.cpu.registers.read(2)
      end
    end

    class TestEncodeDecodeRoundTrip < Minitest::Test
      def test_round_trip
        decoder = RiscVDecoder.new
        cases = {
          "addi" => RiscvSimulator.encode_addi(1, 2, 42),
          "slti" => RiscvSimulator.encode_slti(1, 2, -5),
          "sltiu" => RiscvSimulator.encode_sltiu(1, 2, 5),
          "xori" => RiscvSimulator.encode_xori(1, 2, 0xFF),
          "ori" => RiscvSimulator.encode_ori(1, 2, 0xFF),
          "andi" => RiscvSimulator.encode_andi(1, 2, 0xFF),
          "slli" => RiscvSimulator.encode_slli(1, 2, 5),
          "srli" => RiscvSimulator.encode_srli(1, 2, 5),
          "srai" => RiscvSimulator.encode_srai(1, 2, 5),
          "add" => RiscvSimulator.encode_add(1, 2, 3),
          "sub" => RiscvSimulator.encode_sub(1, 2, 3),
          "sll" => RiscvSimulator.encode_sll(1, 2, 3),
          "slt" => RiscvSimulator.encode_slt(1, 2, 3),
          "sltu" => RiscvSimulator.encode_sltu(1, 2, 3),
          "xor" => RiscvSimulator.encode_xor(1, 2, 3),
          "srl" => RiscvSimulator.encode_srl(1, 2, 3),
          "sra" => RiscvSimulator.encode_sra(1, 2, 3),
          "or" => RiscvSimulator.encode_or(1, 2, 3),
          "and" => RiscvSimulator.encode_and(1, 2, 3),
          "lb" => RiscvSimulator.encode_lb(1, 2, 4),
          "lh" => RiscvSimulator.encode_lh(1, 2, 4),
          "lw" => RiscvSimulator.encode_lw(1, 2, 4),
          "lbu" => RiscvSimulator.encode_lbu(1, 2, 4),
          "lhu" => RiscvSimulator.encode_lhu(1, 2, 4),
          "sb" => RiscvSimulator.encode_sb(3, 2, 4),
          "sh" => RiscvSimulator.encode_sh(3, 2, 4),
          "sw" => RiscvSimulator.encode_sw(3, 2, 4),
          "beq" => RiscvSimulator.encode_beq(1, 2, 8),
          "bne" => RiscvSimulator.encode_bne(1, 2, 8),
          "blt" => RiscvSimulator.encode_blt(1, 2, 8),
          "bge" => RiscvSimulator.encode_bge(1, 2, 8),
          "bltu" => RiscvSimulator.encode_bltu(1, 2, 8),
          "bgeu" => RiscvSimulator.encode_bgeu(1, 2, 8),
          "jal" => RiscvSimulator.encode_jal(1, 8),
          "jalr" => RiscvSimulator.encode_jalr(1, 2, 4),
          "lui" => RiscvSimulator.encode_lui(1, 0x12345),
          "auipc" => RiscvSimulator.encode_auipc(1, 0x12345),
          "ecall" => RiscvSimulator.encode_ecall,
          "mret" => RiscvSimulator.encode_mret,
          "csrrw" => RiscvSimulator.encode_csrrw(1, 0x300, 2),
          "csrrs" => RiscvSimulator.encode_csrrs(1, 0x300, 2),
          "csrrc" => RiscvSimulator.encode_csrrc(1, 0x300, 2)
        }
        cases.each do |name, encoded|
          result = decoder.decode(encoded, 0)
          assert_equal name, result.mnemonic, "Decode(#{name}) failed"
        end
      end
    end

    class TestCSRFile < Minitest::Test
      def test_read_write
        csr = CSRFile.new
        assert_equal 0, csr.read(CSR_MSTATUS)
        csr.write(CSR_MSTATUS, 0x1234)
        assert_equal 0x1234, csr.read(CSR_MSTATUS)
      end

      def test_read_write_atomic
        csr = CSRFile.new
        csr.write(CSR_MSCRATCH, 42)
        old = csr.read_write(CSR_MSCRATCH, 99)
        assert_equal 42, old
        assert_equal 99, csr.read(CSR_MSCRATCH)
      end

      def test_read_set
        csr = CSRFile.new
        csr.write(CSR_MSTATUS, 0xF0)
        old = csr.read_set(CSR_MSTATUS, 0x0F)
        assert_equal 0xF0, old
        assert_equal 0xFF, csr.read(CSR_MSTATUS)
      end

      def test_read_clear
        csr = CSRFile.new
        csr.write(CSR_MSTATUS, 0xFF)
        old = csr.read_clear(CSR_MSTATUS, 0x0F)
        assert_equal 0xFF, old
        assert_equal 0xF0, csr.read(CSR_MSTATUS)
      end
    end

    class TestEdgeCases < Minitest::Test
      def test_shift_amount_masking
        sim = RiscvSimulator.run_program([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_addi(2, 0, 33),
          RiscvSimulator.encode_sll(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        assert_equal 2, sim.cpu.registers.read(3)
      end

      def test_assemble
        data = RiscvSimulator.assemble([0x12345678])
        assert_equal 4, data.bytesize
        assert_equal 0x78, data.bytes[0]
        assert_equal 0x56, data.bytes[1]
        assert_equal 0x34, data.bytes[2]
        assert_equal 0x12, data.bytes[3]
      end
    end
  end
end
