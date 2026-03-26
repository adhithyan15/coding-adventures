"""Round-robin process scheduler.

The scheduler decides which process runs next. Each process gets an
equal time slice (driven by timer interrupts), and processes take
turns in order.

When the timer fires:
  1. Save current process registers to its PCB
  2. Set current process state = Ready
  3. Pick next Ready process (round-robin order)
  4. Load next process registers from its PCB
  5. Set next process state = Running
  6. Return from interrupt -> CPU now runs the next process
"""

from __future__ import annotations

from os_kernel.process import ProcessControlBlock, ProcessState


class Scheduler:
    """Manages the process table and selects the next process to run."""

    def __init__(self: Scheduler, process_table: list[ProcessControlBlock]) -> None:
        self.process_table = process_table
        self.current = 0

    def schedule(self: Scheduler) -> int:
        """Pick the next Ready process using round-robin.

        Returns the PID of the next process to run.
        """
        n = len(self.process_table)
        if n == 0:
            return 0

        for i in range(1, n + 1):
            idx = (self.current + i) % n
            if self.process_table[idx].state == ProcessState.READY:
                return idx

        if self.current < n and self.process_table[self.current].state == ProcessState.READY:
            return self.current

        return 0

    def context_switch(self: Scheduler, from_pid: int, to_pid: int) -> None:
        """Update process states for a context switch.

        Outgoing: Running -> Ready (unless Terminated)
        Incoming: Ready -> Running
        """
        if 0 <= from_pid < len(self.process_table):
            if self.process_table[from_pid].state == ProcessState.RUNNING:
                self.process_table[from_pid].state = ProcessState.READY
        if 0 <= to_pid < len(self.process_table):
            self.process_table[to_pid].state = ProcessState.RUNNING
        self.current = to_pid
