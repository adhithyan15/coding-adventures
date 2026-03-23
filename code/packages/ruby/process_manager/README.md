# Process Manager (D14) — Ruby

A complete process management subsystem implementing Unix-style fork/exec/wait, POSIX signal delivery and handling, process control blocks with lifecycle management, and priority-based scheduling with round-robin within priority levels.

## Where It Fits

```
User Programs (shell, daemons, applications)
│
│  sys_fork(), sys_exec(), sys_wait4(), sys_kill()
▼
Process Manager (D14) ← THIS PACKAGE
│
▼
Virtual Memory (D13), Interrupt Handler (S03), OS Kernel (S04)
```

## Components

- **ProcessState** — Enumeration of process lifecycle states (READY, RUNNING, BLOCKED, TERMINATED, ZOMBIE)
- **ProcessControlBlock** — The kernel's per-process data structure: PID, registers, state, signals, priority
- **Signal** — POSIX signal constants (SIGINT, SIGKILL, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP)
- **SignalManager** — Send, deliver, mask, unmask signals; register custom handlers
- **ProcessManager** — Core operations: create_process, fork, exec, wait, kill, exit_process
- **PriorityScheduler** — Priority-based scheduling with 40 levels and round-robin within each level

## Usage

```ruby
require "coding_adventures_process_manager"

pm = CodingAdventures::ProcessManager::ProcessManager.new

# Create the init process
init = pm.create_process("init")

# Fork a child
child = pm.fork(init)

# Exec a new program in the child
pm.exec(child, entry_point: 0x10000, stack_pointer: 0x7FFFF)

# Child exits
pm.exit_process(child, exit_code: 0)

# Parent reaps the child
result = pm.wait(init)
# result => {pid: 1, exit_code: 0}
```

## Running Tests

```bash
bundle install
bundle exec rake test
```

## Specification

See [D14-process-manager.md](../../../specs/D14-process-manager.md) for the full specification.
