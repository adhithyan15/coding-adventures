# frozen_string_literal: true

module CodingAdventures
  module VmCore
    BranchStats = Data.define(:taken_count, :not_taken_count) do
      def initialize(taken_count: 0, not_taken_count: 0)
        super
      end

      def record(taken)
        if taken
          BranchStats.new(taken_count: taken_count + 1, not_taken_count: not_taken_count)
        else
          BranchStats.new(taken_count: taken_count, not_taken_count: not_taken_count + 1)
        end
      end
    end

    VMMetrics = Data.define(
      :function_call_counts,
      :total_instructions_executed,
      :total_frames_pushed,
      :total_jit_hits,
      :branch_stats,
      :loop_back_edge_counts
    )
  end
end
