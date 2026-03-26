# frozen_string_literal: true

# = Process Manager (D14)
#
# This is the top-level entry point for the Process Manager package.
# It requires all submodules in the correct order.
#
# The Process Manager implements Unix-style process management:
#   - Process Control Blocks (PCBs) with lifecycle states
#   - POSIX signals (SIGINT, SIGKILL, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP)
#   - fork/exec/wait/kill system call implementations
#   - Priority-based scheduling with round-robin within levels
#
# == Quick Start
#
#   require "coding_adventures_process_manager"
#
#   pm = CodingAdventures::ProcessManager::ProcessManager.new
#   init = pm.create_process("init")
#   child = pm.fork(init)
#   pm.exec(child, entry_point: 0x1000, stack_pointer: 0x7FFF)
#   pm.exit_process(child, exit_code: 0)
#   result = pm.wait(init)  #=> {pid: 1, exit_code: 0}

require "set"

require_relative "coding_adventures/process_manager/version"
require_relative "coding_adventures/process_manager/pcb"
require_relative "coding_adventures/process_manager/signals"
require_relative "coding_adventures/process_manager/process_manager"
require_relative "coding_adventures/process_manager/priority_scheduler"
