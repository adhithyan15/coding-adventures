# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Tests for the GPUCore -- the main processing element simulator.
    # =========================================================================
    #
    # The GPUCore ties everything together: registers, memory, ISA, and the
    # fetch-execute loop. These tests verify the core lifecycle: construction,
    # program loading, stepping, running, and resetting.

    class TestCoreConstruction < Minitest::Test
      # Default core has GenericISA, FP32, 32 regs, 4KB memory.
      def test_default_construction
        core = GPUCore.new
        assert_equal "Generic", core.isa.name
        assert_equal FpArithmetic::FP32, core.fmt
        assert_equal 32, core.registers.num_registers
        assert_equal 4096, core.memory.size
        assert_equal 0, core.pc
        refute core.halted?
      end

      # Can provide a custom ISA.
      def test_custom_isa
        isa = GenericISA.new
        core = GPUCore.new(isa: isa)
        assert_same isa, core.isa
      end

      # Can configure register count for different vendors.
      def test_custom_registers
        core = GPUCore.new(num_registers: 255) # NVIDIA
        assert_equal 255, core.registers.num_registers
      end

      # Can configure floating-point format.
      def test_custom_format
        core = GPUCore.new(fmt: FpArithmetic::FP16)
        assert_equal FpArithmetic::FP16, core.fmt
      end

      # Can configure memory size.
      def test_custom_memory
        core = GPUCore.new(memory_size: 1024)
        assert_equal 1024, core.memory.size
      end

      # GPUCore responds to ProcessingElement duck type.
      def test_implements_processing_element
        core = GPUCore.new
        assert_respond_to core, :step
        assert_respond_to core, :halted?
        assert_respond_to core, :reset
      end

      # to_s shows ISA, register count, format, and status.
      def test_to_s
        core = GPUCore.new
        r = core.to_s
        assert_includes r, "Generic"
        assert_includes r, "running"
      end
    end

    class TestLoadProgram < Minitest::Test
      # Loading a program resets PC and cycle count.
      def test_load_program
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        assert_equal 0, core.pc
        assert_equal 0, core.cycle
        refute core.halted?
      end

      # Loading a new program replaces the old one.
      def test_load_replaces_program
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        core.run
        assert core.halted?
        core.load_program([GpuCore.limm(0, 2.0), GpuCore.halt])
        refute core.halted?
        assert_equal 0, core.pc
      end
    end

    class TestStep < Minitest::Test
      # Step through a LIMM instruction.
      def test_step_limm
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        trace = core.step
        assert_equal 0, trace.pc
        assert_equal 1, trace.cycle
        assert_equal 42.0, core.registers.read_float(0)
        assert_equal 1, core.pc
      end

      # Step through an FADD instruction.
      def test_step_fadd
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 1.0),
          GpuCore.limm(1, 2.0),
          GpuCore.fadd(2, 0, 1),
          GpuCore.halt
        ])
        core.step # limm R0, 1.0
        core.step # limm R1, 2.0
        trace = core.step # fadd R2, R0, R1
        assert_equal 3.0, core.registers.read_float(2)
        assert_includes trace.description, "3.0"
      end

      # Stepping into HALT sets halted flag.
      def test_step_halt
        core = GPUCore.new
        core.load_program([GpuCore.halt])
        trace = core.step
        assert trace.halted
        assert core.halted?
      end

      # Stepping a halted core raises RuntimeError.
      def test_step_when_halted_raises
        core = GPUCore.new
        core.load_program([GpuCore.halt])
        core.step
        err = assert_raises(RuntimeError) { core.step }
        assert_match(/halted/, err.message)
      end

      # Stepping past program end raises RuntimeError.
      def test_step_out_of_bounds_raises
        core = GPUCore.new
        core.load_program([GpuCore.nop])
        core.step # PC now 1, program has only 1 instruction
        err = assert_raises(RuntimeError) { core.step }
        assert_match(/PC=1 out of program range/, err.message)
      end

      # Each step increments the cycle counter.
      def test_step_increments_cycle
        core = GPUCore.new
        core.load_program([GpuCore.nop, GpuCore.nop, GpuCore.halt])
        core.step
        assert_equal 1, core.cycle
        core.step
        assert_equal 2, core.cycle
      end
    end

    class TestRun < Minitest::Test
      # Run a simple 3-instruction program.
      def test_simple_program
        core = GPUCore.new
        core.load_program([
          GpuCore.limm(0, 3.0),
          GpuCore.limm(1, 4.0),
          GpuCore.fmul(2, 0, 1),
          GpuCore.halt
        ])
        traces = core.run
        assert_equal 4, traces.length
        assert_equal 12.0, core.registers.read_float(2)
        assert core.halted?
      end

      # Infinite loop hits max_steps limit.
      def test_max_steps_limit
        core = GPUCore.new
        core.load_program([GpuCore.jmp(0)]) # infinite loop
        err = assert_raises(RuntimeError) { core.run(max_steps: 100) }
        assert_match(/Execution limit/, err.message)
      end

      # Running an empty program raises immediately.
      def test_empty_program_raises
        core = GPUCore.new
        core.load_program([])
        err = assert_raises(RuntimeError) { core.run }
        assert_match(/out of program range/, err.message)
      end
    end

    class TestReset < Minitest::Test
      # Reset clears all register values.
      def test_reset_clears_registers
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        core.run
        core.reset
        assert_equal 0.0, core.registers.read_float(0)
      end

      # Reset sets PC back to 0.
      def test_reset_clears_pc
        core = GPUCore.new
        core.load_program([GpuCore.nop, GpuCore.halt])
        core.run
        assert(core.pc != 0 || core.halted?)
        core.reset
        assert_equal 0, core.pc
      end

      # Reset clears the halted flag.
      def test_reset_clears_halted
        core = GPUCore.new
        core.load_program([GpuCore.halt])
        core.run
        assert core.halted?
        core.reset
        refute core.halted?
      end

      # Reset doesn't clear the loaded program -- can run again.
      def test_reset_preserves_program
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 99.0), GpuCore.halt])
        core.run
        core.reset
        core.run
        assert_equal 99.0, core.registers.read_float(0)
      end

      # Reset clears memory.
      def test_reset_clears_memory
        core = GPUCore.new
        core.memory.store_ruby_float(0, 42.0)
        core.reset
        assert_equal 0.0, core.memory.load_float_as_ruby(0)
      end

      # Reset sets cycle counter back to 0.
      def test_reset_clears_cycle
        core = GPUCore.new
        core.load_program([GpuCore.nop, GpuCore.halt])
        core.run
        assert core.cycle > 0
        core.reset
        assert_equal 0, core.cycle
      end
    end

    class TestTraces < Minitest::Test
      # Trace has all expected fields.
      def test_trace_fields
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        trace = core.step
        assert_equal 1, trace.cycle
        assert_equal 0, trace.pc
        assert_equal 1, trace.next_pc
        refute trace.halted
        refute_empty trace.description
      end

      # Trace.format returns readable multi-line string.
      def test_trace_format
        core = GPUCore.new
        core.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        trace = core.step
        formatted = trace.format
        assert_includes formatted, "[Cycle 1]"
        assert_includes formatted, "PC=0"
      end

      # Halt trace shows HALTED.
      def test_halt_trace
        core = GPUCore.new
        core.load_program([GpuCore.halt])
        trace = core.step
        assert_includes trace.format, "HALTED"
      end

      # Trace records which registers changed.
      def test_trace_registers_changed
        core = GPUCore.new
        core.load_program([GpuCore.limm(5, 3.14), GpuCore.halt])
        trace = core.step
        assert_includes trace.registers_changed, "R5"
      end
    end
  end
end
