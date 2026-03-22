# Process Manager (Go)

Advanced process management for the coding-adventures operating system stack.

## Overview

The OS kernel (S04) has a minimal process table: two hardcoded processes and
round-robin scheduling. Real operating systems need dynamic process creation --
the ability for a running program to spawn children, replace itself with a new
program, wait for children to finish, and send signals to other processes.

This package implements the Unix process management model:

- **Fork()** -- Clone a running process. The child is an exact copy of the
  parent, with a new PID.
- **Exec()** -- Replace the current process's program with a new one. The PID
  stays the same, but the code, registers, and signal handlers are replaced.
- **Wait()** -- Block until a child process exits, then retrieve its exit code.
- **Kill()** -- Send a signal (SIGTERM, SIGKILL, etc.) to another process.
- **PriorityScheduler** -- Replace round-robin with priority-based scheduling
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

## Files

| File | Description |
|------|-------------|
| `pcb.go` | ProcessState enum and ProcessControlBlock struct |
| `signals.go` | Signal constants and SignalManager |
| `process_manager.go` | Core process lifecycle: Fork, Exec, Wait, Kill, Exit |
| `priority_scheduler.go` | Priority-based scheduler with round-robin within levels |

## Usage

```go
import pm "github.com/adhithyan15/coding-adventures/code/packages/go/process-manager"

manager := pm.NewProcessManager()

// Create init process
init := manager.CreateProcess("init", -1, 0, 0, 0)

// Fork a child
childPID, childRet := manager.Fork(init.PID)

// Send a signal
manager.Kill(childPID, pm.SIGTERM)

// Exit a process
manager.ExitProcess(childPID, 0)

// Wait for child
childPID, exitCode, ok := manager.Wait(init.PID, -1)
```

## Testing

```bash
go test ./... -v -cover
```
