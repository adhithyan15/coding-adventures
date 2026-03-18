# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module ArmSimulator
    class TestEncoding < Minitest::Test
      def test_encode_mov_imm
        encoded = ArmSimulator.encode_mov_imm(0, 1)
        assert_equal 0xE3A00001, encoded
      end

      def test_encode_mov_imm_larger_value
        encoded = ArmSimulator.encode_mov_imm(1, 255)
        assert_equal 0xE3A010FF, encoded
      end

      def test_encode_add
        encoded = ArmSimulator.encode_add(2, 0, 1)
        assert_equal 0xE0802001, encoded
      end

      def test_encode_sub
        encoded = ArmSimulator.encode_sub(2, 0, 1)
        assert_equal 0xE0402001, encoded
      end

      def test_encode_hlt
        assert_equal 0xFFFFFFFF, ArmSimulator.encode_hlt
      end

      def test_assemble
        program = ArmSimulator.assemble([0xE3A00001, 0xFFFFFFFF])
        assert_equal 8, program.bytesize
        assert_equal Encoding::ASCII_8BIT, program.encoding
      end
    end

    class TestDecoder < Minitest::Test
      def setup
        @decoder = ARMDecoder.new
      end

      def test_decode_hlt
        result = @decoder.decode(HLT_INSTRUCTION, 0)
        assert_equal "hlt", result.mnemonic
      end

      def test_decode_mov_imm
        raw = ArmSimulator.encode_mov_imm(0, 1)
        result = @decoder.decode(raw, 0)
        assert_equal "mov", result.mnemonic
        assert_equal 0, result.fields[:rd]
        assert_equal 1, result.fields[:imm]
        assert_equal 1, result.fields[:i_bit]
      end

      def test_decode_add_register
        raw = ArmSimulator.encode_add(2, 0, 1)
        result = @decoder.decode(raw, 0)
        assert_equal "add", result.mnemonic
        assert_equal 2, result.fields[:rd]
        assert_equal 0, result.fields[:rn]
        assert_equal 1, result.fields[:rm]
        assert_equal 0, result.fields[:i_bit]
      end

      def test_decode_sub_register
        raw = ArmSimulator.encode_sub(3, 1, 2)
        result = @decoder.decode(raw, 0)
        assert_equal "sub", result.mnemonic
        assert_equal 3, result.fields[:rd]
        assert_equal 1, result.fields[:rn]
        assert_equal 2, result.fields[:rm]
      end

      def test_decode_unknown_opcode
        # Create an instruction with an unsupported opcode (0b0000)
        raw = (COND_AL << 28) | (0b0000 << 21)
        result = @decoder.decode(raw, 0)
        assert_includes result.mnemonic, "dp_op"
      end
    end

    class TestExecutor < Minitest::Test
      def setup
        @executor = ARMExecutor.new
        @registers = CpuSimulator::RegisterFile.new(num_registers: 16)
        @memory = CpuSimulator::Memory.new
      end

      def test_execute_hlt
        decoded = CpuSimulator::DecodeResult.new(
          mnemonic: "hlt", fields: {}, raw_instruction: HLT_INSTRUCTION
        )
        result = @executor.execute(decoded, @registers, @memory, 0)
        assert result.halted
      end

      def test_execute_unknown
        decoded = CpuSimulator::DecodeResult.new(
          mnemonic: "nop_x", fields: {}, raw_instruction: 0
        )
        result = @executor.execute(decoded, @registers, @memory, 0)
        assert_equal 4, result.next_pc
        assert_includes result.description, "Unknown"
      end
    end

    class TestARMSimulator < Minitest::Test
      def test_x_equals_1_plus_2
        sim = ARMSimulator.new
        program = ArmSimulator.assemble([
          ArmSimulator.encode_mov_imm(0, 1),
          ArmSimulator.encode_mov_imm(1, 2),
          ArmSimulator.encode_add(2, 0, 1),
          ArmSimulator.encode_hlt
        ])
        traces = sim.run(program)

        assert_equal 4, traces.size
        assert_equal 3, sim.cpu.registers.read(2)
        assert sim.cpu.halted
      end

      def test_subtraction
        sim = ARMSimulator.new
        program = ArmSimulator.assemble([
          ArmSimulator.encode_mov_imm(0, 10),
          ArmSimulator.encode_mov_imm(1, 3),
          ArmSimulator.encode_sub(2, 0, 1),
          ArmSimulator.encode_hlt
        ])
        traces = sim.run(program)

        assert_equal 7, sim.cpu.registers.read(2)
      end

      def test_step_by_step
        sim = ARMSimulator.new
        program = ArmSimulator.assemble([
          ArmSimulator.encode_mov_imm(0, 5),
          ArmSimulator.encode_hlt
        ])
        sim.cpu.load_program(program)

        trace = sim.step
        assert_equal "mov", trace.decode.mnemonic
        assert_equal 5, sim.cpu.registers.read(0)

        trace = sim.step
        assert_equal "hlt", trace.decode.mnemonic
        assert sim.cpu.halted
      end

      def test_pipeline_trace_fields
        sim = ARMSimulator.new
        program = ArmSimulator.assemble([
          ArmSimulator.encode_mov_imm(0, 1),
          ArmSimulator.encode_hlt
        ])
        sim.cpu.load_program(program)
        trace = sim.step
        assert_equal 0, trace.cycle
        assert_equal 0, trace.fetch.pc
        assert_equal "mov", trace.decode.mnemonic
        assert_includes trace.execute.description, "R0"
      end
    end
  end
end
