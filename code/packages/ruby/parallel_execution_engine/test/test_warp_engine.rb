# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for WarpEngine -- SIMT parallel execution (NVIDIA/ARM Mali style).
    # =========================================================================

    def self.make_edge(cycle = 1)
      Clock::ClockEdge.new(cycle: cycle, value: 1, "rising?": true, "falling?": false)
    end

    # -----------------------------------------------------------------------
    # WarpConfig
    # -----------------------------------------------------------------------

    class TestWarpConfig < Minitest::Test
      def test_defaults
        config = WarpConfig.new
        assert_equal 32, config.warp_width
        assert_equal 32, config.num_registers
        assert_equal 1024, config.memory_per_thread
        assert_equal 32, config.max_divergence_depth
        assert_equal false, config.independent_thread_scheduling
      end

      def test_custom
        config = WarpConfig.new(warp_width: 16, num_registers: 64)
        assert_equal 16, config.warp_width
        assert_equal 64, config.num_registers
      end
    end

    # -----------------------------------------------------------------------
    # ThreadContext
    # -----------------------------------------------------------------------

    class TestThreadContext < Minitest::Test
      def test_defaults
        core = GpuCore::GPUCore.new
        ctx = ThreadContext.new(thread_id: 0, core: core)
        assert_equal 0, ctx.thread_id
        assert_equal true, ctx.active
        assert_equal 0, ctx.pc
      end
    end

    # -----------------------------------------------------------------------
    # DivergenceStackEntry
    # -----------------------------------------------------------------------

    class TestDivergenceStackEntry < Minitest::Test
      def test_creation
        entry = DivergenceStackEntry.new(
          reconvergence_pc: 10,
          saved_mask: [true, false, true, false]
        )
        assert_equal 10, entry.reconvergence_pc
        assert_equal [true, false, true, false], entry.saved_mask
      end
    end

    # -----------------------------------------------------------------------
    # WarpEngine -- basic properties
    # -----------------------------------------------------------------------

    class TestWarpEngineProperties < Minitest::Test
      def test_name
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        assert_equal "WarpEngine", engine.name
      end

      def test_width
        engine = WarpEngine.new(WarpConfig.new(warp_width: 16), Clock::ClockGenerator.new)
        assert_equal 16, engine.width
      end

      def test_execution_model
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        assert_equal :simt, engine.execution_model
      end

      def test_initial_halted
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        refute engine.halted?
      end

      def test_active_mask_all_true
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        assert_equal [true, true, true, true], engine.active_mask
      end

      def test_config_access
        config = WarpConfig.new(warp_width: 8)
        engine = WarpEngine.new(config, Clock::ClockGenerator.new)
        assert_same config, engine.config
      end

      def test_to_s
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        r = engine.to_s
        assert_includes r, "WarpEngine"
        assert_includes r, "width=4"
      end
    end

    # -----------------------------------------------------------------------
    # WarpEngine -- program execution
    # -----------------------------------------------------------------------

    class TestWarpEngineExecution < Minitest::Test
      # All threads execute LIMM + HALT.
      def test_simple_program
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])

        traces = engine.run
        assert traces.length >= 2

        engine.threads.each do |t|
          assert_equal 42.0, t.core.registers.read_float(0)
        end
      end

      # Each thread gets different input, computes independently.
      def test_per_thread_data
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(1, 2.0),
          GpuCore.fmul(2, 0, 1),
          GpuCore.halt
        ])

        4.times { |t| engine.set_thread_register(t, 0, (t + 1).to_f) }
        engine.run

        4.times do |t|
          result = engine.threads[t].core.registers.read_float(2)
          assert_equal (t + 1) * 2.0, result
        end
      end

      # Engine halts when all threads execute HALT.
      def test_halts_when_all_done
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.halt])
        engine.run
        assert engine.halted?
      end

      # Setting register for invalid thread raises IndexError.
      def test_thread_register_out_of_range
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        assert_raises(IndexError) { engine.set_thread_register(4, 0, 1.0) }
        assert_raises(IndexError) { engine.set_thread_register(-1, 0, 1.0) }
      end

      # Each step produces an EngineTrace with correct fields.
      def test_step_produces_traces
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 1, trace.cycle
        assert_equal "WarpEngine", trace.engine_name
        assert_equal :simt, trace.execution_model
        assert_equal 4, trace.total_count
        assert trace.active_count > 0
        assert_operator trace.utilization, :>=, 0.0
        assert_operator trace.utilization, :<=, 1.0
      end

      # Utilization should be active_count / total_count.
      def test_utilization_in_trace
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])

        trace = engine.step(ParallelExecutionEngine.make_edge)
        expected = trace.active_count.to_f / trace.total_count
        assert_in_delta expected, trace.utilization, 0.001
      end
    end

    # -----------------------------------------------------------------------
    # WarpEngine -- divergence
    # -----------------------------------------------------------------------

    class TestWarpEngineDivergence < Minitest::Test
      # When all threads agree on a branch, no divergence.
      def test_no_divergence_on_uniform_branch
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(0, 0.0),
          GpuCore.limm(1, 10.0),
          GpuCore.blt(0, 1, 2),
          GpuCore.nop,
          GpuCore.nop,
          GpuCore.halt
        ])
        engine.run
        assert engine.halted?
      end

      # When threads disagree on a branch, divergence occurs.
      def test_divergent_branch
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(1, 2.0),
          GpuCore.blt(0, 1, 2),
          GpuCore.limm(2, 99.0),
          GpuCore.halt,
          GpuCore.limm(2, 42.0),
          GpuCore.halt
        ])

        engine.set_thread_register(0, 0, 0.0)
        engine.set_thread_register(1, 0, 0.0)
        engine.set_thread_register(2, 0, 5.0)
        engine.set_thread_register(3, 0, 5.0)

        engine.run
        assert engine.halted?
      end
    end

    # -----------------------------------------------------------------------
    # WarpEngine -- reset
    # -----------------------------------------------------------------------

    class TestWarpEngineReset < Minitest::Test
      def test_reset_restores_initial_state
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        engine.run
        assert engine.halted?

        engine.reset
        refute engine.halted?
        assert engine.threads.all?(&:active)

        engine.run
        assert engine.halted?
      end

      def test_reset_clears_registers
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        engine.run

        engine.reset
        engine.threads.each do |t|
          assert_equal 0.0, t.core.registers.read_float(0)
        end
      end
    end

    # -----------------------------------------------------------------------
    # WarpEngine -- clock integration
    # -----------------------------------------------------------------------

    class TestWarpEngineClockIntegration < Minitest::Test
      def test_step_with_clock_edge
        clock = Clock::ClockGenerator.new
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), clock)
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])

        edge = clock.tick
        trace = engine.step(edge)
        assert_equal 1, trace.cycle
      end

      def test_halted_step_returns_trace
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.halt])
        engine.run
        assert engine.halted?

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 0, trace.active_count
        assert(trace.description.downcase.include?("halted") || trace.utilization == 0.0)
      end
    end
  end
end
