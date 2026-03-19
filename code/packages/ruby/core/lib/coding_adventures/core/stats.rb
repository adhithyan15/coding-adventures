# frozen_string_literal: true

# CoreStats -- aggregate statistics from all core sub-components.
#
# = Why Aggregate Statistics?
#
# Each sub-component tracks its own statistics independently:
#   - Pipeline: stall cycles, flush cycles, completed instructions
#   - Branch Predictor: accuracy, misprediction count
#   - Hazard Unit: forwarding count, stall count
#   - Cache: hit rate, miss rate, evictions
#
# CoreStats pulls all of these together into a single view, like the
# dashboard of a car that shows speed (from the speedometer), fuel level
# (from the tank sensor), and engine temperature (from the thermostat).
#
# = Key Metrics
#
# IPC (Instructions Per Cycle): the most important performance metric.
#
#   IPC = instructions_completed / total_cycles
#
#   IPC = 1.0: every cycle produces a result (ideal for scalar pipeline)
#   IPC < 1.0: stalls and flushes are wasting cycles
#   IPC > 1.0: superscalar (not modeled yet)
#
# CPI (Cycles Per Instruction): the inverse of IPC.
#
#   CPI = total_cycles / instructions_completed

module CodingAdventures
  module Core
    class CoreStats
      # @return [Integer] number of instructions that reached WB.
      attr_accessor :instructions_completed

      # @return [Integer] total number of clock cycles elapsed.
      attr_accessor :total_cycles

      # @return [CodingAdventures::CpuPipeline::PipelineStats] pipeline stats.
      attr_accessor :pipeline_stats

      # @return [CodingAdventures::BranchPredictor::PredictionStats, nil] predictor stats.
      attr_accessor :predictor_stats

      # @return [Hash<String, CodingAdventures::Cache::CacheStats>] cache stats by level name.
      attr_accessor :cache_stats

      # @return [Integer] total number of forwarding operations.
      attr_accessor :forward_count

      # @return [Integer] total number of stall cycles.
      attr_accessor :stall_count

      # @return [Integer] total number of pipeline flush cycles.
      attr_accessor :flush_count

      def initialize
        @instructions_completed = 0
        @total_cycles = 0
        @pipeline_stats = nil
        @predictor_stats = nil
        @cache_stats = {}
        @forward_count = 0
        @stall_count = 0
        @flush_count = 0
      end

      # Returns instructions per cycle.
      #
      # This is the primary measure of pipeline efficiency:
      #   - 1.0 = perfect (every cycle retires an instruction)
      #   - <1.0 = stalls/flushes wasting cycles
      #   - 0.0 = no instructions completed or no cycles elapsed
      #
      # @return [Float] IPC.
      def ipc
        return 0.0 if @total_cycles == 0
        @instructions_completed.to_f / @total_cycles
      end

      # Returns cycles per instruction.
      #
      # This is the inverse of IPC:
      #   - 1.0 = one cycle per instruction (ideal)
      #   - >1.0 = some cycles wasted
      #   - 0.0 = no instructions completed
      #
      # @return [Float] CPI.
      def cpi
        return 0.0 if @instructions_completed == 0
        @total_cycles.to_f / @instructions_completed
      end

      # Returns a formatted summary of all statistics.
      #
      # @return [String] formatted stats report.
      def to_s
        result = "Core Statistics:\n"
        result += "  Instructions completed: #{@instructions_completed}\n"
        result += "  Total cycles:           #{@total_cycles}\n"
        result += format("  IPC: %.3f   CPI: %.3f\n", ipc, cpi)
        result += "\n"

        if @pipeline_stats
          result += "Pipeline:\n"
          result += "  Stall cycles:  #{@pipeline_stats.stall_cycles}\n"
          result += "  Flush cycles:  #{@pipeline_stats.flush_cycles}\n"
          result += "  Bubble cycles: #{@pipeline_stats.bubble_cycles}\n"
          result += "\n"
        end

        if @predictor_stats
          result += "Branch Prediction:\n"
          result += "  Total branches:  #{@predictor_stats.predictions}\n"
          result += "  Correct:         #{@predictor_stats.correct}\n"
          result += "  Mispredictions:  #{@predictor_stats.incorrect}\n"
          result += format("  Accuracy:        %.1f%%\n", @predictor_stats.accuracy)
          result += "\n"
        end

        unless @cache_stats.empty?
          result += "Cache Performance:\n"
          @cache_stats.each do |name, stats|
            result += format("  %s: accesses=%d, hit_rate=%.1f%%\n",
              name, stats.total_accesses, stats.hit_rate * 100)
          end
          result += "\n"
        end

        result += "Hazards:\n"
        result += "  Forwards: #{@forward_count}\n"
        result += "  Stalls:   #{@stall_count}\n"
        result += "  Flushes:  #{@flush_count}\n"

        result
      end
    end
  end
end
