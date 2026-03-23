"""Priority Scheduler — Priority-based process scheduling.

The S04 OS kernel uses a simple round-robin scheduler: all processes take
turns equally, each getting the same time slice. This is fair but inefficient.
A keyboard handler (which needs to respond in milliseconds) gets the same
treatment as a background log rotator (which can wait seconds).

This module replaces round-robin with a priority-based scheduler.

How It Works
============

Every process has a priority from 0 (highest) to 39 (lowest). The scheduler
maintains one queue per priority level. When it is time to choose the next
process, it starts at priority 0 and works down until it finds a non-empty
queue. Within the same priority level, processes are scheduled round-robin.

    Priority 0:  [kernel_timer]
    Priority 1:  []
    Priority 2:  [keyboard_handler]
    ...
    Priority 20: [user_shell, user_editor, user_browser]
    ...
    Priority 39: [background_backup_job]

When schedule() is called:
    1. Check priority 0 — kernel_timer is there. Pick it.
    2. After kernel_timer runs, schedule() is called again.
    3. Priority 0 is empty now. Check priority 1 — empty.
    4. Check priority 2 — keyboard_handler is there. Pick it.
    5. And so on...

Within priority 20, the three processes (shell, editor, browser) take turns
in round-robin order: shell -> editor -> browser -> shell -> ...

Starvation Problem
==================

There is a risk: if high-priority processes never finish, low-priority ones
NEVER get to run. This is called "starvation." Real schedulers address this
with "aging" — gradually boosting the priority of processes that have been
waiting too long. For example, Linux's CFS (Completely Fair Scheduler) uses
a red-black tree keyed by virtual runtime.

We note this problem but do not solve it in this implementation. It is a
future enhancement.

Time Quantum
============

Higher-priority processes get a larger time quantum (more CPU cycles per
turn). The formula is:

    quantum = 200 - (priority * 4)

    Priority  0: 200 cycles (kernel tasks get the most time)
    Priority 20: 120 cycles (normal user processes)
    Priority 39:  44 cycles (background tasks)

This ensures that important processes not only run first but also run longer
when they do get the CPU.
"""

from __future__ import annotations

import contextlib
from collections import deque

# =============================================================================
# Constants
# =============================================================================

# The valid range of priorities. 0 is the highest priority (runs first),
# 39 is the lowest (runs last). These match Unix nice values shifted to
# be non-negative: nice -20 = priority 0, nice 0 = priority 20,
# nice 19 = priority 39.
MIN_PRIORITY = 0
MAX_PRIORITY = 39

# Default priority for new user processes. This is "nice 0" in Unix terms.
DEFAULT_PRIORITY = 20

# Base time quantum in CPU cycles. Higher priority processes get more cycles.
BASE_QUANTUM = 200

# How many cycles to subtract per priority level.
QUANTUM_PER_PRIORITY = 4


# =============================================================================
# PriorityScheduler
# =============================================================================


class PriorityScheduler:
    """Priority-based scheduler with round-robin within priority levels.

    The scheduler maintains a set of ready queues, one per priority level.
    When asked to schedule the next process, it scans from the highest
    priority (0) to the lowest (39) and picks the first process it finds.
    Within the same priority, processes are served in FIFO (round-robin)
    order.

    Attributes:
        _ready_queues: Dictionary mapping priority level to a deque of PIDs.
            Only priority levels that have been used are created (lazy init).
        _current_pid: The PID of the currently running process, or -1 if
            no process is running.
        _pid_priority: Maps PID -> priority for O(1) priority lookup.

    Example:
        >>> sched = PriorityScheduler()
        >>> sched.add_process(pid=1, priority=20)
        >>> sched.add_process(pid=2, priority=5)
        >>> sched.schedule()  # picks PID 2 (priority 5 is higher)
        2
        >>> sched.schedule()  # picks PID 1 (only one left)
        1
    """

    def __init__(self) -> None:
        """Initialize an empty scheduler with no ready processes."""
        # We use a dict of deques rather than a list[40] to save memory
        # when most priority levels are unused.
        self._ready_queues: dict[int, deque[int]] = {}
        self._current_pid: int = -1
        self._pid_priority: dict[int, int] = {}

    def add_process(self, pid: int, priority: int = DEFAULT_PRIORITY) -> None:
        """Add a process to the appropriate ready queue.

        The process is placed at the END of its priority queue (FIFO).
        This ensures round-robin ordering within the same priority level.

        Args:
            pid: The PID of the process to add.
            priority: The scheduling priority (0-39). Clamped to valid range.

        Example:
            >>> sched = PriorityScheduler()
            >>> sched.add_process(100, priority=20)
            >>> sched.add_process(101, priority=20)
            >>> sched.schedule()  # 100 was added first, runs first
            100
        """
        # Clamp priority to valid range.
        priority = max(MIN_PRIORITY, min(MAX_PRIORITY, priority))

        # Create the queue for this priority level if it doesn't exist.
        if priority not in self._ready_queues:
            self._ready_queues[priority] = deque()

        self._ready_queues[priority].append(pid)
        self._pid_priority[pid] = priority

    def remove_process(self, pid: int) -> None:
        """Remove a process from the scheduler.

        This is called when a process exits, blocks, or is otherwise no
        longer ready to run. The process is removed from whatever priority
        queue it is in.

        If the process is the currently running process, current_pid is
        reset to -1.

        Args:
            pid: The PID of the process to remove.
        """
        priority = self._pid_priority.pop(pid, None)
        if priority is not None and priority in self._ready_queues:
            queue = self._ready_queues[priority]
            with contextlib.suppress(ValueError):
                queue.remove(pid)

            # Clean up empty queues.
            if not queue:
                del self._ready_queues[priority]

        if self._current_pid == pid:
            self._current_pid = -1

    def schedule(self) -> int | None:
        """Select the next process to run.

        Scans priority levels from 0 (highest) to 39 (lowest). Returns the
        first PID found (front of the highest-priority non-empty queue).

        The selected process is REMOVED from the queue. If it should continue
        running after its time quantum, the caller must re-add it with
        add_process().

        Returns:
            The PID of the next process to run, or None if all queues are
            empty (nothing to run).

        Algorithm:
            for priority in 0..40:
                if ready_queues[priority] is not empty:
                    return ready_queues[priority].popleft()
            return None  # idle — nothing to run

        Example:
            >>> sched = PriorityScheduler()
            >>> sched.add_process(1, priority=20)
            >>> sched.add_process(2, priority=5)
            >>> sched.schedule()  # priority 5 beats priority 20
            2
        """
        # Scan from highest priority (0) to lowest (39).
        for priority in range(MIN_PRIORITY, MAX_PRIORITY + 1):
            queue = self._ready_queues.get(priority)
            if queue:
                pid = queue.popleft()

                # Clean up empty queues.
                if not queue:
                    del self._ready_queues[priority]

                self._current_pid = pid
                return pid

        # All queues empty — nothing to run.
        self._current_pid = -1
        return None

    def set_priority(self, pid: int, priority: int) -> None:
        """Change a process's priority.

        The process is moved from its current priority queue to the new one.
        It is placed at the END of the new queue (like a fresh arrival).

        This is similar to the Unix `renice` command.

        Args:
            pid: The PID of the process.
            priority: The new priority (0-39). Clamped to valid range.

        Example:
            >>> sched = PriorityScheduler()
            >>> sched.add_process(1, priority=20)
            >>> sched.set_priority(1, 5)  # promote to higher priority
            >>> sched.get_priority(1)
            5
        """
        # Clamp priority to valid range.
        priority = max(MIN_PRIORITY, min(MAX_PRIORITY, priority))

        old_priority = self._pid_priority.get(pid)
        if old_priority is None:
            # Process not known to the scheduler — just record the priority.
            self._pid_priority[pid] = priority
            return

        if old_priority == priority:
            return  # No change needed.

        # Remove from old queue.
        if old_priority in self._ready_queues:
            queue = self._ready_queues[old_priority]
            with contextlib.suppress(ValueError):
                queue.remove(pid)
            if not queue:
                del self._ready_queues[old_priority]

        # Add to new queue.
        if priority not in self._ready_queues:
            self._ready_queues[priority] = deque()
        self._ready_queues[priority].append(pid)
        self._pid_priority[pid] = priority

    def get_priority(self, pid: int) -> int:
        """Get the current priority of a process.

        Args:
            pid: The PID to query.

        Returns:
            The priority (0-39), or DEFAULT_PRIORITY (20) if the PID is
            not known to the scheduler.
        """
        return self._pid_priority.get(pid, DEFAULT_PRIORITY)

    @property
    def current_pid(self) -> int:
        """The PID of the currently running process, or -1 if none."""
        return self._current_pid

    def get_time_quantum(self, priority: int) -> int:
        """Calculate the time quantum for a given priority level.

        Higher priority (lower number) gets a larger time quantum. The
        formula is:

            quantum = BASE_QUANTUM - (priority * QUANTUM_PER_PRIORITY)
            quantum = 200 - (priority * 4)

        Results:
            Priority  0: 200 cycles
            Priority 20: 120 cycles
            Priority 39:  44 cycles

        Args:
            priority: The priority level (0-39).

        Returns:
            The time quantum in CPU cycles.
        """
        priority = max(MIN_PRIORITY, min(MAX_PRIORITY, priority))
        return BASE_QUANTUM - (priority * QUANTUM_PER_PRIORITY)

    @property
    def is_empty(self) -> bool:
        """Check if the scheduler has any ready processes."""
        return len(self._ready_queues) == 0
