"""Kernel -- the central component of the operating system.

Manages processes, handles system calls, and coordinates scheduling.
The kernel operates at the Python level -- syscall handlers, the scheduler,
and memory management are Python functions.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

from os_kernel.memory_manager import (
    PERM_EXECUTE,
    PERM_READ,
    PERM_WRITE,
    MemoryManager,
    MemoryRegion,
)
from os_kernel.process import (
    ProcessControlBlock,
    ProcessInfo,
    ProcessState,
)
from os_kernel.programs import generate_hello_world_program, generate_idle_program
from os_kernel.scheduler import Scheduler
from os_kernel.syscall import (
    MemoryAccess,
    RegisterAccess,
    SyscallHandler,
    REG_SP,
    default_syscall_table,
)

if TYPE_CHECKING:
    from display import DisplayDriver
    from interrupt_handler import InterruptController, InterruptFrame

# Well-known addresses
DEFAULT_KERNEL_BASE: int = 0x00020000
DEFAULT_KERNEL_SIZE: int = 0x00010000
DEFAULT_IDLE_PROCESS_BASE: int = 0x00030000
DEFAULT_IDLE_PROCESS_SIZE: int = 0x00010000
DEFAULT_USER_PROCESS_BASE: int = 0x00040000
DEFAULT_USER_PROCESS_SIZE: int = 0x00010000
DEFAULT_KERNEL_STACK_TOP: int = 0x0006FFF0
DEFAULT_KERNEL_STACK_BASE: int = 0x00060000
DEFAULT_KERNEL_STACK_SIZE: int = 0x00010000

# Interrupt numbers
INTERRUPT_TIMER: int = 32
INTERRUPT_KEYBOARD: int = 33
INTERRUPT_SYSCALL: int = 128


@dataclass
class KernelConfig:
    """Tunable parameters for the kernel."""

    timer_interval: int = 100
    max_processes: int = 16
    memory_layout: list[MemoryRegion] = field(default_factory=list)


def DefaultKernelConfig() -> KernelConfig:  # noqa: ANN201, N802
    """Return a configuration suitable for the hello-world demo."""
    return KernelConfig(
        timer_interval=100,
        max_processes=16,
        memory_layout=[
            MemoryRegion(base=0x00000000, size=0x00001000, permissions=PERM_READ, owner=-1, name="IDT"),
            MemoryRegion(base=0x00001000, size=0x00001000, permissions=PERM_READ | PERM_WRITE, owner=-1, name="Boot Protocol"),
            MemoryRegion(base=DEFAULT_KERNEL_BASE, size=DEFAULT_KERNEL_SIZE, permissions=PERM_READ | PERM_WRITE | PERM_EXECUTE, owner=-1, name="Kernel Code"),
            MemoryRegion(base=DEFAULT_IDLE_PROCESS_BASE, size=DEFAULT_IDLE_PROCESS_SIZE, permissions=PERM_READ | PERM_WRITE | PERM_EXECUTE, owner=0, name="Idle Process"),
            MemoryRegion(base=DEFAULT_USER_PROCESS_BASE, size=DEFAULT_USER_PROCESS_SIZE, permissions=PERM_READ | PERM_WRITE | PERM_EXECUTE, owner=1, name="User Process"),
            MemoryRegion(base=DEFAULT_KERNEL_STACK_BASE, size=DEFAULT_KERNEL_STACK_SIZE, permissions=PERM_READ | PERM_WRITE, owner=-1, name="Kernel Stack"),
        ],
    )


class Kernel:
    """Central component of the operating system."""

    def __init__(
        self: Kernel,
        config: KernelConfig,
        interrupt_ctrl: Any | None = None,
        display_driver: Any | None = None,
    ) -> None:
        self.config = config
        self.interrupt_ctrl = interrupt_ctrl
        self.display = display_driver
        self.syscall_table: dict[int, SyscallHandler] = default_syscall_table()
        self.process_table: list[ProcessControlBlock] = []
        self.current_process: int = 0
        self.scheduler: Scheduler | None = None
        self.memory_manager: MemoryManager | None = None
        self.keyboard_buffer: list[int] = []
        self.booted: bool = False
        self._next_pid: int = 0

    def boot(self: Kernel) -> None:
        """Initialize all subsystems and start the scheduler.

        Boot sequence:
          1. Initialize memory manager
          2. Register ISRs with the interrupt controller
          3. Create idle process (PID 0)
          4. Create hello-world process (PID 1)
          5. Start scheduler with PID 1 as first running process
        """
        self.memory_manager = MemoryManager(self.config.memory_layout)

        if self.interrupt_ctrl is not None:
            self.interrupt_ctrl.registry.register(
                INTERRUPT_TIMER, lambda frame, kernel: self.handle_timer(frame))
            self.interrupt_ctrl.registry.register(
                INTERRUPT_KEYBOARD, lambda frame, kernel: self.handle_keyboard(frame))
            self.interrupt_ctrl.registry.register(
                INTERRUPT_SYSCALL, lambda frame, kernel: self.handle_syscall_frame(frame))

        idle_binary = generate_idle_program()
        self.create_process("idle", idle_binary, DEFAULT_IDLE_PROCESS_BASE, DEFAULT_IDLE_PROCESS_SIZE)

        hw_binary = generate_hello_world_program(DEFAULT_USER_PROCESS_BASE)
        self.create_process("hello-world", hw_binary, DEFAULT_USER_PROCESS_BASE, DEFAULT_USER_PROCESS_SIZE)

        self.scheduler = Scheduler(self.process_table)

        if len(self.process_table) > 1:
            self.process_table[1].state = ProcessState.RUNNING
            self.current_process = 1
            self.scheduler.current = 1

        self.booted = True

    def create_process(
        self: Kernel, name: str, binary: bytes, mem_base: int, mem_size: int
    ) -> int:
        """Create a new process. Returns the PID or -1 if table is full."""
        if len(self.process_table) >= self.config.max_processes:
            return -1

        pid = self._next_pid
        self._next_pid += 1

        pcb = ProcessControlBlock(
            pid=pid,
            state=ProcessState.READY,
            saved_pc=mem_base,
            stack_pointer=mem_base + mem_size - 16,
            memory_base=mem_base,
            memory_size=mem_size,
            name=name,
        )
        pcb.saved_registers[REG_SP] = pcb.stack_pointer

        self.process_table.append(pcb)
        return pid

    def handle_syscall(
        self: Kernel, syscall_num: int, regs: RegisterAccess, mem: MemoryAccess
    ) -> bool:
        """Dispatch a syscall based on the a7 register value."""
        handler = self.syscall_table.get(syscall_num)
        if handler is None:
            pid = self.current_process
            if 0 <= pid < len(self.process_table):
                self.process_table[pid].state = ProcessState.TERMINATED
                self.process_table[pid].exit_code = -1
            return False
        return handler(self, regs, mem)

    def handle_syscall_frame(self: Kernel, frame: Any) -> None:
        """ISR handler for interrupt 128 (ecall). Simplified."""

    def handle_timer(self: Kernel, frame: Any) -> None:
        """ISR for interrupt 32 (timer tick)."""
        if self.scheduler is None:
            return
        pid = self.current_process
        if 0 <= pid < len(self.process_table):
            pcb = self.process_table[pid]
            if pcb.state == ProcessState.RUNNING:
                pcb.state = ProcessState.READY
                pcb.saved_registers = list(frame.registers)
                pcb.saved_pc = frame.pc

        next_pid = self.scheduler.schedule()
        self.scheduler.context_switch(pid, next_pid)
        self.current_process = next_pid

        if 0 <= next_pid < len(self.process_table):
            next_pcb = self.process_table[next_pid]
            frame.registers = list(next_pcb.saved_registers)
            frame.pc = next_pcb.saved_pc

    def handle_keyboard(self: Kernel, frame: Any) -> None:
        """ISR for interrupt 33 (keyboard). Simplified."""

    def is_idle(self: Kernel) -> bool:
        """Return True when only the idle process remains active."""
        for pcb in self.process_table:
            if pcb.pid == 0:
                continue
            if pcb.state != ProcessState.TERMINATED:
                return False
        return True

    def process_info(self: Kernel, pid: int) -> ProcessInfo:
        """Return a summary of a process."""
        if pid < 0 or pid >= len(self.process_table):
            return ProcessInfo()
        pcb = self.process_table[pid]
        return ProcessInfo(pid=pcb.pid, name=pcb.name, state=pcb.state, pc=pcb.saved_pc)

    def process_count(self: Kernel) -> int:
        """Return the number of processes in the table."""
        return len(self.process_table)

    def get_current_pcb(self: Kernel) -> ProcessControlBlock | None:
        """Return the PCB of the currently running process."""
        if 0 <= self.current_process < len(self.process_table):
            return self.process_table[self.current_process]
        return None

    def add_keystroke(self: Kernel, ch: int) -> None:
        """Append a character to the keyboard buffer."""
        self.keyboard_buffer.append(ch)
