# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    class Scheduler
      attr_accessor :process_table, :current

      def initialize(process_table)
        @process_table = process_table
        @current = 0
      end

      def schedule
        n = @process_table.length
        return 0 if n == 0
        (1..n).each do |i|
          idx = (@current + i) % n
          return idx if @process_table[idx].state == PROCESS_READY
        end
        return @current if @current < n && @process_table[@current].state == PROCESS_READY
        0
      end

      def context_switch(from_pid, to_pid)
        if from_pid >= 0 && from_pid < @process_table.length
          @process_table[from_pid].state = PROCESS_READY if @process_table[from_pid].state == PROCESS_RUNNING
        end
        if to_pid >= 0 && to_pid < @process_table.length
          @process_table[to_pid].state = PROCESS_RUNNING
        end
        @current = to_pid
      end
    end
  end
end
