# process_manager (Lua)

Process lifecycle management for the coding-adventures simulated OS.

## What It Does

Implements the Unix process model:

- **PCB** (Process Control Block) — kernel's per-process data structure
- **fork** — clone a process into a parent/child pair
- **exec** — replace a process's program image
- **wait** — reap a zombie child and collect its exit code
- **kill** — send signals (SIGINT, SIGKILL, SIGTERM, SIGSTOP, SIGCONT, SIGCHLD)
- **schedule** — priority-based round-robin scheduler

## Usage

```lua
local PM = require("coding_adventures.process_manager")

local mgr = PM.Manager.new()

-- Spawn init
local mgr2, init_pid = mgr:spawn("init")

-- Fork to run a command
local mgr3, child_pid = mgr2:fork(init_pid)

-- Exec in the child
local mgr4 = mgr3:exec(child_pid, "ls", { pc = 0x4000 })

-- Run the scheduler
local mgr5, running_pid = mgr4:schedule()

-- Child exits
local mgr6 = mgr5:exit_process(child_pid, 0)

-- Parent waits
local status, mgr7, exit_code = mgr6:wait_child(init_pid, child_pid)
-- exit_code == 0
```

## Process State Transitions

```
fork() → ready ──[schedule()]──► running ──[exit()]──► zombie ──[wait()]──► removed
           ▲                        │
           │                        ▼
           └────── blocked ◄──[block()]
           unblock()
```
