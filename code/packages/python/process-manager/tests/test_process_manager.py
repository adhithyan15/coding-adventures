"""Tests for ProcessManager — fork, exec, wait, kill, exit.

These tests verify the complete process lifecycle and the interactions
between fork, exec, wait, kill, and exit. They also test edge cases like
reparenting orphaned children and zombie reaping.
"""

from process_manager.pcb import ProcessState
from process_manager.process_manager import ProcessManager
from process_manager.signals import Signal


class TestCreateProcess:
    """Test basic process creation."""

    def test_first_process_gets_pid_0(self) -> None:
        """The first process created gets PID 0."""
        pm = ProcessManager()
        pcb = pm.create_process(name="init")
        assert pcb.pid == 0

    def test_sequential_pids(self) -> None:
        """PIDs are assigned sequentially."""
        pm = ProcessManager()
        p0 = pm.create_process(name="init")
        p1 = pm.create_process(name="shell", parent_pid=0)
        assert p0.pid == 0
        assert p1.pid == 1

    def test_create_with_parent(self) -> None:
        """Creating with a parent_pid adds the child to parent's children."""
        pm = ProcessManager()
        parent = pm.create_process(name="init")
        child = pm.create_process(name="shell", parent_pid=parent.pid)
        assert child.parent_pid == parent.pid
        assert child.pid in parent.children

    def test_create_with_priority(self) -> None:
        """Custom priority is respected."""
        pm = ProcessManager()
        pcb = pm.create_process(name="kernel", priority=0)
        assert pcb.priority == 0

    def test_create_with_memory(self) -> None:
        """Memory base and size are stored."""
        pm = ProcessManager()
        pcb = pm.create_process(memory_base=0x10000, memory_size=4096)
        assert pcb.memory_base == 0x10000
        assert pcb.memory_size == 4096

    def test_new_process_state_is_ready(self) -> None:
        """Newly created processes are in READY state."""
        pm = ProcessManager()
        pcb = pm.create_process()
        assert pcb.state == ProcessState.READY

    def test_process_count(self) -> None:
        """process_count reflects the number of processes."""
        pm = ProcessManager()
        assert pm.process_count == 0
        pm.create_process()
        assert pm.process_count == 1
        pm.create_process()
        assert pm.process_count == 2


class TestFork:
    """Test process forking — the Unix way to create processes."""

    def setup_method(self) -> None:
        """Create a ProcessManager with an init process."""
        self.pm = ProcessManager()
        self.init = self.pm.create_process(name="init", priority=0)

    def test_fork_returns_child_pid_and_zero(self) -> None:
        """fork() returns (child_pid, 0).

        The parent receives child_pid (positive), the child receives 0.
        """
        child_pid, child_ret = self.pm.fork(self.init.pid)
        assert child_pid > 0
        assert child_ret == 0

    def test_child_gets_new_pid(self) -> None:
        """Child PID is different from parent PID."""
        child_pid, _ = self.pm.fork(self.init.pid)
        assert child_pid != self.init.pid

    def test_child_parent_pid_set(self) -> None:
        """Child's parent_pid equals the parent's PID."""
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.parent_pid == self.init.pid

    def test_child_in_parent_children_list(self) -> None:
        """The child appears in the parent's children list."""
        child_pid, _ = self.pm.fork(self.init.pid)
        assert child_pid in self.init.children

    def test_child_state_is_ready(self) -> None:
        """The child starts in READY state."""
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.state == ProcessState.READY

    def test_child_inherits_registers(self) -> None:
        """Child gets a copy of parent's registers."""
        self.init.registers[10] = 42
        self.init.registers[15] = 99
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.registers[10] == 42
        assert child.registers[15] == 99

    def test_child_registers_are_independent(self) -> None:
        """Modifying child's registers does not affect parent's."""
        self.init.registers[10] = 42
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        child.registers[10] = 999
        assert self.init.registers[10] == 42

    def test_child_inherits_pc(self) -> None:
        """Child starts at the same PC as parent."""
        self.init.pc = 0x10000
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.pc == 0x10000

    def test_child_inherits_priority(self) -> None:
        """Child inherits parent's priority."""
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.priority == self.init.priority

    def test_child_cpu_time_is_zero(self) -> None:
        """Child starts with cpu_time = 0."""
        self.init.cpu_time = 500
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.cpu_time == 0

    def test_child_has_no_children(self) -> None:
        """Child starts with an empty children list."""
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.children == []

    def test_child_has_no_pending_signals(self) -> None:
        """Child starts with no pending signals."""
        self.init.pending_signals.append(int(Signal.SIGTERM))
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.pending_signals == []

    def test_child_inherits_signal_handlers(self) -> None:
        """Child inherits parent's signal handlers."""
        self.init.signal_handlers[int(Signal.SIGTERM)] = 0x40000
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.signal_handlers[int(Signal.SIGTERM)] == 0x40000

    def test_child_signal_handlers_independent(self) -> None:
        """Modifying child's handlers does not affect parent's."""
        self.init.signal_handlers[int(Signal.SIGTERM)] = 0x40000
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        child.signal_handlers[int(Signal.SIGTERM)] = 0x50000
        assert self.init.signal_handlers[int(Signal.SIGTERM)] == 0x40000

    def test_fork_nonexistent_parent_raises(self) -> None:
        """Forking from a non-existent PID raises ValueError."""
        try:
            self.pm.fork(999)
            assert False, "Should have raised ValueError"  # noqa: B011
        except ValueError:
            pass

    def test_fork_inherits_memory(self) -> None:
        """Child inherits parent's memory base and size."""
        self.init.memory_base = 0x20000
        self.init.memory_size = 8192
        child_pid, _ = self.pm.fork(self.init.pid)
        child = self.pm.get_process(child_pid)
        assert child is not None
        assert child.memory_base == 0x20000
        assert child.memory_size == 8192


class TestExec:
    """Test exec — replacing a process's program."""

    def setup_method(self) -> None:
        """Create a ProcessManager with a process to exec."""
        self.pm = ProcessManager()
        self.proc = self.pm.create_process(name="shell", priority=10)
        self.proc.registers[10] = 42
        self.proc.pc = 0x5000
        self.proc.signal_handlers[int(Signal.SIGTERM)] = 0x40000
        self.proc.pending_signals.append(int(Signal.SIGINT))

    def test_exec_returns_true(self) -> None:
        """exec returns True on success."""
        result = self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert result is True

    def test_exec_sets_pc(self) -> None:
        """PC is set to the entry point."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.pc == 0x10000

    def test_exec_sets_sp(self) -> None:
        """SP is set to the stack pointer."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.sp == 0x7FFFF000

    def test_exec_zeroes_registers(self) -> None:
        """All registers are zeroed after exec."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert all(r == 0 for r in self.proc.registers)

    def test_exec_clears_signal_handlers(self) -> None:
        """Signal handlers are cleared — new program doesn't know them."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.signal_handlers == {}

    def test_exec_clears_pending_signals(self) -> None:
        """Pending signals are cleared after exec."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.pending_signals == []

    def test_exec_keeps_pid(self) -> None:
        """PID does not change after exec."""
        old_pid = self.proc.pid
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.pid == old_pid

    def test_exec_keeps_parent(self) -> None:
        """Parent PID does not change after exec."""
        old_parent = self.proc.parent_pid
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.parent_pid == old_parent

    def test_exec_keeps_priority(self) -> None:
        """Priority is preserved after exec."""
        self.pm.exec(self.proc.pid, 0x10000, 0x7FFFF000)
        assert self.proc.priority == 10

    def test_exec_nonexistent_pid(self) -> None:
        """exec returns False for non-existent PID."""
        result = self.pm.exec(999, 0x10000, 0x7FFFF000)
        assert result is False

    def test_exec_updates_memory(self) -> None:
        """exec updates memory base and size when provided."""
        self.pm.exec(
            self.proc.pid, 0x10000, 0x7FFFF000,
            memory_base=0x20000, memory_size=8192,
        )
        assert self.proc.memory_base == 0x20000
        assert self.proc.memory_size == 8192


class TestWait:
    """Test wait — reaping zombie children."""

    def setup_method(self) -> None:
        """Create parent and child processes."""
        self.pm = ProcessManager()
        self.parent = self.pm.create_process(name="shell")
        self.child_pid, _ = self.pm.fork(self.parent.pid)

    def test_wait_returns_none_if_no_zombie(self) -> None:
        """wait returns None if no zombie children exist."""
        result = self.pm.wait(self.parent.pid)
        assert result is None

    def test_wait_reaps_zombie(self) -> None:
        """wait returns (pid, exit_code) for a zombie child."""
        self.pm.exit_process(self.child_pid, exit_code=42)
        result = self.pm.wait(self.parent.pid, self.child_pid)
        assert result == (self.child_pid, 42)

    def test_wait_removes_from_process_table(self) -> None:
        """After reaping, the child is removed from the process table."""
        self.pm.exit_process(self.child_pid, exit_code=0)
        self.pm.wait(self.parent.pid, self.child_pid)
        assert self.pm.get_process(self.child_pid) is None

    def test_wait_removes_from_children_list(self) -> None:
        """After reaping, the child is removed from parent's children list."""
        self.pm.exit_process(self.child_pid, exit_code=0)
        self.pm.wait(self.parent.pid, self.child_pid)
        assert self.child_pid not in self.parent.children

    def test_wait_any_child(self) -> None:
        """wait with child_pid=-1 reaps any zombie child."""
        child2_pid, _ = self.pm.fork(self.parent.pid)
        self.pm.exit_process(child2_pid, exit_code=7)
        result = self.pm.wait(self.parent.pid)  # -1 = any child
        assert result is not None
        assert result[0] == child2_pid
        assert result[1] == 7

    def test_wait_specific_child_ignores_others(self) -> None:
        """wait for a specific child ignores other zombie children."""
        child2_pid, _ = self.pm.fork(self.parent.pid)
        # Only child2 is a zombie.
        self.pm.exit_process(child2_pid, exit_code=7)
        # Wait specifically for child1 — should get None.
        result = self.pm.wait(self.parent.pid, self.child_pid)
        assert result is None

    def test_wait_nonexistent_parent(self) -> None:
        """wait with non-existent parent returns None."""
        result = self.pm.wait(999)
        assert result is None


class TestKill:
    """Test kill — sending signals to processes."""

    def setup_method(self) -> None:
        """Create a ProcessManager with a target process."""
        self.pm = ProcessManager()
        self.proc = self.pm.create_process(name="daemon")

    def test_kill_sigterm_adds_pending(self) -> None:
        """kill with SIGTERM adds to pending signals."""
        self.pm.kill(self.proc.pid, Signal.SIGTERM)
        assert int(Signal.SIGTERM) in self.proc.pending_signals

    def test_kill_sigkill_terminates(self) -> None:
        """kill with SIGKILL immediately sets ZOMBIE state."""
        self.proc.state = ProcessState.RUNNING
        self.pm.kill(self.proc.pid, Signal.SIGKILL)
        assert self.proc.state == ProcessState.ZOMBIE

    def test_kill_sigstop_blocks(self) -> None:
        """kill with SIGSTOP sets BLOCKED state."""
        self.proc.state = ProcessState.RUNNING
        self.pm.kill(self.proc.pid, Signal.SIGSTOP)
        assert self.proc.state == ProcessState.BLOCKED

    def test_kill_sigcont_resumes(self) -> None:
        """kill with SIGCONT resumes a BLOCKED process."""
        self.proc.state = ProcessState.BLOCKED
        self.pm.kill(self.proc.pid, Signal.SIGCONT)
        assert self.proc.state == ProcessState.READY

    def test_kill_nonexistent_pid(self) -> None:
        """kill returns False for non-existent PID."""
        result = self.pm.kill(999, Signal.SIGTERM)
        assert result is False

    def test_kill_returns_true_on_success(self) -> None:
        """kill returns True when signal is delivered."""
        result = self.pm.kill(self.proc.pid, Signal.SIGTERM)
        assert result is True


class TestExitProcess:
    """Test process termination and reparenting."""

    def setup_method(self) -> None:
        """Create a three-level process tree: init -> parent -> child."""
        self.pm = ProcessManager()
        self.init = self.pm.create_process(name="init", priority=0)
        self.parent_pid, _ = self.pm.fork(self.init.pid)
        self.child_pid, _ = self.pm.fork(self.parent_pid)

    def test_exit_sets_zombie(self) -> None:
        """exit sets the process state to ZOMBIE."""
        self.pm.exit_process(self.child_pid, exit_code=0)
        child = self.pm.get_process(self.child_pid)
        assert child is not None
        assert child.state == ProcessState.ZOMBIE

    def test_exit_records_exit_code(self) -> None:
        """exit records the exit code."""
        self.pm.exit_process(self.child_pid, exit_code=42)
        child = self.pm.get_process(self.child_pid)
        assert child is not None
        assert child.exit_code == 42

    def test_exit_reparents_children(self) -> None:
        """When a process exits, its children are reparented to PID 0 (init)."""
        # parent has child. If parent exits, child should be reparented to init.
        self.pm.exit_process(self.parent_pid, exit_code=0)
        child = self.pm.get_process(self.child_pid)
        assert child is not None
        assert child.parent_pid == 0

    def test_exit_sends_sigchld_to_parent(self) -> None:
        """exit sends SIGCHLD to the parent process."""
        self.pm.exit_process(self.child_pid, exit_code=0)
        parent = self.pm.get_process(self.parent_pid)
        assert parent is not None
        assert int(Signal.SIGCHLD) in parent.pending_signals

    def test_exit_nonexistent_pid(self) -> None:
        """exit on a non-existent PID does nothing (no crash)."""
        self.pm.exit_process(999)  # Should not raise

    def test_exit_clears_children_list(self) -> None:
        """After exit, the process's children list is cleared."""
        self.pm.exit_process(self.parent_pid, exit_code=0)
        parent = self.pm.get_process(self.parent_pid)
        assert parent is not None
        assert parent.children == []


class TestQueryMethods:
    """Test get_process, get_children, get_parent."""

    def setup_method(self) -> None:
        self.pm = ProcessManager()
        self.init = self.pm.create_process(name="init")
        self.child_pid, _ = self.pm.fork(self.init.pid)

    def test_get_process(self) -> None:
        """get_process returns the PCB for a valid PID."""
        pcb = self.pm.get_process(self.init.pid)
        assert pcb is not None
        assert pcb.pid == self.init.pid

    def test_get_process_nonexistent(self) -> None:
        """get_process returns None for invalid PID."""
        assert self.pm.get_process(999) is None

    def test_get_children(self) -> None:
        """get_children returns the list of child PIDs."""
        children = self.pm.get_children(self.init.pid)
        assert self.child_pid in children

    def test_get_children_nonexistent(self) -> None:
        """get_children returns empty list for invalid PID."""
        assert self.pm.get_children(999) == []

    def test_get_parent(self) -> None:
        """get_parent returns the parent PID."""
        assert self.pm.get_parent(self.child_pid) == self.init.pid

    def test_get_parent_nonexistent(self) -> None:
        """get_parent returns -1 for invalid PID."""
        assert self.pm.get_parent(999) == -1

    def test_signal_manager_property(self) -> None:
        """signal_manager property returns the SignalManager instance."""
        sm = self.pm.signal_manager
        assert sm is not None


class TestForkExecWaitLifecycle:
    """Integration tests for the complete fork+exec+wait lifecycle.

    This is the pattern used by every Unix shell:
        1. Shell forks a child.
        2. Child execs the command.
        3. Child exits.
        4. Shell waits and gets exit code.
    """

    def test_shell_runs_command(self) -> None:
        """Simulate: shell forks, child execs "ls", child exits, parent waits."""
        pm = ProcessManager()
        shell = pm.create_process(name="shell", priority=20)

        # Fork
        child_pid, child_ret = pm.fork(shell.pid)
        assert child_ret == 0

        # Exec (child replaces itself with "ls")
        pm.exec(child_pid, entry_point=0x10000, stack_pointer=0x7FFFF000)
        child = pm.get_process(child_pid)
        assert child is not None
        assert child.pc == 0x10000
        assert all(r == 0 for r in child.registers)

        # Child finishes and exits
        pm.exit_process(child_pid, exit_code=0)
        child = pm.get_process(child_pid)
        assert child is not None
        assert child.state == ProcessState.ZOMBIE

        # Parent waits
        result = pm.wait(shell.pid, child_pid)
        assert result == (child_pid, 0)

        # Zombie is reaped
        assert pm.get_process(child_pid) is None

    def test_shell_runs_failing_command(self) -> None:
        """Simulate: child exits with non-zero status (error)."""
        pm = ProcessManager()
        shell = pm.create_process(name="shell")
        child_pid, _ = pm.fork(shell.pid)
        pm.exec(child_pid, entry_point=0x10000, stack_pointer=0x7FFFF000)
        pm.exit_process(child_pid, exit_code=1)
        result = pm.wait(shell.pid, child_pid)
        assert result is not None
        assert result[1] == 1  # non-zero exit code

    def test_signal_chain(self) -> None:
        """Simulate: send SIGTERM (caught), then SIGKILL (uncatchable).

        1. Process B has a SIGTERM handler — SIGTERM is caught.
        2. Process A sends SIGKILL — B is terminated regardless.
        3. Parent receives SIGCHLD.
        """
        pm = ProcessManager()
        parent = pm.create_process(name="shell")
        child_pid, _ = pm.fork(parent.pid)
        child = pm.get_process(child_pid)
        assert child is not None

        # Register SIGTERM handler on child.
        pm.signal_manager.register_handler(child, Signal.SIGTERM, 0x40000)

        # Send SIGTERM — it goes to pending (handler exists).
        pm.kill(child_pid, Signal.SIGTERM)
        assert child.state != ProcessState.ZOMBIE  # Not killed

        # Now send SIGKILL — uncatchable.
        pm.kill(child_pid, Signal.SIGKILL)
        assert child.state == ProcessState.ZOMBIE  # Killed
