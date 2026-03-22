# CodingAdventures.ProcessManager

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

- **PCB (Process Control Block):** Immutable struct tracking a process — PID, registers, state, parent/children, signals, priority.
- **SignalManager:** Send, deliver, mask, unmask, and handle POSIX signals. All operations return new structs (immutable).
- **Manager:** Create processes, fork, exec, wait_for_child, kill, exit_process. Carries state through functional pipeline.
- **Scheduler:** 40-level priority scheduling with round-robin within each level.

## Usage

```elixir
alias CodingAdventures.ProcessManager.Manager

mgr = %Manager{}

# Create init and shell
{init, mgr} = Manager.create_process(mgr, "init")
{shell, mgr} = Manager.create_process(mgr, "shell", 0)

# Fork a child
{:ok, _parent_result, child_pid, mgr} = Manager.fork(mgr, shell.pid)

# Exec a new program
{:ok, mgr} = Manager.exec(mgr, child_pid, 0x10000, 0x7FFFF000)

# Child exits
{:ok, mgr} = Manager.exit_process(mgr, child_pid, 0)

# Parent reaps the zombie
{:ok, reaped_pid, exit_code, mgr} = Manager.wait_for_child(mgr, shell.pid)
```

## Testing

```bash
mix test
```

## License

MIT
