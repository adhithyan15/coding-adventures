# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module CpuSimulator
    # === A minimal decoder/executor for testing ===
    #
    # We need a concrete ISA to test the generic CPU. This tiny instruction set
    # has just three operations encoded as 32-bit words:
    #
    #   0x0100VVRR  ->  LOAD_IMM  Rd=RR, value=VV   (Rd = VV)
    #   0x02RRSSDD  ->  ADD       Rd=DD, Rs1=RR, Rs2=SS  (Rd = Rs1 + Rs2)
    #   0xFFFFFFFF  ->  HALT
    class TestDecoder
      def decode(raw, pc)
        if raw == 0xFFFFFFFF
          DecodeResult.new(mnemonic: "halt", fields: {}, raw_instruction: raw)
        elsif (raw >> 24) == 0x01
          rd = raw & 0xFF
          imm = (raw >> 8) & 0xFF
          DecodeResult.new(mnemonic: "load_imm", fields: {rd: rd, imm: imm}, raw_instruction: raw)
        elsif (raw >> 24) == 0x02
          rd = raw & 0xFF
          rs2 = (raw >> 8) & 0xFF
          rs1 = (raw >> 16) & 0xFF
          DecodeResult.new(mnemonic: "add", fields: {rd: rd, rs1: rs1, rs2: rs2}, raw_instruction: raw)
        else
          DecodeResult.new(mnemonic: "unknown", fields: {opcode: raw}, raw_instruction: raw)
        end
      end
    end

    class TestExecutor
      def execute(decoded, registers, memory, pc)
        case decoded.mnemonic
        when "load_imm"
          rd = decoded.fields[:rd]
          imm = decoded.fields[:imm]
          registers.write(rd, imm)
          ExecuteResult.new(
            description: "R#{rd} = #{imm}",
            registers_changed: {"R#{rd}" => imm},
            memory_changed: {},
            next_pc: pc + 4
          )
        when "add"
          rd = decoded.fields[:rd]
          rs1 = decoded.fields[:rs1]
          rs2 = decoded.fields[:rs2]
          v1 = registers.read(rs1)
          v2 = registers.read(rs2)
          result = (v1 + v2) & 0xFFFFFFFF
          registers.write(rd, result)
          ExecuteResult.new(
            description: "R#{rd} = R#{rs1}(#{v1}) + R#{rs2}(#{v2}) = #{result}",
            registers_changed: {"R#{rd}" => result},
            memory_changed: {},
            next_pc: pc + 4
          )
        when "halt"
          ExecuteResult.new(
            description: "Halt",
            registers_changed: {},
            memory_changed: {},
            next_pc: pc,
            halted: true
          )
        else
          ExecuteResult.new(
            description: "Unknown",
            registers_changed: {},
            memory_changed: {},
            next_pc: pc + 4
          )
        end
      end
    end

    # -----------------------------------------------------------------------
    # RegisterFile tests
    # -----------------------------------------------------------------------
    class TestRegisterFile < Minitest::Test
      def test_initial_values_are_zero
        regs = RegisterFile.new(num_registers: 4, bit_width: 32)
        4.times { |i| assert_equal 0, regs.read(i) }
      end

      def test_write_and_read
        regs = RegisterFile.new(num_registers: 4)
        regs.write(1, 42)
        assert_equal 42, regs.read(1)
      end

      def test_value_masked_to_bit_width
        regs = RegisterFile.new(num_registers: 4, bit_width: 8)
        regs.write(0, 256) # 256 doesn't fit in 8 bits
        assert_equal 0, regs.read(0) # wrapped: 256 & 0xFF = 0
      end

      def test_out_of_range_read
        regs = RegisterFile.new(num_registers: 4)
        assert_raises(IndexError) { regs.read(4) }
        assert_raises(IndexError) { regs.read(-1) }
      end

      def test_out_of_range_write
        regs = RegisterFile.new(num_registers: 4)
        assert_raises(IndexError) { regs.write(4, 0) }
        assert_raises(IndexError) { regs.write(-1, 0) }
      end

      def test_dump
        regs = RegisterFile.new(num_registers: 4)
        regs.write(1, 5)
        expected = {"R0" => 0, "R1" => 5, "R2" => 0, "R3" => 0}
        assert_equal expected, regs.dump
      end

      def test_invalid_num_registers
        assert_raises(ArgumentError) { RegisterFile.new(num_registers: 0) }
      end

      def test_invalid_bit_width
        assert_raises(ArgumentError) { RegisterFile.new(bit_width: 0) }
      end
    end

    # -----------------------------------------------------------------------
    # Memory tests
    # -----------------------------------------------------------------------
    class TestMemory < Minitest::Test
      def test_initial_values_are_zero
        mem = Memory.new(size: 16)
        assert_equal 0, mem.read_byte(0)
      end

      def test_write_and_read_byte
        mem = Memory.new(size: 16)
        mem.write_byte(0, 42)
        assert_equal 42, mem.read_byte(0)
      end

      def test_byte_masked
        mem = Memory.new(size: 16)
        mem.write_byte(0, 256)
        assert_equal 0, mem.read_byte(0)
      end

      def test_read_and_write_word
        mem = Memory.new(size: 16)
        mem.write_word(0, 0x12345678)
        assert_equal 0x12345678, mem.read_word(0)
      end

      def test_little_endian_byte_order
        mem = Memory.new(size: 16)
        mem.write_word(0, 0x12345678)
        assert_equal 0x78, mem.read_byte(0) # LSB first
        assert_equal 0x56, mem.read_byte(1)
        assert_equal 0x34, mem.read_byte(2)
        assert_equal 0x12, mem.read_byte(3) # MSB last
      end

      def test_load_bytes
        mem = Memory.new(size: 16)
        mem.load_bytes(0, "\x01\x02\x03".b)
        assert_equal 1, mem.read_byte(0)
        assert_equal 2, mem.read_byte(1)
        assert_equal 3, mem.read_byte(2)
      end

      def test_out_of_bounds
        mem = Memory.new(size: 16)
        assert_raises(IndexError) { mem.read_byte(16) }
        assert_raises(IndexError) { mem.write_byte(16, 0) }
        assert_raises(IndexError) { mem.read_word(14) } # 14+4 > 16
      end

      def test_dump
        mem = Memory.new(size: 16)
        mem.write_byte(0, 0xAB)
        assert_equal [0xAB, 0, 0, 0], mem.dump(0, 4)
      end

      def test_invalid_size
        assert_raises(ArgumentError) { Memory.new(size: 0) }
      end

      def test_negative_address
        mem = Memory.new(size: 16)
        assert_raises(IndexError) { mem.read_byte(-1) }
      end
    end

    # -----------------------------------------------------------------------
    # CPU tests
    # -----------------------------------------------------------------------
    class TestCPU < Minitest::Test
      def setup
        @decoder = TestDecoder.new
        @executor = TestExecutor.new
      end

      def make_program(*instructions)
        instructions.map { |i| [i].pack("V") }.join.b
      end

      def test_step_single_instruction
        cpu = CPU.new(decoder: @decoder, executor: @executor, num_registers: 16)
        # LOAD_IMM R0, 42
        program = make_program(0x01002A00, 0xFFFFFFFF)
        cpu.load_program(program)

        trace = cpu.step
        assert_equal 0, trace.cycle
        assert_equal "load_imm", trace.decode.mnemonic
        assert_equal 42, cpu.registers.read(0)
        assert_equal 4, cpu.pc
      end

      def test_run_complete_program
        # x = 1 + 2: LOAD R0,1; LOAD R1,2; ADD R2,R0,R1; HALT
        cpu = CPU.new(decoder: @decoder, executor: @executor, num_registers: 16)
        program = make_program(
          0x01000100, # LOAD_IMM R0, 1
          0x01000201, # LOAD_IMM R1, 2
          0x02000102, # ADD R2, R0, R1
          0xFFFFFFFF  # HALT
        )
        cpu.load_program(program)
        traces = cpu.run

        assert_equal 4, traces.size
        assert_equal 3, cpu.registers.read(2)
        assert cpu.halted
      end

      def test_halted_cpu_raises_on_step
        cpu = CPU.new(decoder: @decoder, executor: @executor)
        cpu.load_program(make_program(0xFFFFFFFF))
        cpu.step # executes HALT
        assert_raises(RuntimeError) { cpu.step }
      end

      def test_state_snapshot
        cpu = CPU.new(decoder: @decoder, executor: @executor, num_registers: 4)
        s = cpu.state
        assert_equal 0, s[:pc]
        assert_equal false, s[:halted]
        assert_equal 0, s[:cycle]
        assert_equal({"R0" => 0, "R1" => 0, "R2" => 0, "R3" => 0}, s[:registers])
      end

      def test_pipeline_trace_format
        cpu = CPU.new(decoder: @decoder, executor: @executor, num_registers: 16)
        program = make_program(0x01002A00, 0xFFFFFFFF)
        cpu.load_program(program)
        trace = cpu.step
        formatted = trace.format_pipeline
        assert_includes formatted, "Cycle 0"
        assert_includes formatted, "FETCH"
        assert_includes formatted, "DECODE"
        assert_includes formatted, "EXECUTE"
      end

      def test_max_steps_limit
        # A program that never halts — just loads values forever
        cpu = CPU.new(decoder: @decoder, executor: @executor, num_registers: 16, memory_size: 65536)
        # Fill memory with LOAD_IMM instructions
        instructions = Array.new(100) { 0x01000100 }
        program = make_program(*instructions)
        cpu.load_program(program)
        traces = cpu.run(max_steps: 5)
        assert_equal 5, traces.size
      end
    end

    # -----------------------------------------------------------------------
    # Data.define immutability tests
    # -----------------------------------------------------------------------
    class TestDataRecords < Minitest::Test
      def test_fetch_result_immutable
        fr = FetchResult.new(pc: 0, raw_instruction: 42)
        assert_equal 0, fr.pc
        assert_equal 42, fr.raw_instruction
        assert_raises(NoMethodError) { fr.pc = 1 }
      end

      def test_decode_result_immutable
        dr = DecodeResult.new(mnemonic: "add", fields: {rd: 1}, raw_instruction: 0)
        assert_equal "add", dr.mnemonic
        assert_raises(NoMethodError) { dr.mnemonic = "sub" }
      end

      def test_execute_result_default_halted
        er = ExecuteResult.new(
          description: "test",
          registers_changed: {},
          memory_changed: {},
          next_pc: 4
        )
        assert_equal false, er.halted
      end

      def test_pipeline_trace_default_register_snapshot
        fr = FetchResult.new(pc: 0, raw_instruction: 0)
        dr = DecodeResult.new(mnemonic: "nop", fields: {}, raw_instruction: 0)
        er = ExecuteResult.new(description: "nop", registers_changed: {},
          memory_changed: {}, next_pc: 4)
        pt = PipelineTrace.new(cycle: 0, fetch: fr, decode: dr, execute: er)
        assert_equal({}, pt.register_snapshot)
      end
    end
  end
end
