# D14 — Process Manager

## Overview

The S04 kernel has a minimal process table: two hardcoded processes (idle and
hello-world), round-robin scheduling, and no way to create new processes at
runtime. Real operating systems need dynamic process creation — the ability for
a running program to spawn children, replace itself with a new program, wait
for children to finish, and send signals to other processes.

Unix solved this with three elegant system calls: **fork**, **exec**, and
**wait**. Together, they form the foundation of every Unix shell, every daemon,
and every server process. Understanding how they work is understanding how Unix
works.

**Analogy:** Think of a restaurant kitchen. The head chef (parent process) can:
- **fork():** Clone themselves — now there are two identical chefs with the
  same knowledge and state. The clone (child) can do different work.
- **exec():** The clone throws away their current recipe book and picks up a
  completely different one. They are still the same person (same PID), but now
  they are cooking something entirely different.
- **wait():** The head chef pauses and watches the clone work. When the clone
  finishes and leaves, the head chef resumes.

This is exactly how your shell works. When you type `ls`:

```
Shell (PID 100)
│
├── fork() → creates child (PID 101)
│   │         Child is an exact copy of the shell!
│   │
│   ├── [Child PID 101]: exec("ls")
│   │     Replaces shell code with ls code.
│   │     ls runs, prints files, exits.
│   │
│   └── [Parent PID 100]: wait(101)
│         Pauses until child exits.
│         Resumes when ls is done.
│
├── Shell prompt appears again.
```

## Where It Fits

```
User Programs (shell, daemons, applications)
│
│  sys_fork(), sys_exec(), sys_wait4(), sys_kill()
▼
Process Manager (D14) ← YOU ARE HERE
│
│  ┌──────────────────────────────────────────────────┐
│  │  ProcessManager                                   │
│  │  ├── fork()   — clone a process (COW)             │
│  │  ├── exec()   — replace with new binary           │
│  │  ├── wait()   — wait for child to exit            │
│  │  ├── kill()   — send signal to a process          │
│  │  └── PriorityScheduler — replaces round-robin     │
│  └──────────────────────────────────────────────────┘
│
│  Uses D13 Virtual Memory for address space operations
▼
Virtual Memory (D13)
│  clone_address_space()  — for fork()
│  create_address_space() — for exec()
│  destroy_address_space() — for exit()
▼
OS Kernel (S04) — manages PCBs, dispatches syscalls
```

**Depends on:** D13 Virtual Memory (fork needs COW clone, exec needs fresh
address space), S03 Interrupt Handler (signals delivered via interrupt
mechanism), S04 OS Kernel (extends the existing process table and scheduler)

**Used by:** User programs via system calls

## Key Concepts

### The fork/exec Split

Many students ask: "Why not just have a single `spawn(program)` call?" Windows
does this with `CreateProcess()`. Unix deliberately splits creation into two
steps, and the reason is flexibility.

Between fork() and exec(), the child process can set up its environment:

```
pid = fork()
if pid == 0:
    # I am the child. I can:
    close(stdout)              # close my stdout
    open("output.txt", WRITE)  # redirect stdout to a file
    exec("ls")                 # NOW run ls — its output goes to the file!
```

This is how shell redirection (`ls > output.txt`) works. With a single
`spawn()` call, you would need to pass all these setup options as parameters —
making the API complex and inflexible. The fork/exec split lets arbitrary code
run between creation and execution.

### Process Lifecycle

```
                          fork()
                            │
                            ▼
                    ┌───────────────┐
           ┌──────►│    READY       │◄──────────────────────┐
           │       │  (waiting for  │                        │
           │       │   CPU time)    │                        │
           │       └───────┬───────┘                        │
           │               │ scheduler picks this process   │
           │               ▼                                │
           │       ┌───────────────┐       I/O or           │
           │       │   RUNNING     │──── signal ───►┌───────┴──────┐
           │       │  (on CPU)     │                │   BLOCKED    │
           │       └───────┬───────┘◄───────────────│  (waiting)   │
           │               │                        └──────────────┘
     SIGCONT               │ sys_exit() or
           │               │ fatal signal
           │               ▼
           │       ┌───────────────┐     parent calls wait()
           │       │    ZOMBIE     │────────────────────────────┐
           │       │ (exited, but  │                            │
     ┌─────┴────┐  │  parent has   │                            ▼
     │ STOPPED  │  │  not waited)  │                     ┌──────────┐
     │(SIGSTOP) │  └───────────────┘                     │ REMOVED  │
     └──────────┘                                        │(reaped)  │
                                                         └──────────┘
```

**Why ZOMBIE?** When a process exits, the kernel cannot immediately delete its
PCB. The parent might call wait() later to retrieve the exit status. So the
kernel keeps the PCB around in a "zombie" state — the process is dead (no
address space, no open files), but its PID and exit status are preserved. The
parent's wait() call "reaps" the zombie, freeing the PCB entirely.

If the parent exits without waiting, the zombie's parent is reassigned to PID 1
(the init process), which periodically reaps orphaned zombies.

## Data Structures

### Extended Process Control Block (PCB)

The S04 kernel's PCB is extended with new fields for process relationships,
signals, and priority scheduling:

```
ProcessControlBlock:
┌──────────────────────────────────────────────────────────────────┐
│ === Existing fields (from S04) ===                               │
│ pid: int                      # Unique process identifier.      │
│ name: string                  # Human-readable name.            │
│ state: ProcessState           # Ready, Running, Blocked, Zombie,│
│                               # Stopped.                        │
│ saved_registers: [32]int      # RISC-V x0–x31 register values. │
│ saved_pc: int                 # Program counter at last switch. │
│                                                                  │
│ === New fields (D14) ===                                         │
│                                                                  │
│ parent_pid: int               # PID of the process that forked  │
│                               # this one. PID 0 (idle) and      │
│                               # PID 1 (init) have parent_pid=0. │
│                                                                  │
│ children: list[int]           # PIDs of all child processes.    │
│                               # Updated by fork() (add child)   │
│                               # and wait() (remove reaped child)│
│                                                                  │
│ exit_status: int              # Exit code set by sys_exit().    │
│                               # 0 = success, nonzero = error.   │
│                               # Only meaningful in Zombie state. │
│                                                                  │
│ priority: int                 # Scheduling priority, 0–39.      │
│   # 0 = highest priority (kernel tasks, real-time).              │
│   # 20 = default for user processes.                             │
│   # 39 = lowest priority (background/idle tasks).                │
│   #                                                              │
│   # Lower number = more CPU time.                                │
│   # This is the Unix convention: "nice" values.                  │
│                                                                  │
│ cpu_time: int                 # Total CPU cycles consumed by    │
│                               # this process across its entire  │
│                               # lifetime. Useful for profiling  │
│                               # and fair scheduling.            │
│                                                                  │
│ pending_signals: list[Signal] # Signals that have been sent to  │
│                               # this process but not yet         │
│                               # delivered. Delivered when the    │
│                               # process is next scheduled.       │
│                                                                  │
│ signal_handlers: map[Signal → address]                           │
│   # Custom signal handlers registered by the process.            │
│   # If a signal has no custom handler, the default action is     │
│   # used (usually: terminate the process).                       │
│   #                                                              │
│   # Example: a web server registers a SIGTERM handler that       │
│   # gracefully closes connections before exiting.                │
└──────────────────────────────────────────────────────────────────┘
```

### Signal Enum

Signals are software interrupts sent between processes. They are the Unix
mechanism for inter-process communication and process control.

```
Signal:
  SIGINT   = 2       # Interrupt. Sent when user presses Ctrl+C.
                     # Default action: terminate the process.
                     # Can be caught (e.g., to save work before exit).

  SIGKILL  = 9       # Kill. Unconditionally terminates the process.
                     # CANNOT be caught or ignored. This is the
                     # "nuclear option" — use SIGTERM first.

  SIGTERM  = 15      # Terminate. Polite request to exit.
                     # Default action: terminate.
                     # Can be caught (for graceful shutdown).
                     # This is what `kill <pid>` sends by default.

  SIGCHLD  = 17      # Child status changed. Sent to the parent when
                     # a child exits, is stopped, or is continued.
                     # Default action: ignore.
                     # The shell catches this to know when background
                     # jobs finish.

  SIGCONT  = 18      # Continue. Resume a stopped process.
                     # Sent by `fg` in the shell or `kill -CONT`.

  SIGSTOP  = 19      # Stop. Suspends the process.
                     # CANNOT be caught or ignored (like SIGKILL).
                     # Sent by Ctrl+Z in the shell.
```

**Why these specific numbers?** They are the standard POSIX signal numbers.
Every Unix-like system uses the same numbering. We implement only the most
essential signals; real systems define around 31 standard signals.

### Signal Delivery and Handling

```
Signal Delivery Flow:

  Process A calls sys_kill(pid_B, SIGTERM):
    │
    ▼
  Kernel: add SIGTERM to B's pending_signals list
    │
    ▼
  When B is next scheduled (context switch to B):
    │
    ▼
  Kernel checks B's pending_signals:
    "B has SIGTERM pending."
    │
    ├── Does B have a custom handler for SIGTERM?
    │   │
    │   ├── YES: Save B's current PC and registers.
    │   │        Set PC = handler address.
    │   │        B resumes in the handler function.
    │   │        When handler returns, kernel restores
    │   │        B's original PC and registers.
    │   │
    │   └── NO: Apply default action.
    │            For SIGTERM, default = terminate.
    │            Kernel sets B's state to Zombie.
    │
    └── Special cases:
        SIGKILL: always terminates, cannot be caught.
        SIGSTOP: always stops, cannot be caught.
        SIGCONT: always resumes, even if B has a handler.
```

### PriorityScheduler

Replaces the S04 round-robin scheduler with priority-based scheduling.

```
PriorityScheduler:
┌──────────────────────────────────────────────────────────────────┐
│ ready_queues: array[40] of queue[PCB]                            │
│   # One queue per priority level (0–39).                        │
│   # Each queue is FIFO within that priority level.              │
│   #                                                              │
│   # Priority 0:  [kernel_task_1, kernel_task_2]                  │
│   # Priority 1:  []                                              │
│   # ...                                                          │
│   # Priority 20: [user_shell, user_editor, user_browser]         │
│   # ...                                                          │
│   # Priority 39: [background_backup_job]                         │
│                                                                  │
│ current_process: PCB | None                                      │
│   # The currently running process.                               │
│                                                                  │
│ time_quantum: int                                                │
│   # How many CPU cycles a process gets before being preempted.  │
│   # Higher priority = larger quantum.                            │
│   # Priority 0: 200 cycles, Priority 20: 100 cycles,            │
│   # Priority 39: 50 cycles.                                     │
└──────────────────────────────────────────────────────────────────┘

Methods:

  schedule() → PCB
    # Called by the timer interrupt handler.
    # Finds the highest-priority non-empty queue.
    # Returns the front process from that queue.
    #
    # Algorithm:
    for priority in 0..40:
      if ready_queues[priority] is not empty:
        next_process = ready_queues[priority].dequeue()
        return next_process
    return idle_process    # nothing to run — idle

  enqueue(pcb: PCB) → None
    # Add a process to the appropriate ready queue
    # based on its priority field.
    ready_queues[pcb.priority].enqueue(pcb)

  preempt(current: PCB) → None
    # Put the current process back at the END of its
    # priority queue (round-robin within same priority).
    current.state = Ready
    enqueue(current)
```

**Why priority scheduling?** Round-robin treats all processes equally. But a
keyboard handler should respond faster than a background file indexer. Priority
scheduling ensures that interactive and time-critical processes get CPU time
before batch jobs.

**Starvation risk:** A continuous stream of high-priority processes could
prevent low-priority processes from ever running. Real schedulers address this
with "aging" — gradually boosting the priority of starved processes. We note
this as a future enhancement but do not implement it in this spec.

## Algorithms

### How fork() Works

fork() is the Unix mechanism for creating a new process. It creates an exact
copy of the calling process.

```
fork(parent_pid: int, mmu: MMU) → (parent_result: int, child_pid: int):

  Step 1: Allocate a new PID.
    child_pid = next_available_pid()
    # PIDs are assigned sequentially. Real systems reuse PIDs
    # after processes exit, but we keep it simple.

  Step 2: Create a new PCB by copying the parent's.
    parent_pcb = process_table[parent_pid]
    child_pcb = copy(parent_pcb)
    child_pcb.pid = child_pid
    child_pcb.parent_pid = parent_pid
    child_pcb.state = Ready
    child_pcb.children = []           # child starts with no children
    child_pcb.pending_signals = []    # no pending signals
    child_pcb.cpu_time = 0            # fresh CPU time counter

  Step 3: Clone the address space using copy-on-write.
    mmu.clone_address_space(parent_pid, child_pid)
    # This does NOT copy physical memory! It shares all frames
    # and marks them read-only. Actual copies happen lazily on
    # write faults. (See D13 for COW details.)

  Step 4: Set return values.
    # The magic of fork(): both processes resume from the same
    # point, but they see DIFFERENT return values.
    parent_pcb.saved_registers[A0] = child_pid   # parent sees child PID
    child_pcb.saved_registers[A0] = 0             # child sees 0

    # This is how the program knows which process it is:
    # pid = fork()
    # if pid == 0: "I am the child"
    # else: "I am the parent, child's PID is pid"

  Step 5: Update parent's children list.
    parent_pcb.children.append(child_pid)

  Step 6: Add child to the scheduler.
    scheduler.enqueue(child_pcb)
    process_table[child_pid] = child_pcb

  Step 7: Return.
    # Parent continues running with child_pid as return value.
    # Child will run when the scheduler picks it.
```

**What gets copied and what gets shared:**

```
┌────────────────────────┬──────────┬──────────────────────────────┐
│ What                   │ Copied?  │ Notes                        │
├────────────────────────┼──────────┼──────────────────────────────┤
│ Registers              │ Yes      │ Exact copy, except A0       │
│ Program counter        │ Yes      │ Both resume at same point   │
│ Address space          │ COW      │ Shared until written        │
│ PID                    │ No       │ Child gets a new PID        │
│ Parent PID             │ No       │ Child's parent = caller     │
│ Children list          │ No       │ Child starts with none      │
│ Pending signals        │ No       │ Child starts with none      │
│ Signal handlers        │ Yes      │ Inherited from parent       │
│ Priority               │ Yes      │ Same priority as parent     │
│ CPU time               │ No       │ Child starts at 0           │
└────────────────────────┴──────────┴──────────────────────────────┘
```

### How exec() Works

exec() replaces the current process's program with a new one. The PID stays
the same, but everything else changes.

```
exec(pid: int, binary: bytes, mmu: MMU) → None:

  Step 1: Destroy the old address space.
    mmu.destroy_address_space(pid)
    # Free all physical frames owned by this process.
    # If some frames were shared (COW), decrement their
    # reference counts but do not free them.

  Step 2: Create a fresh address space.
    mmu.create_address_space(pid)
    # Empty page table, ready for new mappings.

  Step 3: Load the binary into memory.
    # Parse the binary to find:
    #   - Code section: executable instructions
    #   - Data section: initialized global variables
    #   - BSS section: uninitialized globals (zeroed)
    #
    # For each section:
    #   Allocate physical frames.
    #   Map virtual pages with appropriate permissions:
    #     Code:  readable + executable (not writable!)
    #     Data:  readable + writable
    #     BSS:   readable + writable (zeroed)
    #   Copy section contents into the frames.

    load_section(pid, code, base=0x00010000, perms=RX)
    load_section(pid, data, base=code_end,   perms=RW)
    zero_section(pid, bss,  base=data_end,   perms=RW)

  Step 4: Set up the stack.
    # Allocate pages for the stack at a high virtual address.
    stack_top = 0x7FFFF000
    stack_pages = 4    # 16 KB stack
    for i in 0..stack_pages:
      frame = mmu.frame_allocator.allocate()
      mmu.map_page(pid, stack_top - (i * 4096), frame, RW)

  Step 5: Reset registers.
    pcb = process_table[pid]
    pcb.saved_registers = [0] * 32    # all registers to zero
    pcb.saved_registers[SP] = stack_top   # stack pointer
    pcb.saved_pc = 0x00010000             # entry point (start of code)

  Step 6: Clear pending signals and reset signal handlers.
    pcb.pending_signals = []
    pcb.signal_handlers = {}
    # The new program has no knowledge of the old program's
    # signal handlers. Reset to defaults.

  Step 7: Process continues running as the new program.
    # On the next context switch back to this process, it will
    # begin executing at the entry point of the new binary.
```

**What changes and what stays the same:**

```
┌────────────────────────┬──────────┬──────────────────────────────┐
│ What                   │ Changes? │ Notes                        │
├────────────────────────┼──────────┼──────────────────────────────┤
│ PID                    │ No       │ Same PID, new program        │
│ Parent PID             │ No       │ Same parent                  │
│ Children list          │ No       │ Children survive exec        │
│ Address space          │ Yes      │ Completely replaced          │
│ Registers              │ Yes      │ Zeroed (fresh start)         │
│ Program counter        │ Yes      │ Set to entry point           │
│ Signal handlers        │ Yes      │ Reset to defaults            │
│ Pending signals        │ Yes      │ Cleared                      │
│ Priority               │ No       │ Inherited from before exec   │
│ CPU time               │ No       │ Continues accumulating       │
└────────────────────────┴──────────┴──────────────────────────────┘
```

### How wait() Works

wait() lets a parent process block until a child exits. It retrieves the
child's exit status and reaps the zombie.

```
wait4(parent_pid: int, target_pid: int) → (child_pid: int, status: int):
  # If target_pid > 0: wait for that specific child.
  # If target_pid == -1: wait for ANY child.

  parent_pcb = process_table[parent_pid]

  # Verify the target is actually a child of this parent.
  if target_pid > 0 and target_pid not in parent_pcb.children:
    return (-1, ERROR_NOT_CHILD)

  loop:
    # Check if any matching child is already a zombie.
    for child_pid in parent_pcb.children:
      if target_pid > 0 and child_pid != target_pid:
        continue

      child_pcb = process_table[child_pid]
      if child_pcb.state == Zombie:
        # Found a zombie child — reap it.
        status = child_pcb.exit_status
        parent_pcb.children.remove(child_pid)
        del process_table[child_pid]     # free the PCB
        return (child_pid, status)

    # No zombie children yet. Block the parent.
    parent_pcb.state = Blocked
    parent_pcb.blocked_reason = WAITING_FOR_CHILD
    scheduler.schedule()    # switch to another process

    # When a child exits, it sends SIGCHLD to the parent,
    # which unblocks the parent and resumes this loop.
```

### How kill() Works

kill() sends a signal to a process (despite its name, it does not necessarily
kill the target — it delivers a signal, which might be caught).

```
kill(sender_pid: int, target_pid: int, signal: Signal) → int:

  target_pcb = process_table.get(target_pid)
  if target_pcb is None:
    return -1    # ERROR_NO_SUCH_PROCESS

  # Special handling for uncatchable signals:
  if signal == SIGKILL:
    # Immediately terminate. Cannot be caught or ignored.
    terminate_process(target_pid)
    return 0

  if signal == SIGSTOP:
    # Immediately stop. Cannot be caught or ignored.
    target_pcb.state = Stopped
    send_sigchld_to_parent(target_pid)
    return 0

  if signal == SIGCONT:
    # Resume a stopped process.
    if target_pcb.state == Stopped:
      target_pcb.state = Ready
      scheduler.enqueue(target_pcb)
      send_sigchld_to_parent(target_pid)
    return 0

  # For all other signals: add to pending list.
  # Will be delivered next time the process is scheduled.
  target_pcb.pending_signals.append(signal)
  return 0
```

### Process Groups and Sessions (Brief Overview)

Real Unix systems organize processes into groups and sessions:

```
Session (e.g., a terminal login)
├── Foreground process group
│   └── shell → fork/exec → ls (gets keyboard input)
│
├── Background process group 1
│   └── make -j8 (runs but does not get keyboard)
│
└── Background process group 2
    └── find / -name "*.log" (also in background)
```

- A **process group** is a set of related processes (e.g., a pipeline:
  `cat file | grep pattern | sort`). Signals like SIGINT (Ctrl+C) are sent
  to the entire foreground process group.

- A **session** is a collection of process groups, typically associated with
  a terminal. It has one foreground group and zero or more background groups.

We note these concepts for completeness but do not implement them in this spec.
They become important when implementing a shell or job control.

## Syscalls

### sys_fork (number 57)

```
sys_fork() → int
  # No arguments.
  # Returns:
  #   In the parent: child's PID (positive integer)
  #   In the child: 0
  #   On error: -1
  #
  # RISC-V calling convention:
  #   ecall with a7 = 57
  #   Return value in a0
```

### sys_exec (number 221)

```
sys_exec(path_addr: int, argv_addr: int) → int
  # path_addr: virtual address of the program path string
  # argv_addr: virtual address of the argument list (array of string pointers)
  #
  # On success: does not return! The calling program is replaced.
  # On failure: returns -1 (e.g., file not found, not executable)
  #
  # RISC-V:
  #   ecall with a7 = 221
  #   a0 = path_addr
  #   a1 = argv_addr
```

### sys_wait4 (number 260)

```
sys_wait4(pid: int, status_addr: int) → int
  # pid: which child to wait for (-1 = any child)
  # status_addr: virtual address where the exit status is written
  #
  # Returns: PID of the child that exited, or -1 on error
  # Blocks until a matching child exits.
  #
  # RISC-V:
  #   ecall with a7 = 260
  #   a0 = pid
  #   a1 = status_addr
  #   Return value in a0
```

### sys_kill (number 62)

```
sys_kill(pid: int, signal: int) → int
  # pid: target process
  # signal: signal number to send (e.g., 15 for SIGTERM)
  #
  # Returns: 0 on success, -1 on error
  #
  # RISC-V:
  #   ecall with a7 = 62
  #   a0 = pid
  #   a1 = signal
  #   Return value in a0
```

### sys_getpid (number 172)

```
sys_getpid() → int
  # No arguments.
  # Returns the PID of the calling process.
  #
  # RISC-V:
  #   ecall with a7 = 172
  #   Return value in a0
```

### sys_getppid (number 173)

```
sys_getppid() → int
  # No arguments.
  # Returns the PID of the calling process's parent.
  #
  # RISC-V:
  #   ecall with a7 = 173
  #   Return value in a0
```

## Dependencies

```
D14 Process Manager
│
├── depends on: D13 Virtual Memory
│   # fork() calls mmu.clone_address_space() for COW.
│   # exec() calls mmu.destroy_address_space() + create_address_space().
│   # exit() calls mmu.destroy_address_space().
│
├── depends on: S03 Interrupt Handler
│   # SIGCHLD is delivered via the interrupt mechanism.
│   # Timer interrupt (32) triggers the priority scheduler.
│
├── depends on: S04 OS Kernel
│   # Extends the existing process table and syscall dispatcher.
│   # Adds new syscalls to the syscall table.
│
└── used by: User programs
    # Shell: fork + exec to run commands.
    # Daemons: fork to run in background.
    # Servers: fork to handle concurrent clients.
```

## Testing Strategy

### Unit Tests

1. **PCB extensions:** Create a PCB, verify all new fields (parent_pid,
   children, exit_status, priority, cpu_time, pending_signals,
   signal_handlers) have correct initial values.

2. **Signal enum:** Verify all signal numbers match POSIX values (SIGINT=2,
   SIGKILL=9, SIGTERM=15, SIGCHLD=17, SIGCONT=18, SIGSTOP=19).

3. **PriorityScheduler:**
   - schedule() picks from the highest-priority non-empty queue.
   - With processes at priorities 5, 20, and 39: priority 5 runs first.
   - Two processes at the same priority: round-robin between them.
   - Empty scheduler returns idle process.
   - enqueue() places process in the correct priority queue.
   - Time quantum varies by priority (higher priority = larger quantum).

4. **fork():**
   - Child gets a new PID (different from parent).
   - Child's parent_pid equals parent's PID.
   - Parent's return value = child's PID.
   - Child's return value = 0.
   - Child appears in parent's children list.
   - Child's state = Ready.
   - Child's cpu_time = 0.
   - Child inherits parent's priority.
   - clone_address_space() is called (verify via mock/spy on MMU).

5. **exec():**
   - destroy_address_space() is called for the old space.
   - create_address_space() is called for the new space.
   - PC is set to the entry point.
   - Registers are zeroed (except SP).
   - Signal handlers are cleared.
   - PID does not change.
   - Parent PID does not change.

6. **wait():**
   - Parent blocks when no zombie children exist.
   - Parent unblocks when child exits (becomes zombie).
   - wait() returns the correct child PID and exit status.
   - Zombie PCB is removed after reaping.
   - wait(-1) returns any zombie child.
   - wait(specific_pid) only matches that child.
   - wait() for a non-child returns error.

7. **kill():**
   - SIGTERM adds to pending_signals.
   - SIGKILL immediately terminates (state = Zombie).
   - SIGSTOP immediately stops (state = Stopped).
   - SIGCONT resumes a stopped process (state = Ready).
   - kill() to nonexistent PID returns error.
   - After SIGKILL, parent receives SIGCHLD.

8. **Signal delivery:**
   - Pending signal is delivered on next schedule.
   - Custom handler: PC is redirected to handler address.
   - No handler: default action (terminate for SIGTERM).
   - SIGKILL/SIGSTOP cannot be caught (custom handler is ignored).

### Integration Tests

9. **fork + exec + wait lifecycle:**
   - Process A forks child B.
   - Child B execs a new program.
   - Child B exits with status 42.
   - Parent A waits and receives (pid=B, status=42).
   - Zombie B is reaped — no longer in process table.

10. **fork with COW verification:**
    - Parent writes data to a page.
    - fork() — child shares the page.
    - Child reads the page — same data (no COW fault).
    - Child writes to the page — COW fault, private copy created.
    - Parent reads the page — still sees original data.

11. **Priority preemption:**
    - Low-priority process is running.
    - High-priority process becomes ready (e.g., unblocked).
    - Next timer tick: scheduler picks high-priority process.

12. **Signal chain:**
    - Process A sends SIGTERM to process B.
    - Process B has a custom handler: handler runs, B continues.
    - Process A sends SIGKILL to process B.
    - Process B is terminated (handler cannot catch SIGKILL).
    - Process A's pending_signals contains SIGCHLD.

### Coverage Target

Target: 95%+ line coverage. Process management is the core of the kernel —
bugs in fork/exec/wait can cause orphaned processes, memory leaks, or security
holes. Every code path (success, error, edge case) must be tested.
