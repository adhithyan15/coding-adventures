"""Tests for the ProcessControlBlock and ProcessState.

These tests verify that the extended PCB has correct default values,
supports state transitions, and properly stores all fields needed for
process management (parent/child relationships, signals, priority).
"""

from process_manager.pcb import ProcessControlBlock, ProcessState

# =============================================================================
# ProcessState Tests
# =============================================================================


class TestProcessState:
    """Verify the process state enum values and properties."""

    def test_state_values(self) -> None:
        """Each state has the expected integer value.

        These values are used internally for comparisons and serialization.
        They must not change without updating all consumers.
        """
        assert ProcessState.READY == 0
        assert ProcessState.RUNNING == 1
        assert ProcessState.BLOCKED == 2
        assert ProcessState.TERMINATED == 3
        assert ProcessState.ZOMBIE == 4

    def test_state_count(self) -> None:
        """There are exactly 5 states in the lifecycle."""
        assert len(ProcessState) == 5

    def test_state_ordering(self) -> None:
        """States can be compared (IntEnum supports <, >)."""
        assert ProcessState.READY < ProcessState.RUNNING
        assert ProcessState.ZOMBIE > ProcessState.BLOCKED

    def test_state_names(self) -> None:
        """Each state has a meaningful name for debugging."""
        assert ProcessState.READY.name == "READY"
        assert ProcessState.ZOMBIE.name == "ZOMBIE"


# =============================================================================
# ProcessControlBlock Tests
# =============================================================================


class TestProcessControlBlock:
    """Verify PCB creation, defaults, and field behavior."""

    def test_creation_with_pid(self) -> None:
        """A PCB requires a PID at minimum."""
        pcb = ProcessControlBlock(pid=42)
        assert pcb.pid == 42

    def test_default_name(self) -> None:
        """Name defaults to empty string."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.name == ""

    def test_default_state(self) -> None:
        """New processes start in READY state."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.state == ProcessState.READY

    def test_default_registers(self) -> None:
        """All 32 registers default to 0.

        RISC-V has 32 general-purpose registers (x0-x31). A new process
        starts with all registers zeroed.
        """
        pcb = ProcessControlBlock(pid=0)
        assert len(pcb.registers) == 32
        assert all(r == 0 for r in pcb.registers)

    def test_default_pc_and_sp(self) -> None:
        """PC and SP default to 0."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.pc == 0
        assert pcb.sp == 0

    def test_default_memory(self) -> None:
        """Memory base and size default to 0."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.memory_base == 0
        assert pcb.memory_size == 0

    def test_default_parent_pid(self) -> None:
        """Parent PID defaults to -1 (no parent)."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.parent_pid == -1

    def test_default_children(self) -> None:
        """Children list starts empty."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.children == []

    def test_default_signals(self) -> None:
        """Signal-related fields start empty."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.pending_signals == []
        assert pcb.signal_handlers == {}
        assert pcb.signal_mask == set()

    def test_default_priority(self) -> None:
        """Default priority is 20 (normal user process)."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.priority == 20

    def test_default_cpu_time(self) -> None:
        """CPU time starts at 0."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.cpu_time == 0

    def test_default_exit_code(self) -> None:
        """Exit code defaults to 0."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.exit_code == 0

    def test_custom_fields(self) -> None:
        """All fields can be set at creation time."""
        pcb = ProcessControlBlock(
            pid=5,
            name="httpd",
            state=ProcessState.RUNNING,
            priority=10,
            parent_pid=1,
            memory_base=0x10000,
            memory_size=4096,
        )
        assert pcb.pid == 5
        assert pcb.name == "httpd"
        assert pcb.state == ProcessState.RUNNING
        assert pcb.priority == 10
        assert pcb.parent_pid == 1
        assert pcb.memory_base == 0x10000
        assert pcb.memory_size == 4096

    def test_state_transition(self) -> None:
        """State can be changed to simulate lifecycle transitions."""
        pcb = ProcessControlBlock(pid=0)
        assert pcb.state == ProcessState.READY

        pcb.state = ProcessState.RUNNING
        assert pcb.state == ProcessState.RUNNING

        pcb.state = ProcessState.ZOMBIE
        assert pcb.state == ProcessState.ZOMBIE

    def test_register_independence(self) -> None:
        """Each PCB has its own independent register set.

        This is critical for context switching — modifying one process's
        registers must not affect another's.
        """
        pcb1 = ProcessControlBlock(pid=0)
        pcb2 = ProcessControlBlock(pid=1)

        pcb1.registers[10] = 42
        assert pcb2.registers[10] == 0

    def test_children_list_independence(self) -> None:
        """Each PCB has its own children list."""
        pcb1 = ProcessControlBlock(pid=0)
        pcb2 = ProcessControlBlock(pid=1)

        pcb1.children.append(99)
        assert pcb2.children == []

    def test_signal_handlers_independence(self) -> None:
        """Each PCB has its own signal handlers dict."""
        pcb1 = ProcessControlBlock(pid=0)
        pcb2 = ProcessControlBlock(pid=1)

        pcb1.signal_handlers[15] = 0x40000
        assert pcb2.signal_handlers == {}
