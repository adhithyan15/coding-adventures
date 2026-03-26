"""Tests for PriorityScheduler.

These tests verify that the priority scheduler correctly picks the highest
priority process, uses round-robin within the same priority level, and
supports priority changes.
"""

from process_manager.priority_scheduler import (
    BASE_QUANTUM,
    DEFAULT_PRIORITY,
    MAX_PRIORITY,
    MIN_PRIORITY,
    QUANTUM_PER_PRIORITY,
    PriorityScheduler,
)


class TestAddAndSchedule:
    """Test adding processes and scheduling them."""

    def test_schedule_empty(self) -> None:
        """Empty scheduler returns None."""
        sched = PriorityScheduler()
        assert sched.schedule() is None

    def test_schedule_single_process(self) -> None:
        """Single process is scheduled."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        assert sched.schedule() == 1

    def test_schedule_picks_highest_priority(self) -> None:
        """Higher priority (lower number) runs first.

        Priority 5 > Priority 20 > Priority 39.
        """
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.add_process(2, priority=5)
        sched.add_process(3, priority=39)

        assert sched.schedule() == 2  # priority 5 first
        assert sched.schedule() == 1  # priority 20 second
        assert sched.schedule() == 3  # priority 39 last

    def test_round_robin_within_priority(self) -> None:
        """Same-priority processes are scheduled round-robin (FIFO).

        Three processes at priority 20:
            First call: PID 10 (added first)
            Second call: PID 11 (added second)
            Third call: PID 12 (added third)
        """
        sched = PriorityScheduler()
        sched.add_process(10, priority=20)
        sched.add_process(11, priority=20)
        sched.add_process(12, priority=20)

        assert sched.schedule() == 10
        assert sched.schedule() == 11
        assert sched.schedule() == 12

    def test_schedule_removes_from_queue(self) -> None:
        """Scheduled processes are removed from the ready queue.

        After scheduling, the process must be re-added to run again.
        """
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.schedule()  # removes PID 1
        assert sched.schedule() is None  # queue is now empty

    def test_priority_clamped_low(self) -> None:
        """Priority below 0 is clamped to 0."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=-5)
        assert sched.get_priority(1) == 0

    def test_priority_clamped_high(self) -> None:
        """Priority above 39 is clamped to 39."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=100)
        assert sched.get_priority(1) == 39


class TestRemoveProcess:
    """Test removing processes from the scheduler."""

    def test_remove_process(self) -> None:
        """Removing a process takes it out of the ready queue."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.remove_process(1)
        assert sched.schedule() is None

    def test_remove_nonexistent(self) -> None:
        """Removing a non-existent process does not crash."""
        sched = PriorityScheduler()
        sched.remove_process(999)  # Should not raise

    def test_remove_resets_current_pid(self) -> None:
        """Removing the current process resets current_pid to -1."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.schedule()  # PID 1 is now current
        sched.remove_process(1)
        assert sched.current_pid == -1

    def test_remove_from_middle(self) -> None:
        """Removing a process from the middle of a queue works."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.add_process(2, priority=20)
        sched.add_process(3, priority=20)
        sched.remove_process(2)
        assert sched.schedule() == 1
        assert sched.schedule() == 3


class TestSetPriority:
    """Test changing a process's priority."""

    def test_set_priority(self) -> None:
        """Changing priority moves the process to the new queue."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.set_priority(1, 5)
        assert sched.get_priority(1) == 5

    def test_set_priority_affects_scheduling(self) -> None:
        """After priority change, the process is picked at the new level."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.add_process(2, priority=20)

        # Promote PID 1 to priority 5.
        sched.set_priority(1, 5)

        # PID 1 should now be scheduled first (priority 5 > 20).
        assert sched.schedule() == 1
        assert sched.schedule() == 2

    def test_set_same_priority_no_change(self) -> None:
        """Setting the same priority has no effect."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.set_priority(1, 20)
        assert sched.get_priority(1) == 20
        assert sched.schedule() == 1

    def test_set_priority_clamped(self) -> None:
        """Priority values are clamped to valid range."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.set_priority(1, -10)
        assert sched.get_priority(1) == 0

    def test_set_priority_unknown_pid(self) -> None:
        """Setting priority for unknown PID just records it."""
        sched = PriorityScheduler()
        sched.set_priority(99, 5)
        assert sched.get_priority(99) == 5


class TestGetPriority:
    """Test priority queries."""

    def test_known_pid(self) -> None:
        """Returns the actual priority for a known PID."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=10)
        assert sched.get_priority(1) == 10

    def test_unknown_pid(self) -> None:
        """Returns DEFAULT_PRIORITY (20) for an unknown PID."""
        sched = PriorityScheduler()
        assert sched.get_priority(999) == DEFAULT_PRIORITY


class TestCurrentPid:
    """Test the current_pid property."""

    def test_initial_current_pid(self) -> None:
        """current_pid starts at -1 (no process running)."""
        sched = PriorityScheduler()
        assert sched.current_pid == -1

    def test_current_pid_after_schedule(self) -> None:
        """current_pid is set to the scheduled process."""
        sched = PriorityScheduler()
        sched.add_process(42, priority=20)
        sched.schedule()
        assert sched.current_pid == 42

    def test_current_pid_after_empty_schedule(self) -> None:
        """current_pid is -1 when nothing is scheduled."""
        sched = PriorityScheduler()
        sched.schedule()
        assert sched.current_pid == -1


class TestTimeQuantum:
    """Test time quantum calculation."""

    def test_highest_priority_quantum(self) -> None:
        """Priority 0 gets the maximum quantum."""
        sched = PriorityScheduler()
        assert sched.get_time_quantum(0) == BASE_QUANTUM

    def test_default_priority_quantum(self) -> None:
        """Priority 20 gets a medium quantum."""
        sched = PriorityScheduler()
        expected = BASE_QUANTUM - (20 * QUANTUM_PER_PRIORITY)
        assert sched.get_time_quantum(20) == expected

    def test_lowest_priority_quantum(self) -> None:
        """Priority 39 gets the minimum quantum."""
        sched = PriorityScheduler()
        expected = BASE_QUANTUM - (39 * QUANTUM_PER_PRIORITY)
        assert sched.get_time_quantum(39) == expected

    def test_quantum_clamped(self) -> None:
        """Out-of-range priority is clamped before calculation."""
        sched = PriorityScheduler()
        assert sched.get_time_quantum(-5) == BASE_QUANTUM
        assert sched.get_time_quantum(100) == BASE_QUANTUM - (39 * QUANTUM_PER_PRIORITY)


class TestIsEmpty:
    """Test the is_empty property."""

    def test_empty_scheduler(self) -> None:
        """A new scheduler is empty."""
        sched = PriorityScheduler()
        assert sched.is_empty is True

    def test_non_empty_scheduler(self) -> None:
        """Scheduler with processes is not empty."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        assert sched.is_empty is False

    def test_empty_after_schedule(self) -> None:
        """Scheduler is empty after all processes are scheduled."""
        sched = PriorityScheduler()
        sched.add_process(1, priority=20)
        sched.schedule()
        assert sched.is_empty is True


class TestConstants:
    """Verify module-level constants."""

    def test_priority_range(self) -> None:
        assert MIN_PRIORITY == 0
        assert MAX_PRIORITY == 39

    def test_default_priority(self) -> None:
        assert DEFAULT_PRIORITY == 20

    def test_base_quantum(self) -> None:
        assert BASE_QUANTUM == 200

    def test_quantum_per_priority(self) -> None:
        assert QUANTUM_PER_PRIORITY == 4
