"""Process Control Block (PCB) — The kernel's record of every process.

Every running program is represented in the kernel by a Process Control Block.
Think of it as a "save file" for a process — when the kernel needs to switch
from one process to another, it saves the current process's state into its PCB
and loads the next process's state from its PCB. This is called a "context
switch."

The S04 OS kernel had a minimal PCB with just a PID, name, state, registers,
and program counter. This module extends that PCB with the fields needed for
real process management:

    Parent/child relationships:
        Every process (except the very first one) was created by another
        process. The creator is the "parent," the created process is the
        "child." This forms a tree structure — the process tree — with the
        init process (PID 0) at the root.

    Signals:
        Processes can receive asynchronous notifications called "signals."
        A signal might tell the process to terminate (SIGTERM), to stop
        (SIGSTOP), or that a child has exited (SIGCHLD). The PCB tracks
        which signals are pending and which handlers the process has
        registered.

    Priority:
        Not all processes are equally important. A keyboard handler should
        respond faster than a background file indexer. Each process has a
        priority from 0 (highest) to 39 (lowest), following the Unix "nice"
        convention. The scheduler uses this to decide who runs next.

State Machine
=============

A process moves through these states during its lifetime:

    READY -------> RUNNING -------> BLOCKED
      ^              |                 |
      |              |                 |
      +--------------+---------<-------+
      |              |
      |              v
      |           ZOMBIE  --------> (reaped/removed)
      |              ^
      |              |
    STOPPED ---------+

    READY:       Waiting for CPU time. The scheduler will pick it eventually.
    RUNNING:     Currently executing on the CPU.
    BLOCKED:     Waiting for I/O or another event. Cannot run until unblocked.
    TERMINATED:  Finished execution. All resources freed.
    ZOMBIE:      Terminated but parent hasn't called wait() yet. The PCB is
                 kept around so the parent can retrieve the exit status.
    (STOPPED is handled via SIGSTOP/SIGCONT signals.)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum

# =============================================================================
# ProcessState — The Five States of a Process
# =============================================================================
#
# We use IntEnum so states can be compared with < and > if needed, and so they
# have nice string representations for debugging.


class ProcessState(IntEnum):
    """The possible states of a process.

    These map directly to the state machine diagram above. A process starts in
    READY, transitions to RUNNING when the scheduler picks it, may become
    BLOCKED waiting for I/O, and eventually reaches ZOMBIE when it exits
    (before the parent reaps it with wait()).
    """

    READY = 0
    RUNNING = 1
    BLOCKED = 2
    TERMINATED = 3
    ZOMBIE = 4  # Terminated but parent hasn't called wait() yet.


# =============================================================================
# ProcessControlBlock — The Kernel's Record of a Process
# =============================================================================
#
# Every field in this dataclass corresponds to a piece of information the
# kernel needs to manage the process. When the CPU switches from process A
# to process B, the kernel:
#   1. Saves A's current registers and PC into A's PCB.
#   2. Loads B's saved registers and PC from B's PCB.
#   3. Jumps to B's saved PC — and B resumes exactly where it left off.
#
# The new fields (parent_pid, children, signals, priority) enable the
# advanced features: fork/exec/wait, signal delivery, and priority scheduling.


@dataclass
class ProcessControlBlock:
    """Extended PCB with parent/child relationships, signals, and priority.

    Attributes:
        pid: Unique process identifier. Assigned sequentially starting from 0.
        name: Human-readable process name (e.g., "shell", "ls", "httpd").
        state: Current state in the process lifecycle.

        registers: The 32 RISC-V general-purpose registers (x0-x31).
            When a process is not running, its register values are saved here.
            x0 is always 0 in RISC-V (hardwired zero register).
            x10 (a0) is used for function return values and fork() results.

        pc: Program counter — the address of the next instruction to execute.
        sp: Stack pointer — points to the top of the process's stack.

        memory_base: Starting address of this process's memory region.
        memory_size: Size of the process's memory region in bytes.

        parent_pid: PID of the process that created this one via fork().
            The init process (PID 0) has parent_pid = -1 (no parent).

        children: List of PIDs of all child processes created by this process.
            Updated by fork() (adds child) and wait() (removes reaped child).

        pending_signals: Signals that have been sent to this process but not
            yet delivered. They will be processed when the process is next
            scheduled.

        signal_handlers: Map from signal number to the address of a custom
            handler function. If a signal is not in this map, the default
            action is used (usually: terminate).

        signal_mask: Set of signal numbers that are currently blocked. A
            blocked signal is not delivered — it stays in pending_signals
            until unmasked. SIGKILL and SIGSTOP can never be masked.

        priority: Scheduling priority, 0-39. Lower number = higher priority.
            0 = highest priority (kernel tasks, real-time).
            20 = default for user processes (like Unix "nice 0").
            39 = lowest priority (background/idle tasks).

        cpu_time: Total CPU cycles consumed by this process. Useful for
            profiling and fair scheduling decisions.

        exit_code: Exit status set by the process when it terminates.
            0 = success, nonzero = error. Only meaningful in ZOMBIE state.
    """

    pid: int
    name: str = ""
    state: ProcessState = ProcessState.READY

    # --- Saved CPU state ---
    # These 32 registers correspond to RISC-V x0-x31.
    # When we save/restore context, we save ALL 32, even though x0 is always 0.
    registers: list[int] = field(default_factory=lambda: [0] * 32)
    pc: int = 0
    sp: int = 0

    # --- Memory region ---
    memory_base: int = 0
    memory_size: int = 0

    # --- Process relationships ---
    # parent_pid = -1 means "no parent" (only for the very first process).
    parent_pid: int = -1
    children: list[int] = field(default_factory=list)

    # --- Signals ---
    # pending_signals: signals waiting to be delivered.
    # signal_handlers: custom handlers registered by the process.
    # signal_mask: signals that are currently blocked from delivery.
    pending_signals: list[int] = field(default_factory=list)
    signal_handlers: dict[int, int] = field(default_factory=dict)
    signal_mask: set[int] = field(default_factory=set)

    # --- Scheduling ---
    priority: int = 20  # Default: normal user process.
    cpu_time: int = 0  # Total cycles consumed.

    # --- Exit info ---
    exit_code: int = 0
