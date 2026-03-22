# Process Manager (D14) — Rust

A complete process management subsystem implementing Unix-style fork/exec/wait, POSIX signal delivery and handling, process control blocks with lifecycle management, and priority-based scheduling with round-robin within priority levels.

## Where It Fits

```
User Programs (shell, daemons, applications)
│
│  sys_fork(), sys_exec(), sys_wait4(), sys_kill()
▼
Process Manager (D14) ← THIS CRATE
│
▼
Virtual Memory (D13), Interrupt Handler (S03), OS Kernel (S04)
```

## Components

- **ProcessState** — Enum of lifecycle states (Ready, Running, Blocked, Terminated, Zombie)
- **Signal** — Enum of POSIX signals (SigInt, SigKill, SigTerm, SigChld, SigCont, SigStop)
- **ProcessControlBlock** — Per-process data: PID, registers, state, signals, priority
- **SignalManager** — Send, deliver, mask, unmask signals; register custom handlers
- **ProcessManager** — Core operations: create_process, fork, exec, wait, kill, exit_process
- **PriorityScheduler** — 40-level priority scheduling with round-robin within levels

## Usage

```rust
use process_manager::{ProcessManager, Signal};

let mut pm = ProcessManager::new();
let init = pm.create_process("init", None);
let child = pm.fork(init).unwrap();
pm.exec(child, 0x10000, 0x7FFFF);
pm.exit_process(child, 0);
let result = pm.wait(init);
// result == Some((child_pid, 0))
```

## Running Tests

```bash
cargo test -p process-manager
```

## Specification

See [D14-process-manager.md](../../../specs/D14-process-manager.md) for the full specification.
