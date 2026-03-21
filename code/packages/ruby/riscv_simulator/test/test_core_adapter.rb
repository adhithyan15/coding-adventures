# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module RiscvSimulator
    # === Simple register file for testing ===

    class SimpleRegisterFile
      def initialize(count = 32)
        @regs = Array.new(count, 0)
      end

      def read(index)
        return 0 if index < 0 || index >= @regs.size

        @regs[index]
      end

      def write(index, value)
        @regs[index] = value if index > 0 && index < @regs.size
      end
    end

    # === Helper to create tokens ===

    def self.make_token(pc: 0)
      {
        pc: pc,
        opcode: "",
        rd: -1,
        rs1: -1,
        rs2: -1,
        immediate: 0,
        alu_result: 0,
        write_data: 0,
        branch_taken: false,
        branch_target: 0,
        reg_write: false,
        mem_read: false,
        mem_write: false,
        is_branch: false,
        is_halt: false,
        raw_instruction: 0
      }
    end

    # === Tests ===

    class TestRiscVISADecoder < Minitest::Test
      def test_instruction_size
        decoder = RiscVISADecoder.new
        assert_equal 4, decoder.instruction_size
      end

      def test_csr_accessor
        decoder = RiscVISADecoder.new
        refute_nil decoder.csr
      end

      def test_factory_function
        decoder = RiscvSimulator.new_riscv_core
        assert_instance_of RiscVISADecoder, decoder
      end
    end

    class TestDecodeControlSignals < Minitest::Test
      def check_signals(raw, reg_write: false, mem_read: false, mem_write: false,
        is_branch: false, is_halt: false)
        decoder = RiscVISADecoder.new
        token = RiscvSimulator.make_token
        decoder.decode(raw, token)
        assert_equal reg_write, token[:reg_write] || false, "reg_write for #{token[:opcode]}"
        assert_equal mem_read, token[:mem_read] || false, "mem_read for #{token[:opcode]}"
        assert_equal mem_write, token[:mem_write] || false, "mem_write for #{token[:opcode]}"
        assert_equal is_branch, token[:is_branch] || false, "is_branch for #{token[:opcode]}"
        assert_equal is_halt, token[:is_halt] || false, "is_halt for #{token[:opcode]}"
      end

      def test_r_type_arithmetic
        [
          RiscvSimulator.encode_add(3, 1, 2),
          RiscvSimulator.encode_sub(3, 1, 2),
          RiscvSimulator.encode_sll(3, 1, 2),
          RiscvSimulator.encode_slt(3, 1, 2),
          RiscvSimulator.encode_sltu(3, 1, 2),
          RiscvSimulator.encode_xor(3, 1, 2),
          RiscvSimulator.encode_srl(3, 1, 2),
          RiscvSimulator.encode_sra(3, 1, 2),
          RiscvSimulator.encode_or(3, 1, 2),
          RiscvSimulator.encode_and(3, 1, 2)
        ].each { |raw| check_signals(raw, reg_write: true) }
      end

      def test_i_type_arithmetic
        [
          RiscvSimulator.encode_addi(1, 2, 5),
          RiscvSimulator.encode_slti(1, 2, 5),
          RiscvSimulator.encode_sltiu(1, 2, 5),
          RiscvSimulator.encode_xori(1, 2, 5),
          RiscvSimulator.encode_ori(1, 2, 5),
          RiscvSimulator.encode_andi(1, 2, 5),
          RiscvSimulator.encode_slli(1, 2, 3),
          RiscvSimulator.encode_srli(1, 2, 3),
          RiscvSimulator.encode_srai(1, 2, 3)
        ].each { |raw| check_signals(raw, reg_write: true) }
      end

      def test_upper_immediate
        check_signals(RiscvSimulator.encode_lui(1, 0x12345), reg_write: true)
        check_signals(RiscvSimulator.encode_auipc(1, 0x12345), reg_write: true)
      end

      def test_loads
        [
          RiscvSimulator.encode_lb(1, 2, 0),
          RiscvSimulator.encode_lh(1, 2, 0),
          RiscvSimulator.encode_lw(1, 2, 0),
          RiscvSimulator.encode_lbu(1, 2, 0),
          RiscvSimulator.encode_lhu(1, 2, 0)
        ].each { |raw| check_signals(raw, reg_write: true, mem_read: true) }
      end

      def test_stores
        [
          RiscvSimulator.encode_sb(1, 2, 0),
          RiscvSimulator.encode_sh(1, 2, 0),
          RiscvSimulator.encode_sw(1, 2, 0)
        ].each { |raw| check_signals(raw, mem_write: true) }
      end

      def test_branches
        [
          RiscvSimulator.encode_beq(1, 2, 8),
          RiscvSimulator.encode_bne(1, 2, 8),
          RiscvSimulator.encode_blt(1, 2, 8),
          RiscvSimulator.encode_bge(1, 2, 8),
          RiscvSimulator.encode_bltu(1, 2, 8),
          RiscvSimulator.encode_bgeu(1, 2, 8)
        ].each { |raw| check_signals(raw, is_branch: true) }
      end

      def test_jumps
        check_signals(RiscvSimulator.encode_jal(1, 8), reg_write: true, is_branch: true)
        check_signals(RiscvSimulator.encode_jalr(1, 2, 0), reg_write: true, is_branch: true)
      end

      def test_ecall
        check_signals(RiscvSimulator.encode_ecall, is_halt: true)
      end

      def test_csr_instructions
        check_signals(RiscvSimulator.encode_csrrw(1, 0x300, 2), reg_write: true)
        check_signals(RiscvSimulator.encode_csrrs(1, 0x300, 2), reg_write: true)
        check_signals(RiscvSimulator.encode_csrrc(1, 0x300, 2), reg_write: true)
      end

      def test_mret
        check_signals(RiscvSimulator.encode_mret, is_branch: true)
      end
    end

    class TestExecuteDirectly < Minitest::Test
      def test_alu_operations
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 10)
        regs.write(2, 3)

        cases = [
          ["add", RiscvSimulator.encode_add(3, 1, 2), 13],
          ["sub", RiscvSimulator.encode_sub(3, 1, 2), 7],
          ["sll", RiscvSimulator.encode_sll(3, 1, 2), 80],
          ["srl", RiscvSimulator.encode_srl(3, 1, 2), 1],
          ["xor", RiscvSimulator.encode_xor(3, 1, 2), 10 ^ 3],
          ["or", RiscvSimulator.encode_or(3, 1, 2), 10 | 3],
          ["and", RiscvSimulator.encode_and(3, 1, 2), 10 & 3],
          ["addi", RiscvSimulator.encode_addi(3, 1, 5), 15],
          ["slli", RiscvSimulator.encode_slli(3, 1, 2), 40],
          ["srli", RiscvSimulator.encode_srli(3, 1, 1), 5],
          ["lui", RiscvSimulator.encode_lui(3, 1), 4096],
          ["lw", RiscvSimulator.encode_lw(3, 1, 4), 14],
          ["sw", RiscvSimulator.encode_sw(2, 1, 8), 18]
        ]

        cases.each do |name, raw, expected_alu|
          token = RiscvSimulator.make_token
          decoder.decode(raw, token)
          decoder.execute(token, regs)
          assert_equal expected_alu, token[:alu_result], "#{name}: ALU result"
        end
      end

      def test_beq_not_taken
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 10)
        regs.write(2, 3)

        token = RiscvSimulator.make_token
        decoder.decode(RiscvSimulator.encode_beq(1, 2, 100), token)
        decoder.execute(token, regs)
        assert_equal 4, token[:alu_result]
      end
    end

    class TestBranchExecution < Minitest::Test
      def test_branches
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 5)
        regs.write(2, 5)
        regs.write(3, 10)

        cases = [
          ["beq_taken", RiscvSimulator.encode_beq(1, 2, 20), true, 20],
          ["beq_not_taken", RiscvSimulator.encode_beq(1, 3, 20), false, 0],
          ["bne_taken", RiscvSimulator.encode_bne(1, 3, 20), true, 20],
          ["bne_not_taken", RiscvSimulator.encode_bne(1, 2, 20), false, 0],
          ["blt_taken", RiscvSimulator.encode_blt(1, 3, 20), true, 20],
          ["blt_not_taken", RiscvSimulator.encode_blt(3, 1, 20), false, 0],
          ["bge_taken", RiscvSimulator.encode_bge(3, 1, 20), true, 20],
          ["bge_not_taken", RiscvSimulator.encode_bge(1, 3, 20), false, 0],
          ["bltu_taken", RiscvSimulator.encode_bltu(1, 3, 20), true, 20],
          ["bgeu_taken", RiscvSimulator.encode_bgeu(3, 1, 20), true, 20]
        ]

        cases.each do |name, raw, expected_taken, expected_target|
          token = RiscvSimulator.make_token
          decoder.decode(raw, token)
          decoder.execute(token, regs)
          assert_equal expected_taken, token[:branch_taken], "#{name}: branch_taken"
          assert_equal expected_target, token[:branch_target], "#{name}: branch_target" if expected_taken
        end
      end
    end

    class TestJumpExecution < Minitest::Test
      def test_jal
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new

        token = RiscvSimulator.make_token(pc: 8)
        decoder.decode(RiscvSimulator.encode_jal(1, 20), token)
        decoder.execute(token, regs)

        assert token[:branch_taken]
        assert_equal 28, token[:branch_target]
        assert_equal 12, token[:write_data]
      end

      def test_jalr
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(5, 100)

        token = RiscvSimulator.make_token(pc: 16)
        decoder.decode(RiscvSimulator.encode_jalr(1, 5, 8), token)
        decoder.execute(token, regs)

        assert token[:branch_taken]
        assert_equal 108, token[:branch_target]
        assert_equal 20, token[:write_data]
      end
    end

    class TestSetLessThan < Minitest::Test
      def test_slt
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 5)
        regs.write(2, 10)

        token = RiscvSimulator.make_token
        decoder.decode(RiscvSimulator.encode_slt(3, 1, 2), token)
        decoder.execute(token, regs)
        assert_equal 1, token[:alu_result]
      end

      def test_sltu
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 5)
        regs.write(2, 10)

        token = RiscvSimulator.make_token
        decoder.decode(RiscvSimulator.encode_sltu(3, 1, 2), token)
        decoder.execute(token, regs)
        assert_equal 1, token[:alu_result]
      end
    end

    class TestSRA < Minitest::Test
      def test_sra_negative
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(1, 0xFFFFFFF0) # -16
        regs.write(2, 2)

        token = RiscvSimulator.make_token
        decoder.decode(RiscvSimulator.encode_sra(3, 1, 2), token)
        decoder.execute(token, regs)
        assert_equal(-4, token[:alu_result])
      end
    end

    class TestUnknownInstruction < Minitest::Test
      def test_no_crash
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new

        token = RiscvSimulator.make_token
        token[:opcode] = "SOMETHING_UNKNOWN"
        decoder.execute(token, regs)
      end
    end

    class TestCSRExtraction < Minitest::Test
      def test_csrrw_csr_address
        decoder = RiscVISADecoder.new
        regs = SimpleRegisterFile.new
        regs.write(2, 42)

        raw = RiscvSimulator.encode_csrrw(1, 0x300, 2)
        token = RiscvSimulator.make_token
        token[:raw_instruction] = raw
        decoder.decode(raw, token)
        decoder.execute(token, regs)

        assert_equal 0, token[:alu_result]
        assert_equal 42, decoder.csr.read(0x300)
      end
    end

    class TestSparseMemoryIntegration < Minitest::Test
      def test_sparse_memory_program_storage
        mem = CpuSimulator::SparseMemory.new([
          CpuSimulator::MemoryRegion.new(base: 0x00000000, size: 0x10000, name: "RAM"),
          CpuSimulator::MemoryRegion.new(base: 0xFFFF0000, size: 0x100, name: "ROM", read_only: true)
        ])

        program = [RiscvSimulator.encode_addi(1, 0, 42), RiscvSimulator.encode_ecall]
        bytes = program.map { |i| [i & 0xFFFFFFFF].pack("V") }.join.bytes.to_a
        mem.load_bytes(0, bytes)

        refute_equal 0, mem.read_word(0)
        assert_equal 0, mem.read_byte(0xFFFF0000)
      end
    end
  end
end
