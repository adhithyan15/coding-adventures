# frozen_string_literal: true

# = Priority Scheduler
#
# The PriorityScheduler replaces the simple round-robin scheduler from
# the S04 OS Kernel with a priority-based scheduler. Instead of treating
# all processes equally, it gives more CPU time to higher-priority tasks.
#
# == Why Priority Scheduling?
#
# Round-robin is fair but blind. A keyboard handler that needs to respond
# in milliseconds gets the same time slice as a background file indexer.
# Priority scheduling fixes this by letting important tasks run first.
#
# == How It Works
#
# The scheduler maintains 40 separate queues, one per priority level:
#
#   Priority 0:  [kernel_task_1, kernel_task_2]    <-- runs first
#   Priority 1:  []
#   ...
#   Priority 20: [user_shell, user_editor]         <-- default for users
#   ...
#   Priority 39: [background_backup]               <-- runs last
#
# When the timer interrupt fires, the scheduler picks the front process
# from the highest-priority (lowest-numbered) non-empty queue.
#
# Within the same priority level, processes run in round-robin order:
# each gets a time slice, then goes to the back of the queue.
#
# == Time Quantum
#
# Higher-priority processes get larger time slices:
#   Priority 0:  200 cycles (kernel tasks need to finish quickly)
#   Priority 20: 100 cycles (normal user processes)
#   Priority 39:  50 cycles (background tasks get less CPU)
#
# The formula: quantum = 200 - (priority * 200 / 39)
# This linearly interpolates between 200 (highest) and ~50 (lowest).
#
# == Starvation
#
# A continuous stream of high-priority processes could prevent low-priority
# processes from ever running. Real schedulers address this with "aging" --
# gradually boosting the priority of starved processes. We note this as a
# future enhancement but keep the implementation simple.

module CodingAdventures
  module ProcessManager
    # PriorityScheduler implements priority-based process scheduling
    # with round-robin within each priority level.
    #
    # == Example
    #
    #   scheduler = PriorityScheduler.new
    #
    #   # Add processes at different priorities
    #   scheduler.add_process(high_priority_pcb)   # priority 5
    #   scheduler.add_process(normal_pcb)           # priority 20
    #   scheduler.add_process(background_pcb)       # priority 39
    #
    #   # Schedule always picks highest priority first
    #   scheduler.schedule  #=> high_priority_pcb
    #   scheduler.schedule  #=> normal_pcb  (only after priority 5 queue empties)
    class PriorityScheduler
      # Number of priority levels (0-39, inclusive).
      NUM_PRIORITIES = 40

      # Maximum time quantum (for highest priority, 0).
      MAX_QUANTUM = 200

      # Minimum time quantum (for lowest priority, 39).
      MIN_QUANTUM = 50

      attr_reader :ready_queues

      def initialize
        # Create 40 empty queues, one per priority level.
        # Each queue is an Array used as a FIFO (push to end, shift from front).
        @ready_queues = Array.new(NUM_PRIORITIES) { [] }
      end

      # Adds a process to the appropriate ready queue based on its priority.
      #
      # The process goes to the END of its priority queue, ensuring
      # round-robin ordering among processes of the same priority.
      #
      # @param pcb [ProcessControlBlock] the process to enqueue
      # @return [void]
      def add_process(pcb)
        priority = pcb.priority.clamp(0, NUM_PRIORITIES - 1)
        @ready_queues[priority] << pcb
      end

      # Removes a process from all ready queues.
      #
      # This is called when a process blocks, exits, or is killed.
      # We search all queues because the process might have had its
      # priority changed since it was enqueued.
      #
      # @param pid [Integer] the PID of the process to remove
      # @return [ProcessControlBlock, nil] the removed PCB, or nil
      def remove_process(pid)
        @ready_queues.each do |queue|
          idx = queue.index { |pcb| pcb.pid == pid }
          return queue.delete_at(idx) if idx
        end
        nil
      end

      # Selects the next process to run.
      #
      # == Algorithm
      #
      # Iterate through priority levels from 0 (highest) to 39 (lowest).
      # Return the first process from the first non-empty queue.
      #
      #   for priority in 0..39:
      #     if ready_queues[priority] is not empty:
      #       return ready_queues[priority].shift  # dequeue from front
      #   return nil  # nothing to run
      #
      # The dequeued process is NOT automatically re-enqueued. The caller
      # (kernel) is responsible for calling add_process() when the process's
      # time slice expires (preemption).
      #
      # @return [ProcessControlBlock, nil] the next process to run, or nil
      def schedule
        @ready_queues.each do |queue|
          return queue.shift unless queue.empty?
        end
        nil
      end

      # Changes a process's priority.
      #
      # If the process is currently in a ready queue, it is moved to the
      # new priority's queue. This is how the Unix `nice` and `renice`
      # commands work.
      #
      # @param pid [Integer] PID of the process
      # @param new_priority [Integer] the new priority (0-39)
      # @return [Boolean] true if the priority was changed
      def set_priority(pid, new_priority)
        new_priority = new_priority.clamp(0, NUM_PRIORITIES - 1)

        # Find and remove the process from its current queue.
        pcb = remove_process(pid)
        return false if pcb.nil?

        # Update priority and re-enqueue.
        pcb.priority = new_priority
        add_process(pcb)
        true
      end

      # Calculates the time quantum for a given priority level.
      #
      # Higher priority (lower number) = larger quantum = more CPU time.
      # The formula linearly interpolates between MAX_QUANTUM and MIN_QUANTUM:
      #
      #   quantum = MAX_QUANTUM - (priority * (MAX_QUANTUM - MIN_QUANTUM) / (NUM_PRIORITIES - 1))
      #
      # Examples:
      #   Priority 0:  200 cycles
      #   Priority 20: ~123 cycles
      #   Priority 39:  50 cycles
      #
      # @param priority [Integer] the priority level (0-39)
      # @return [Integer] the time quantum in CPU cycles
      def self.time_quantum_for(priority)
        priority = priority.clamp(0, NUM_PRIORITIES - 1)
        range = MAX_QUANTUM - MIN_QUANTUM
        MAX_QUANTUM - (priority * range / (NUM_PRIORITIES - 1))
      end

      # Returns the total number of processes across all ready queues.
      def total_ready
        @ready_queues.sum(&:size)
      end

      # Returns true if all ready queues are empty.
      def empty?
        @ready_queues.all?(&:empty?)
      end
    end
  end
end
