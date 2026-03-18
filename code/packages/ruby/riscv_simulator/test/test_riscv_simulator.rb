# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module RiscvSimulator
    class TestEncoding < Minitest::Test
      def test_encode_addi
        encoded = RiscvSimulator.encode_addi(1, 0, 1)
        assert_equal 0x00100093, encoded
      end

      def test_encode_add
        encoded = RiscvSimulator.encode_add(3, 1, 2)
        assert_equal 0x002081B3, encoded
      end

      def test_encode_sub
        encoded = RiscvSimulator.encode_sub(3, 1, 2)
        # funct7=0x20, rs2=2, rs1=1, funct3=0, rd=3, opcode=0110011
        expected = (0x20 << 25) | (2 << 20) | (1 << 15) | (0 << 12) | (3 << 7) | 0b0110011
        assert_equal expected, encoded
      end

      def test_encode_ecall
        assert_equal 0b1110011, RiscvSimulator.encode_ecall
      end

      def test_assemble
        program = RiscvSimulator.assemble([0x00100093, 0x73])
        assert_equal 8, program.bytesize
      end
    end

    class TestDecoder < Minitest::Test
      def setup
        @decoder = RiscVDecoder.new
      end

      def test_decode_addi
        raw = RiscvSimulator.encode_addi(1, 0, 1)
        result = @decoder.decode(raw, 0)
        assert_equal "addi", result.mnemonic
        assert_equal 1, result.fields[:rd]
        assert_equal 0, result.fields[:rs1]
        assert_equal 1, result.fields[:imm]
      end

      def test_decode_addi_negative
        raw = RiscvSimulator.encode_addi(1, 0, -1)
        result = @decoder.decode(raw, 0)
        assert_equal "addi", result.mnemonic
        assert_equal(-1, result.fields[:imm])
      end

      def test_decode_add
        raw = RiscvSimulator.encode_add(3, 1, 2)
        result = @decoder.decode(raw, 0)
        assert_equal "add", result.mnemonic
        assert_equal 3, result.fields[:rd]
        assert_equal 1, result.fields[:rs1]
        assert_equal 2, result.fields[:rs2]
      end

      def test_decode_sub
        raw = RiscvSimulator.encode_sub(3, 1, 2)
        result = @decoder.decode(raw, 0)
        assert_equal "sub", result.mnemonic
      end

      def test_decode_ecall
        result = @decoder.decode(RiscvSimulator.encode_ecall, 0)
        assert_equal "ecall", result.mnemonic
      end

      def test_decode_unknown
        result = @decoder.decode(0b1111111, 0)
        assert_includes result.mnemonic, "UNKNOWN"
      end

      def test_decode_unknown_r_type
        # R-type with unsupported funct3/funct7
        raw = (0x01 << 25) | (0 << 20) | (0 << 15) | (1 << 12) | (0 << 7) | OPCODE_OP
        result = @decoder.decode(raw, 0)
        assert_includes result.mnemonic, "r_op"
      end
    end

    class TestExecutor < Minitest::Test
      def setup
        @executor = RiscVExecutor.new
        @registers = CpuSimulator::RegisterFile.new(num_registers: 32)
        @memory = CpuSimulator::Memory.new
      end

      def test_exec_unknown
        decoded = CpuSimulator::DecodeResult.new(
          mnemonic: "nope", fields: {}, raw_instruction: 0
        )
        result = @executor.execute(decoded, @registers, @memory, 0)
        assert_equal 4, result.next_pc
      end
    end

    class TestRiscVSimulator < Minitest::Test
      def test_x_equals_1_plus_2
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 1),
          RiscvSimulator.encode_addi(2, 0, 2),
          RiscvSimulator.encode_add(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        traces = sim.run(program)

        assert_equal 4, traces.size
        assert_equal 3, sim.cpu.registers.read(3)
        assert sim.cpu.halted
      end

      def test_x0_stays_zero
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(0, 0, 42), # write to x0 should be ignored
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)
        assert_equal 0, sim.cpu.registers.read(0)
      end

      def test_subtraction
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 10),
          RiscvSimulator.encode_addi(2, 0, 3),
          RiscvSimulator.encode_sub(3, 1, 2),
          RiscvSimulator.encode_ecall
        ])
        sim.run(program)
        assert_equal 7, sim.cpu.registers.read(3)
      end

      def test_step
        sim = RiscVSimulator.new
        program = RiscvSimulator.assemble([
          RiscvSimulator.encode_addi(1, 0, 5),
          RiscvSimulator.encode_ecall
        ])
        sim.cpu.load_program(program)
        trace = sim.step
        assert_equal "addi", trace.decode.mnemonic
      end
    end
  end
end
