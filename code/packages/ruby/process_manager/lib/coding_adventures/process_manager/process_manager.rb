# frozen_string_literal: true

# = ProcessManager: fork, exec, wait, kill, exit
#
# The ProcessManager is the heart of Unix process management. It implements
# the five fundamental operations that every Unix system provides:
#
#   fork()  -- Clone a running process. The parent continues running, and
#              the child gets an exact copy of the parent's state.
#
#   exec()  -- Replace the current process's program with a new one. The
#              PID stays the same, but the code, data, and stack change.
#
#   wait()  -- A parent waits for a child to exit. Returns the child's
#              exit status and reaps the zombie.
#
#   kill()  -- Send a signal to a process. Despite the name, it doesn't
#              necessarily kill the target -- it delivers a signal.
#
#   exit()  -- A process exits voluntarily. Sets state to ZOMBIE, notifies
#              the parent, and reparents any orphaned children.
#
# == The fork/exec Pattern
#
# Unix deliberately splits process creation into two steps. This is
# different from Windows (which uses a single CreateProcess call) and
# it's more flexible. Between fork() and exec(), the child can set up
# its environment:
#
#   pid = fork()
#   if pid == 0:        # child
#     close(stdout)     # redirect output
#     open("out.txt")
#     exec("ls")        # NOW run the program
#   else:               # parent
#     wait(pid)         # wait for child to finish
#
# This is exactly how shell I/O redirection works (`ls > out.txt`).
#
# == Zombie Processes
#
# When a process exits, the kernel cannot immediately delete its PCB.
# The parent might call wait() later to retrieve the exit status. So
# the kernel keeps the PCB in a "zombie" state -- the process is dead
# (no address space, no resources), but its PID and exit status live on.
#
# If the parent exits without waiting, orphaned zombies are reparented
# to PID 0 (the idle/init process), which periodically reaps them.

module CodingAdventures
  module ProcessManager
    # ProcessManager manages the process table and implements the core
    # Unix process lifecycle operations.
    #
    # It maintains a hash table mapping PIDs to ProcessControlBlocks,
    # and uses a SignalManager for signal delivery.
    #
    # == Example
    #
    #   pm = ProcessManager.new
    #   init_pid = pm.create_process("init")  #=> 1
    #
    #   # Fork a child
    #   child_pid = pm.fork(init_pid)  #=> 2
    #
    #   # Exec a new program in the child
    #   pm.exec(child_pid, entry_point: 0x1000, stack_pointer: 0x7FFF)
    #
    #   # Child exits
    #   pm.exit_process(child_pid, exit_code: 0)
    #
    #   # Parent reaps the child
    #   result = pm.wait(init_pid)  #=> {pid: 2, exit_code: 0}
    class ProcessManager
      attr_reader :process_table, :signal_manager

      def initialize
        # The process table maps PID => ProcessControlBlock.
        # This is the kernel's central data structure for process management.
        @process_table = {}

        # Next PID to assign. PIDs are monotonically increasing.
        # PID 0 is reserved for the idle process (created automatically).
        @next_pid = 0

        # Signal manager handles all signal-related operations.
        @signal_manager = SignalManager.new
      end

      # Creates a new process and adds it to the process table.
      #
      # This is the primitive process creation operation. fork() is built
      # on top of this, but create_process is useful for bootstrapping
      # the first processes (idle, init) that aren't forked from anything.
      #
      # @param name [String] human-readable name for the process
      # @param priority [Integer] scheduling priority (0-39, default 20)
      # @return [Integer] the PID of the newly created process
      #
      # @example Create the init process
      #   pid = pm.create_process("init")
      #   pm.process_table[pid].name  #=> "init"
      def create_process(name, priority: ProcessControlBlock::DEFAULT_PRIORITY)
        pid = @next_pid
        @next_pid += 1

        pcb = ProcessControlBlock.new(pid: pid, name: name, priority: priority)
        @process_table[pid] = pcb

        pid
      end

      # Forks a process, creating an exact copy with a new PID.
      #
      # == How fork() Works
      #
      # 1. Allocate a new PID for the child.
      # 2. Copy the parent's PCB (registers, PC, memory bounds, etc.).
      # 3. Reset child-specific fields (new PID, empty children, cpu_time=0).
      # 4. Set return values: parent gets child_pid, child gets 0 (in register A0).
      # 5. Add child to parent's children list.
      # 6. Add child to process table.
      #
      # == What Gets Copied vs. What Doesn't
      #
      #   Copied:    registers, PC, SP, memory bounds, signal handlers, priority
      #   NOT copied: PID (new), children (empty), pending signals (empty), cpu_time (0)
      #
      # In a real OS, fork() also clones the address space using copy-on-write
      # (COW). We don't model virtual memory here -- that's D13's job.
      #
      # @param parent_pid [Integer] PID of the process to fork
      # @return [Integer] PID of the new child process, or -1 on error
      #
      # @example
      #   child_pid = pm.fork(parent_pid)
      #   parent_pcb = pm.process_table[parent_pid]
      #   child_pcb  = pm.process_table[child_pid]
      #   parent_pcb.registers[10]  #=> child_pid  (A0 register, RISC-V)
      #   child_pcb.registers[10]   #=> 0
      def fork(parent_pid)
        parent_pcb = @process_table[parent_pid]
        return -1 if parent_pcb.nil?

        # Step 1: Allocate a new PID.
        child_pid = @next_pid
        @next_pid += 1

        # Step 2-3: Create a copy of the parent's PCB with child-specific resets.
        child_pcb = parent_pcb.fork_copy(child_pid)

        # Step 4: Set the return values.
        # In RISC-V, register x10 (a0) holds the return value.
        # Parent sees child_pid, child sees 0.
        # This is the magic of fork(): both processes resume from the
        # same point, but they see DIFFERENT return values.
        parent_pcb.registers[10] = child_pid
        child_pcb.registers[10] = 0

        # Step 5: Add child to parent's children list.
        parent_pcb.children << child_pid

        # Step 6: Add child to the process table.
        @process_table[child_pid] = child_pcb

        child_pid
      end

      # Replaces a process's program with a new one (exec).
      #
      # == How exec() Works
      #
      # The process keeps its PID, parent, and children, but everything
      # else is replaced: registers are zeroed, PC is set to the new
      # entry point, signal handlers are cleared.
      #
      # In a real OS, exec() would destroy the old address space, load
      # the new binary, and set up fresh memory mappings. We model this
      # at the PCB level.
      #
      # == What Changes vs. What Stays
      #
      #   Changes:  registers (zeroed), PC (entry point), SP (new stack),
      #             signal handlers (cleared), pending signals (cleared)
      #   Stays:    PID, parent_pid, children, priority, cpu_time
      #
      # @param pid [Integer] PID of the process to exec in
      # @param entry_point [Integer] address of the first instruction
      # @param stack_pointer [Integer] initial stack pointer value
      # @return [Boolean] true on success, false if PID not found
      def exec(pid, entry_point:, stack_pointer:)
        pcb = @process_table[pid]
        return false if pcb.nil?

        # Zero all registers. The new program starts with a clean slate.
        pcb.registers = Array.new(ProcessControlBlock::NUM_REGISTERS, 0)

        # Set the program counter to the new program's entry point.
        pcb.pc = entry_point

        # Set the stack pointer. The stack grows downward from this address.
        pcb.sp = stack_pointer

        # Clear all signal handlers. The new program doesn't know about
        # the old program's handlers, so we reset to defaults.
        pcb.signal_handlers = {}

        # Clear pending signals. The new program shouldn't inherit signals
        # intended for the old program.
        pcb.pending_signals = []

        true
      end

      # Waits for a zombie child and reaps it.
      #
      # == How wait() Works
      #
      # The parent scans its children list for any child in ZOMBIE state.
      # If found, the zombie is reaped: its exit code is returned, it's
      # removed from the parent's children list, and its PCB is deleted
      # from the process table.
      #
      # If no zombie children exist, wait() returns nil (in a real OS,
      # the parent would block until a child exits).
      #
      # @param parent_pid [Integer] PID of the waiting parent
      # @return [Hash, nil] {pid:, exit_code:} of the reaped child, or nil
      #
      # @example
      #   pm.exit_process(child_pid, exit_code: 42)
      #   result = pm.wait(parent_pid)
      #   result[:pid]        #=> child_pid
      #   result[:exit_code]  #=> 42
      def wait(parent_pid)
        parent_pcb = @process_table[parent_pid]
        return nil if parent_pcb.nil?

        # Scan children for zombies.
        parent_pcb.children.each do |child_pid|
          child_pcb = @process_table[child_pid]
          next unless child_pcb&.zombie?

          # Found a zombie child -- reap it.
          exit_code = child_pcb.exit_code

          # Remove from parent's children list.
          parent_pcb.children.delete(child_pid)

          # Remove the PCB from the process table entirely.
          # The zombie has been reaped -- its PID can eventually be reused.
          @process_table.delete(child_pid)

          return {pid: child_pid, exit_code: exit_code}
        end

        # No zombie children found.
        nil
      end

      # Sends a signal to a process.
      #
      # This is the Unix kill() system call. Despite its alarming name,
      # it doesn't necessarily kill anything -- it delivers a signal.
      # Only SIGKILL guarantees termination.
      #
      # @param target_pid [Integer] PID of the process to signal
      # @param signal [Integer] the signal number to send
      # @return [Boolean] true if the signal was delivered, false on error
      def kill(target_pid, signal)
        target_pcb = @process_table[target_pid]
        return false if target_pcb.nil?

        @signal_manager.send_signal(target_pcb, signal)
      end

      # Terminates a process voluntarily.
      #
      # == How exit() Works
      #
      # 1. Set the process state to ZOMBIE.
      # 2. Record the exit code.
      # 3. Reparent all children to PID 0 (the idle/init process).
      # 4. Send SIGCHLD to the parent, notifying it that a child exited.
      #
      # The process becomes a zombie until its parent calls wait() to
      # retrieve the exit status. The zombie's PID and exit code are
      # preserved; everything else is released.
      #
      # @param pid [Integer] PID of the exiting process
      # @param exit_code [Integer] the exit status (0 = success)
      # @return [Boolean] true on success, false if PID not found
      def exit_process(pid, exit_code: 0)
        pcb = @process_table[pid]
        return false if pcb.nil?

        # Step 1: Set state to ZOMBIE.
        pcb.state = ProcessState::ZOMBIE
        pcb.exit_code = exit_code

        # Step 2: Reparent all children to PID 0.
        # In real Unix, orphans go to PID 1 (init). We use PID 0 for simplicity.
        # This prevents orphaned zombies from never being reaped.
        pcb.children.each do |child_pid|
          child_pcb = @process_table[child_pid]
          next unless child_pcb

          child_pcb.parent_pid = 0

          # Add to PID 0's children list if PID 0 exists.
          init_pcb = @process_table[0]
          init_pcb&.children&.push(child_pid)
        end
        pcb.children = []

        # Step 3: Send SIGCHLD to the parent.
        # This notifies the parent that a child has changed state.
        parent_pcb = @process_table[pcb.parent_pid]
        @signal_manager.send_signal(parent_pcb, Signal::SIGCHLD) if parent_pcb

        true
      end

      # Returns the number of processes currently in the process table.
      def process_count
        @process_table.size
      end

      # Returns true if a process with the given PID exists.
      def process_exists?(pid)
        @process_table.key?(pid)
      end

      # Returns the PCB for a given PID, or nil if not found.
      def get_process(pid)
        @process_table[pid]
      end
    end
  end
end
