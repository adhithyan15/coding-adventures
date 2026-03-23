"""Process Manager — Core process lifecycle management.

This module implements the four fundamental Unix process operations:

    fork()  — Clone a running process.
    exec()  — Replace a process's program with a new one.
    wait()  — Wait for a child process to exit and retrieve its exit code.
    kill()  — Send a signal to a process.

Together with exit(), these operations form the complete process lifecycle.
Every program you have ever run on a Unix system was created by fork+exec
and cleaned up by exit+wait.

How Your Shell Works
====================

When you type "ls" in a terminal, this is what happens:

    1. The shell (e.g., bash) calls fork().
       Now there are TWO copies of the shell running.

    2. The child copy calls exec("ls").
       The child is no longer a shell — it is now the "ls" program.

    3. "ls" runs, prints the directory listing, and calls exit(0).
       The child becomes a zombie (state = ZOMBIE).

    4. The parent shell calls wait().
       It receives the child's exit code (0 = success).
       The zombie is reaped (PCB removed from the process table).

    5. The parent shell prints the next prompt. Ready for the next command.

This fork+exec pattern is used everywhere in Unix:
    - Web servers fork() to handle each client connection.
    - Daemons fork() twice to detach from the terminal.
    - init (PID 1) fork+execs every system service at boot.
    - Shells fork+exec every command you type.

The PID 0 Convention
====================

PID 0 is special in our system. It acts as the "init" process — the ancestor
of all other processes. When a process exits without its parent waiting,
its children are "reparented" to PID 0. This prevents orphaned zombies from
accumulating, because PID 0 (init) periodically reaps them.

In real Unix systems, the idle process is PID 0 and init is PID 1, but for
our educational implementation, we combine them into PID 0.
"""

from __future__ import annotations

from process_manager.pcb import ProcessControlBlock, ProcessState
from process_manager.signals import Signal, SignalManager


class ProcessManager:
    """Manages the complete process lifecycle.

    The ProcessManager is the kernel's process subsystem. It owns the process
    table (a dictionary mapping PIDs to PCBs) and provides the system calls
    that user programs invoke to create, manage, and terminate processes.

    Attributes:
        _processes: The process table. Maps PID -> ProcessControlBlock.
        _next_pid: The next PID to assign. Incremented after each allocation.
        _signal_manager: Handles signal delivery for all processes.

    Example:
        >>> pm = ProcessManager()
        >>> init = pm.create_process(name="init", priority=0)
        >>> init.pid
        0
        >>> child_pid, child_ret = pm.fork(init.pid)
        >>> child_ret  # child sees 0 from fork
        0
    """

    def __init__(self) -> None:
        """Initialize an empty process manager.

        The process table starts empty. The first call to create_process()
        will allocate PID 0 (the init process).
        """
        self._processes: dict[int, ProcessControlBlock] = {}
        self._next_pid: int = 0
        self._signal_manager = SignalManager()

    # =========================================================================
    # Process Creation
    # =========================================================================

    def create_process(
        self,
        name: str = "",
        parent_pid: int = -1,
        priority: int = 20,
        memory_base: int = 0,
        memory_size: int = 0,
    ) -> ProcessControlBlock:
        """Create a new process with a unique PID.

        This is the low-level process creation mechanism. It allocates a fresh
        PCB, assigns a unique PID, and adds it to the process table. Higher-
        level operations like fork() use this internally.

        Args:
            name: Human-readable name for debugging (e.g., "shell", "ls").
            parent_pid: PID of the parent process. -1 for the root process.
            priority: Scheduling priority, 0 (highest) to 39 (lowest).
                      Default is 20 (normal user process).
            memory_base: Starting address of the process's memory region.
            memory_size: Size of the process's memory region in bytes.

        Returns:
            The newly created ProcessControlBlock.

        Example:
            >>> pm = ProcessManager()
            >>> pcb = pm.create_process(name="init", priority=0)
            >>> pcb.pid
            0
            >>> pcb.state
            <ProcessState.READY: 0>
        """
        pid = self._next_pid
        self._next_pid += 1

        pcb = ProcessControlBlock(
            pid=pid,
            name=name,
            state=ProcessState.READY,
            parent_pid=parent_pid,
            priority=priority,
            memory_base=memory_base,
            memory_size=memory_size,
        )

        self._processes[pid] = pcb

        # If this process has a parent, add it to the parent's children list.
        if parent_pid >= 0 and parent_pid in self._processes:
            self._processes[parent_pid].children.append(pid)

        return pcb

    # =========================================================================
    # fork() — Clone a Process
    # =========================================================================
    #
    # fork() is the most unusual system call in computing. It creates a new
    # process that is an EXACT COPY of the calling process. Both processes
    # resume at the same point in the code, but they receive different return
    # values:
    #
    #   - The parent receives the child's PID (a positive integer).
    #   - The child receives 0.
    #
    # This is how the program knows which copy it is:
    #
    #   pid = fork()
    #   if pid == 0:
    #       print("I am the child!")
    #   else:
    #       print(f"I am the parent. My child is PID {pid}.")

    def fork(self, parent_pid: int) -> tuple[int, int]:
        """Fork: create a child process as a copy of the parent.

        The child gets:
            - A new, unique PID.
            - A copy of the parent's registers (including PC).
            - The same priority as the parent.
            - The same memory base and size as the parent.
            - The same signal handlers as the parent.
            - An empty children list (no grandchildren).
            - No pending signals (clean slate).
            - cpu_time reset to 0.

        The parent gets:
            - The child's PID added to its children list.

        Returns:
            A tuple (child_pid, 0) where:
                - child_pid is the PID assigned to the new child process.
                  The parent would see this as the return value of fork().
                - 0 is what the child would see as the return value of fork().

            In a real kernel, fork() returns child_pid to the parent and 0 to
            the child by writing different values into their respective a0
            registers. Here, we return both values as a tuple so the caller
            can simulate this behavior.

        Raises:
            ValueError: If parent_pid does not exist in the process table.

        Example:
            >>> pm = ProcessManager()
            >>> parent = pm.create_process(name="shell")
            >>> parent.registers[10] = 42  # some state in a0
            >>> child_pid, child_ret = pm.fork(parent.pid)
            >>> child_ret
            0
            >>> child = pm.get_process(child_pid)
            >>> child.parent_pid == parent.pid
            True
        """
        parent = self._processes.get(parent_pid)
        if parent is None:
            msg = f"No process with PID {parent_pid}"
            raise ValueError(msg)

        # Allocate a new PID for the child.
        child_pid = self._next_pid
        self._next_pid += 1

        # Create the child PCB as a copy of the parent.
        # We copy each field explicitly to make it clear what is copied,
        # what is shared, and what is reset.
        child = ProcessControlBlock(
            pid=child_pid,
            name=parent.name,  # Same name (will be changed by exec).
            state=ProcessState.READY,  # Child starts ready to run.
            # --- Copied from parent ---
            registers=list(parent.registers),  # Deep copy of registers.
            pc=parent.pc,  # Same program counter.
            sp=parent.sp,  # Same stack pointer.
            memory_base=parent.memory_base,  # Same memory region.
            memory_size=parent.memory_size,
            priority=parent.priority,  # Inherit priority.
            signal_handlers=dict(parent.signal_handlers),  # Copy handlers.
            # --- Reset for child ---
            parent_pid=parent_pid,  # My parent is the caller.
            children=[],  # I have no children yet.
            pending_signals=[],  # No pending signals.
            signal_mask=set(parent.signal_mask),  # Copy mask.
            cpu_time=0,  # Fresh CPU time counter.
            exit_code=0,
        )

        # Add child to the process table.
        self._processes[child_pid] = child

        # Update parent's children list.
        parent.children.append(child_pid)

        # Return (child_pid_for_parent, 0_for_child).
        # In a real kernel, we would write child_pid into parent's a0
        # register and 0 into child's a0 register. Here, we return both
        # values so the test/caller can verify.
        return (child_pid, 0)

    # =========================================================================
    # exec() — Replace Process Image
    # =========================================================================
    #
    # exec() throws away the current program and loads a new one. It is like
    # erasing a whiteboard and drawing something completely different. The
    # person holding the whiteboard (the PID) is the same, but the content
    # is entirely new.
    #
    # What changes: registers (zeroed), PC (set to entry point), signal
    #   handlers (cleared), memory.
    # What stays: PID, parent_pid, children, priority, cpu_time.

    def exec(
        self,
        pid: int,
        entry_point: int,
        stack_pointer: int,
        memory_base: int = 0,
        memory_size: int = 0,
    ) -> bool:
        """Exec: replace a process's program with a new one.

        The process keeps its PID, parent, children, and priority, but
        everything else is replaced:
            - Registers are zeroed.
            - PC is set to entry_point.
            - SP is set to stack_pointer.
            - Signal handlers are cleared (the new program doesn't know
              about the old program's handlers).
            - Pending signals are cleared.
            - Memory region is updated if new values are provided.

        Args:
            pid: The PID of the process to exec.
            entry_point: The virtual address where the new program starts.
            stack_pointer: The initial stack pointer for the new program.
            memory_base: New memory base address (0 = keep current).
            memory_size: New memory region size (0 = keep current).

        Returns:
            True if exec succeeded, False if the PID does not exist.

        Example:
            >>> pm = ProcessManager()
            >>> proc = pm.create_process(name="shell")
            >>> pm.exec(proc.pid, entry_point=0x10000, stack_pointer=0x7FFFF000)
            True
            >>> proc.pc
            65536
            >>> proc.registers  # all zeroed
            [0, 0, 0, 0, ..., 0]
        """
        pcb = self._processes.get(pid)
        if pcb is None:
            return False

        # Reset all registers to zero — the new program starts fresh.
        pcb.registers = [0] * 32

        # Set the program counter to the new program's entry point.
        pcb.pc = entry_point

        # Set the stack pointer.
        pcb.sp = stack_pointer

        # Update memory region if new values provided.
        if memory_base != 0:
            pcb.memory_base = memory_base
        if memory_size != 0:
            pcb.memory_size = memory_size

        # Clear signal handlers — the new program has no knowledge of
        # the old program's signal handling setup.
        pcb.signal_handlers.clear()

        # Clear pending signals — the new program starts with a clean
        # signal state.
        pcb.pending_signals.clear()

        return True

    # =========================================================================
    # wait() — Wait for a Child to Exit
    # =========================================================================
    #
    # wait() is how a parent process collects the exit status of its children.
    # Without wait(), terminated child processes become "zombies" — they are
    # dead (no longer running) but their PCBs remain in the process table,
    # consuming resources.
    #
    # The wait() call:
    #   1. Checks if any child is in ZOMBIE state.
    #   2. If yes: reaps the zombie (removes its PCB) and returns its exit code.
    #   3. If no: returns None (in a real kernel, the parent would block).

    def wait(
        self, parent_pid: int, child_pid: int = -1
    ) -> tuple[int, int] | None:
        """Wait for a child to terminate and retrieve its exit code.

        If child_pid is -1, wait for ANY child that is a zombie.
        If child_pid is a specific PID, wait only for that child.

        When a zombie child is found:
            1. The child's exit code is retrieved.
            2. The child is removed from the parent's children list.
            3. The child's PCB is removed from the process table (reaped).

        Args:
            parent_pid: The PID of the waiting parent.
            child_pid: The PID of the specific child to wait for, or -1
                       to wait for any child.

        Returns:
            A tuple (reaped_child_pid, exit_code) if a zombie child was
            found and reaped, or None if no zombie children are available.

        Example:
            >>> pm = ProcessManager()
            >>> parent = pm.create_process(name="shell")
            >>> child_pid, _ = pm.fork(parent.pid)
            >>> pm.exit_process(child_pid, exit_code=42)
            >>> result = pm.wait(parent.pid, child_pid)
            >>> result
            (1, 42)
        """
        parent = self._processes.get(parent_pid)
        if parent is None:
            return None

        # Search the parent's children for a zombie.
        for c_pid in list(parent.children):  # Copy list to allow mutation.
            # If we are waiting for a specific child, skip others.
            if child_pid >= 0 and c_pid != child_pid:
                continue

            child = self._processes.get(c_pid)
            if child is None:
                continue

            if child.state == ProcessState.ZOMBIE:
                # Found a zombie child — reap it!
                exit_code = child.exit_code

                # Remove child from parent's children list.
                parent.children.remove(c_pid)

                # Remove child from the process table entirely.
                del self._processes[c_pid]

                return (c_pid, exit_code)

        # No zombie children found. In a real kernel, the parent would be
        # blocked (state = BLOCKED) until a child exits. For our educational
        # implementation, we return None to indicate "nothing to reap yet."
        return None

    # =========================================================================
    # kill() — Send a Signal to a Process
    # =========================================================================
    #
    # Despite its name, kill() does not necessarily kill a process. It sends
    # a signal, which the process may catch, ignore, or be terminated by.
    # The name comes from its original purpose: sending the default SIGTERM
    # signal, which terminates the process. But you can send any signal.

    def kill(self, pid: int, signal: Signal) -> bool:
        """Send a signal to a process.

        This delegates to the SignalManager, which handles the actual signal
        delivery semantics (immediate action for SIGKILL/SIGSTOP, pending
        queue for everything else).

        Args:
            pid: The PID of the target process.
            signal: The signal to send.

        Returns:
            True if the signal was delivered, False if the PID does not exist.

        Example:
            >>> pm = ProcessManager()
            >>> proc = pm.create_process(name="daemon")
            >>> pm.kill(proc.pid, Signal.SIGTERM)
            True
            >>> proc.pending_signals
            [15]
        """
        process = self._processes.get(pid)
        if process is None:
            return False

        return self._signal_manager.send_signal(process, signal)

    # =========================================================================
    # exit_process() — Terminate a Process
    # =========================================================================
    #
    # When a process calls exit(), or when it reaches the end of its main()
    # function, the kernel:
    #   1. Sets the process state to ZOMBIE.
    #   2. Records the exit code.
    #   3. Reparents any children to PID 0 (init).
    #   4. Sends SIGCHLD to the parent so it knows a child has exited.
    #
    # The process is NOT immediately removed from the process table. It stays
    # as a zombie until the parent calls wait() to retrieve the exit code.
    # This is why you sometimes see zombie processes in `ps` output — their
    # parents haven't waited on them yet.

    def exit_process(self, pid: int, exit_code: int = 0) -> None:
        """Terminate a process.

        Sets the process state to ZOMBIE and records the exit code. The
        process's children are reparented to PID 0 (init process), and
        SIGCHLD is sent to the parent to notify it.

        The PCB remains in the process table until the parent calls wait()
        to reap it.

        Args:
            pid: The PID of the process to terminate.
            exit_code: The exit status code. Convention:
                       0 = success, nonzero = error.

        Example:
            >>> pm = ProcessManager()
            >>> parent = pm.create_process(name="shell")
            >>> child_pid, _ = pm.fork(parent.pid)
            >>> pm.exit_process(child_pid, exit_code=0)
            >>> child = pm.get_process(child_pid)
            >>> child.state
            <ProcessState.ZOMBIE: 4>
        """
        pcb = self._processes.get(pid)
        if pcb is None:
            return

        # Set state to ZOMBIE — the process is dead but not yet reaped.
        pcb.state = ProcessState.ZOMBIE
        pcb.exit_code = exit_code

        # Reparent children to PID 0 (init).
        # In a real system, init periodically calls wait() to reap orphaned
        # zombies. Without reparenting, orphaned children would never be
        # reaped and their PCBs would leak.
        init_process = self._processes.get(0)
        for child_pid_item in pcb.children:
            child = self._processes.get(child_pid_item)
            if child is not None:
                child.parent_pid = 0
                if init_process is not None and pid != 0:
                    init_process.children.append(child_pid_item)

        pcb.children.clear()

        # Send SIGCHLD to the parent so it knows a child has exited.
        # The parent can then call wait() to retrieve the exit code.
        if pcb.parent_pid >= 0:
            parent = self._processes.get(pcb.parent_pid)
            if parent is not None:
                self._signal_manager.send_signal(parent, Signal.SIGCHLD)

    # =========================================================================
    # Query Methods
    # =========================================================================

    def get_process(self, pid: int) -> ProcessControlBlock | None:
        """Get a process by PID.

        Args:
            pid: The PID to look up.

        Returns:
            The ProcessControlBlock, or None if the PID does not exist.
        """
        return self._processes.get(pid)

    def get_children(self, pid: int) -> list[int]:
        """Get the PIDs of all children of a process.

        Args:
            pid: The parent PID.

        Returns:
            List of child PIDs, or empty list if PID not found.
        """
        pcb = self._processes.get(pid)
        if pcb is None:
            return []
        return list(pcb.children)

    def get_parent(self, pid: int) -> int:
        """Get the parent PID of a process.

        Args:
            pid: The PID to query.

        Returns:
            The parent PID, or -1 if the PID does not exist.
        """
        pcb = self._processes.get(pid)
        if pcb is None:
            return -1
        return pcb.parent_pid

    @property
    def signal_manager(self) -> SignalManager:
        """Access the signal manager for direct signal operations."""
        return self._signal_manager

    @property
    def process_count(self) -> int:
        """Return the number of processes in the process table."""
        return len(self._processes)
