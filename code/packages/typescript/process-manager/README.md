# @coding-adventures/process-manager

Process management subsystem for the computing stack (Layer D14). Implements the core Unix process lifecycle: **fork**, **exec**, **wait**, **signals**, and **priority scheduling**.

## Where It Fits

```
User Programs (shell, daemons)
|
|  sys_fork(), sys_exec(), sys_wait4(), sys_kill()
v
Process Manager (D14) <-- THIS PACKAGE
|
v
Virtual Memory (D13), OS Kernel (S04)
```

## Components

- **ProcessControlBlock (PCB):** The kernel's data structure for tracking a process — PID, registers, state, parent/children, signals, priority.
- **SignalManager:** Send, deliver, mask, unmask, and handle POSIX signals (SIGINT, SIGKILL, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP).
- **ProcessManager:** Create processes, fork (clone), exec (replace program), wait (reap zombies), kill (send signals), exit (terminate with reparenting).
- **PriorityScheduler:** 40-level priority scheduling with round-robin within each level. Higher priority (lower number) runs first.

## Usage

```typescript
import {
  ProcessManager,
  PriorityScheduler,
  ProcessState,
  Signal,
} from "@coding-adventures/process-manager";

// Create a process manager and init process
const pm = new ProcessManager();
const init = pm.create_process("init");

// Fork a child
const result = pm.fork(init.pid);
const child = pm.get_process(result.child_pid);

// Exec a new program in the child
pm.exec(child.pid, 0x10000, 0x7FFFF000);

// Child exits
pm.exit_process(child.pid, 0);

// Parent reaps the zombie
const wait_result = pm.wait(init.pid);
// wait_result = { child_pid: 1, exit_code: 0 }
```

## Testing

```bash
npx vitest run --coverage
```

## License

MIT
