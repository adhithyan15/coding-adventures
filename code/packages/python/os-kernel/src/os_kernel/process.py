"""Process Control Block and process states.

Every process is in exactly one state at any time:

    (none) --CreateProcess--> Ready
    Ready --Scheduled--> Running
    Running --Timer tick / sys_yield--> Ready
    Running --sys_exit--> Terminated
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum


class ProcessState(IntEnum):
    """Execution state of a process."""

    READY = 0
    RUNNING = 1
    BLOCKED = 2
    TERMINATED = 3

    def __str__(self: ProcessState) -> str:
        names = {0: "Ready", 1: "Running", 2: "Blocked", 3: "Terminated"}
        return names.get(self.value, "Unknown")


# Convenient aliases
PROCESS_READY = ProcessState.READY
PROCESS_RUNNING = ProcessState.RUNNING
PROCESS_BLOCKED = ProcessState.BLOCKED
PROCESS_TERMINATED = ProcessState.TERMINATED


@dataclass
class ProcessControlBlock:
    """Holds all state for a single process.

    Attributes:
        pid: Unique process identifier.
        state: Current execution state.
        saved_registers: All 32 RISC-V registers for context switching.
        saved_pc: Program counter where this process should resume.
        stack_pointer: Top of this process's stack.
        memory_base: Start address of this process's memory region.
        memory_size: Size of this process's memory region.
        name: Human-readable identifier (e.g., "idle", "hello-world").
        exit_code: Set by sys_exit when the process terminates.
    """

    pid: int = 0
    state: ProcessState = ProcessState.READY
    saved_registers: list[int] = field(default_factory=lambda: [0] * 32)
    saved_pc: int = 0
    stack_pointer: int = 0
    memory_base: int = 0
    memory_size: int = 0
    name: str = ""
    exit_code: int = 0


@dataclass
class ProcessInfo:
    """Lightweight summary of a process for snapshots and debugging."""

    pid: int = 0
    name: str = ""
    state: ProcessState = ProcessState.READY
    pc: int = 0
