# frozen_string_literal: true

# = Process States and the Process Control Block (PCB)
#
# Every process in a Unix-like operating system goes through a lifecycle
# of states. At any moment, the kernel tracks each process's state using
# a data structure called the Process Control Block (PCB).
#
# == Process States
#
# A process can be in one of five states:
#
#   READY (0)       -- The process is loaded in memory and waiting for CPU time.
#                      The scheduler can pick it up at any time.
#
#   RUNNING (1)     -- The process is currently executing on the CPU.
#                      Only one process can be RUNNING per CPU core.
#
#   BLOCKED (2)     -- The process is waiting for something external: I/O,
#                      a signal, a child to exit, etc. It cannot run until
#                      the event it's waiting for occurs.
#
#   TERMINATED (3)  -- The process has finished execution normally (called
#                      exit). Its resources have been cleaned up.
#
#   ZOMBIE (4)      -- The process has exited, but its parent hasn't called
#                      wait() yet. The kernel keeps the PCB around so the
#                      parent can retrieve the exit status. Once the parent
#                      calls wait(), the zombie is "reaped" and removed.
#
# == State Transition Diagram
#
#                      fork()
#                        |
#                        v
#                 +------+------+
#        +------->|    READY    |<-----------+
#        |        +------+------+            |
#        |               | schedule()        |
#        |               v                   |
#        |        +------+------+    I/O     |
#        |        |   RUNNING   |----------->+------+------+
#        |        +------+------+            |   BLOCKED   |
#        |               |                   +------+------+
#   SIGCONT              | exit() or
#        |               | fatal signal
#        |               v
#   +----+-----+  +------+------+  wait()  +------------+
#   |  STOPPED |  |    ZOMBIE   |--------->| TERMINATED |
#   +----------+  +-------------+          +------------+
#
# == The Process Control Block
#
# The PCB is the kernel's "file" on each process. It stores everything
# the kernel needs to manage the process: its identity (PID), its CPU
# state (registers, program counter), its relationships (parent, children),
# and its signal handling configuration.
#
# Think of it like a patient's medical chart in a hospital. Every time
# a doctor (the CPU) switches to a new patient (process), they consult
# the chart to know where they left off and what to do next.

module CodingAdventures
  module ProcessManager
    # ProcessState enumerates the possible states a process can be in.
    #
    # Each state is represented by an integer constant matching the
    # POSIX convention used in real operating systems.
    module ProcessState
      READY      = 0
      RUNNING    = 1
      BLOCKED    = 2
      TERMINATED = 3
      ZOMBIE     = 4

      # All valid state values, for validation purposes.
      ALL = [READY, RUNNING, BLOCKED, TERMINATED, ZOMBIE].freeze

      # Human-readable names for debugging and logging.
      NAMES = {
        READY      => "READY",
        RUNNING    => "RUNNING",
        BLOCKED    => "BLOCKED",
        TERMINATED => "TERMINATED",
        ZOMBIE     => "ZOMBIE"
      }.freeze

      # Returns true if the given value is a valid process state.
      #
      # @param value [Integer] the state value to check
      # @return [Boolean]
      def self.valid?(value)
        ALL.include?(value)
      end

      # Returns the human-readable name for a state value.
      #
      # @param value [Integer] the state value
      # @return [String] the state name, or "UNKNOWN" for invalid values
      def self.name_for(value)
        NAMES.fetch(value, "UNKNOWN")
      end
    end

    # ProcessControlBlock holds all the information the kernel needs
    # to manage a single process.
    #
    # == Fields
    #
    # Identity:
    #   pid          -- Unique process identifier. Assigned sequentially.
    #   name         -- Human-readable name (e.g., "bash", "ls").
    #
    # CPU State (saved during context switch):
    #   registers    -- Array of 32 integers representing RISC-V x0-x31.
    #   pc           -- Program counter: address of the next instruction.
    #   sp           -- Stack pointer: top of the process's stack.
    #
    # Memory:
    #   memory_base  -- Start address of the process's memory region.
    #   memory_size  -- Size of the memory region in bytes.
    #
    # Process Lifecycle:
    #   state        -- Current ProcessState (READY, RUNNING, etc.).
    #   exit_code    -- Exit status code. Only meaningful in ZOMBIE state.
    #                   0 = success, nonzero = error.
    #
    # Relationships:
    #   parent_pid   -- PID of the process that created this one via fork().
    #                   PID 0 (idle) and PID 1 (init) have parent_pid = 0.
    #   children     -- Array of PIDs of all child processes.
    #
    # Signals:
    #   pending_signals  -- Array of signal values waiting to be delivered.
    #   signal_handlers  -- Hash mapping signal number => handler address.
    #                       If a signal has no entry, the default action applies.
    #   signal_mask      -- Set of signal numbers that are currently blocked.
    #                       Masked signals accumulate in pending_signals but
    #                       are not delivered until unmasked.
    #
    # Scheduling:
    #   priority     -- Scheduling priority, 0-39. Lower = higher priority.
    #                   0 = kernel/real-time tasks, 20 = default user process,
    #                   39 = lowest priority (background tasks).
    #   cpu_time     -- Total CPU cycles consumed. Used for profiling and
    #                   fair scheduling decisions.
    class ProcessControlBlock
      attr_accessor :pid, :name, :state, :registers, :pc, :sp,
        :memory_base, :memory_size, :parent_pid, :children,
        :pending_signals, :signal_handlers, :signal_mask,
        :priority, :cpu_time, :exit_code

      # The number of general-purpose registers in RISC-V (x0-x31).
      NUM_REGISTERS = 32

      # Default scheduling priority for user processes.
      # This follows the Unix "nice" convention where 20 is the baseline.
      DEFAULT_PRIORITY = 20

      # Creates a new Process Control Block with sensible defaults.
      #
      # @param pid [Integer] unique process identifier
      # @param name [String] human-readable process name
      # @param priority [Integer] scheduling priority (0-39, default 20)
      #
      # @example Create a new user process
      #   pcb = ProcessControlBlock.new(pid: 1, name: "init")
      #   pcb.state      #=> ProcessState::READY
      #   pcb.priority   #=> 20
      #   pcb.registers  #=> [0, 0, 0, ..., 0]  (32 zeros)
      def initialize(pid:, name:, priority: DEFAULT_PRIORITY)
        # Identity
        @pid  = pid
        @name = name

        # CPU state -- all registers start at zero, like a freshly powered CPU.
        @registers = Array.new(NUM_REGISTERS, 0)
        @pc = 0
        @sp = 0

        # Memory bounds
        @memory_base = 0
        @memory_size = 0

        # Lifecycle
        @state     = ProcessState::READY
        @exit_code = 0

        # Relationships -- no parent, no children initially.
        @parent_pid = 0
        @children   = []

        # Signals -- clean slate.
        @pending_signals = []
        @signal_handlers = {}
        @signal_mask     = Set.new

        # Scheduling
        @priority = priority.clamp(0, 39)
        @cpu_time = 0
      end

      # Returns true if this process is in the READY state.
      def ready?
        @state == ProcessState::READY
      end

      # Returns true if this process is currently executing on the CPU.
      def running?
        @state == ProcessState::RUNNING
      end

      # Returns true if this process is waiting for an external event.
      def blocked?
        @state == ProcessState::BLOCKED
      end

      # Returns true if this process has exited but not been reaped.
      def zombie?
        @state == ProcessState::ZOMBIE
      end

      # Returns true if this process has been fully cleaned up.
      def terminated?
        @state == ProcessState::TERMINATED
      end

      # Creates a deep copy of this PCB for use in fork().
      #
      # Fork copies most fields but resets some to fresh values:
      #   - New PID (must be set by caller)
      #   - parent_pid set to original PID
      #   - Empty children list (child starts with no children of its own)
      #   - Empty pending signals (child doesn't inherit pending signals)
      #   - cpu_time reset to 0 (fresh accounting)
      #   - Signal handlers ARE inherited (child keeps parent's handlers)
      #   - Priority IS inherited (child keeps parent's priority)
      #
      # @param new_pid [Integer] the PID for the forked child process
      # @return [ProcessControlBlock] a new PCB for the child process
      def fork_copy(new_pid)
        child = ProcessControlBlock.new(pid: new_pid, name: @name, priority: @priority)
        child.registers    = @registers.dup
        child.pc           = @pc
        child.sp           = @sp
        child.memory_base  = @memory_base
        child.memory_size  = @memory_size
        child.state        = ProcessState::READY
        child.parent_pid   = @pid
        child.children     = []
        child.pending_signals = []
        child.signal_handlers = @signal_handlers.dup
        child.signal_mask     = @signal_mask.dup
        child.cpu_time     = 0
        child.exit_code    = 0
        child
      end

      # Returns a human-readable string representation for debugging.
      def to_s
        "PCB[pid=#{@pid}, name=#{@name}, state=#{ProcessState.name_for(@state)}, " \
          "priority=#{@priority}, parent=#{@parent_pid}]"
      end
    end
  end
end
