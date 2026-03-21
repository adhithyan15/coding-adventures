"""System call interface.

System calls are how user programs request services from the kernel.
On RISC-V, the convention is:

    a7 (x17): syscall number
    a0 (x10): first argument / return value
    a1 (x11): second argument
    a2 (x12): third argument
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from os_kernel.kernel import Kernel

# Syscall numbers
SYS_EXIT: int = 0
SYS_WRITE: int = 1
SYS_READ: int = 2
SYS_YIELD: int = 3

# RISC-V register numbers
REG_A0: int = 10
REG_A1: int = 11
REG_A2: int = 12
REG_A7: int = 17
REG_SP: int = 2


class RegisterAccess(Protocol):
    """Read/write access to CPU registers."""

    def read_register(self: RegisterAccess, index: int) -> int: ...
    def write_register(self: RegisterAccess, index: int, value: int) -> None: ...


class MemoryAccess(Protocol):
    """Read access to the CPU's memory."""

    def read_memory_byte(self: MemoryAccess, address: int) -> int: ...


# Type alias for syscall handler functions
from collections.abc import Callable  # noqa: E402

SyscallHandler = Callable[["Kernel", RegisterAccess, MemoryAccess], bool]


def _handle_sys_exit(k: Kernel, regs: RegisterAccess, mem: MemoryAccess) -> bool:
    """Terminate the current process. a0 = exit code."""
    exit_code = regs.read_register(REG_A0)
    pid = k.current_process
    if 0 <= pid < len(k.process_table):
        from os_kernel.process import ProcessState
        k.process_table[pid].state = ProcessState.TERMINATED
        k.process_table[pid].exit_code = exit_code
    next_pid = k.scheduler.schedule()
    k.scheduler.context_switch(pid, next_pid)
    k.current_process = next_pid
    return True


def _handle_sys_write(k: Kernel, regs: RegisterAccess, mem: MemoryAccess) -> bool:
    """Write bytes to stdout. a0=fd, a1=buf addr, a2=length."""
    fd = regs.read_register(REG_A0)
    buf_addr = regs.read_register(REG_A1)
    length = regs.read_register(REG_A2)

    if fd != 1:
        regs.write_register(REG_A0, 0)
        return True

    if k.display is None:
        regs.write_register(REG_A0, 0)
        return True

    written = 0
    for i in range(length):
        ch = mem.read_memory_byte(buf_addr + i)
        k.display.put_char(ch)
        written += 1

    regs.write_register(REG_A0, written)
    return True


def _handle_sys_read(k: Kernel, regs: RegisterAccess, mem: MemoryAccess) -> bool:
    """Read bytes from stdin. a0=fd, a1=buf addr, a2=max length."""
    fd = regs.read_register(REG_A0)
    length = regs.read_register(REG_A2)

    if fd != 0:
        regs.write_register(REG_A0, 0)
        return True

    available = len(k.keyboard_buffer)
    to_read = min(length, available)
    regs.write_register(REG_A0, to_read)

    if to_read > 0:
        k.keyboard_buffer = k.keyboard_buffer[to_read:]

    return True


def _handle_sys_yield(k: Kernel, regs: RegisterAccess, mem: MemoryAccess) -> bool:
    """Voluntarily give up the CPU."""
    from os_kernel.process import ProcessState
    pid = k.current_process
    if 0 <= pid < len(k.process_table):
        if k.process_table[pid].state == ProcessState.RUNNING:
            k.process_table[pid].state = ProcessState.READY
    next_pid = k.scheduler.schedule()
    k.scheduler.context_switch(pid, next_pid)
    k.current_process = next_pid
    return True


def default_syscall_table() -> dict[int, SyscallHandler]:
    """Return the standard syscall dispatch table."""
    return {
        SYS_EXIT: _handle_sys_exit,
        SYS_WRITE: _handle_sys_write,
        SYS_READ: _handle_sys_read,
        SYS_YIELD: _handle_sys_yield,
    }
