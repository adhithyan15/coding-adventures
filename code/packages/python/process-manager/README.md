# Process Manager

Advanced process management for the coding-adventures operating system stack.

## Overview

The OS kernel (S04) has a minimal process table: two hardcoded processes and
round-robin scheduling. Real operating systems need dynamic process creation --
the ability for a running program to spawn children, replace itself with a new
program, wait for children to finish, and send signals to other processes.

This package implements the Unix process management model:

- **fork()** -- Clone a running process. The child is an exact copy of the
  parent, with a new PID.
- **exec()** -- Replace the current process's program with a new one. The PID
  stays the same, but the code, registers, and signal handlers are replaced.
- **wait()** -- Block until a child process exits, then retrieve its exit code.
- **kill()** -- Send a signal (SIGTERM, SIGKILL, etc.) to another process.
- **Priority scheduling** -- Replace round-robin with priority-based scheduling
  where lower priority numbers get more CPU time.

## Where It Fits

```
User Programs (shell, daemons, applications)
    |
    |  sys_fork(), sys_exec(), sys_wait4(), sys_kill()
    v
Process Manager (D14)  <-- THIS PACKAGE
    |
    v
OS Kernel (S04) -- manages PCBs, dispatches syscalls
```

## Modules

| Module | Description |
|--------|-------------|
| `pcb` | Extended ProcessControlBlock with parent/child relationships, signals, priority |
| `signals` | Signal enum, signal delivery, masking, and handler registration |
| `process_manager` | Core process lifecycle: fork, exec, wait, kill, exit |
| `priority_scheduler` | Priority-based scheduler with round-robin within priority levels |

## Usage

```python
from process_manager import ProcessManager, Signal, ProcessState

# Create a process manager
pm = ProcessManager()

# Create the init process (PID 0)
init = pm.create_process(name="init", priority=0)

# Fork a child
child_pid, child_return = pm.fork(init.pid)
# parent gets child_pid > 0
# child would get 0

# Send a signal
pm.kill(child_pid, Signal.SIGTERM)

# Exit a process
pm.exit_process(child_pid, exit_code=0)

# Wait for child
result = pm.wait(init.pid, child_pid)
# result = (child_pid, 0)
```

## Installation

```bash
pip install -e ".[dev]"
```

## Testing

```bash
pytest tests/ -v
```
