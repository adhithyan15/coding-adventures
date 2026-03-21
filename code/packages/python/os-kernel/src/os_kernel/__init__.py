"""S04 OS Kernel -- minimal monolithic kernel for the simulated computer.

Manages two processes (idle and hello-world), handles system calls, and
coordinates a round-robin scheduler via timer interrupts.

Design Philosophy:
    This kernel operates at the Python level -- syscall handlers, the
    scheduler, and memory management are Python functions. The hello-world
    and idle programs are real RISC-V machine code that triggers ecall
    instructions, which the SystemBoard intercepts and dispatches to the kernel.
"""

from os_kernel.kernel import (
    Kernel,
    KernelConfig,
    DefaultKernelConfig,
)
from os_kernel.process import (
    ProcessControlBlock,
    ProcessInfo,
    ProcessState,
    PROCESS_READY,
    PROCESS_RUNNING,
    PROCESS_BLOCKED,
    PROCESS_TERMINATED,
)
from os_kernel.scheduler import Scheduler
from os_kernel.memory_manager import (
    MemoryManager,
    MemoryRegion,
    PERM_READ,
    PERM_WRITE,
    PERM_EXECUTE,
)
from os_kernel.syscall import (
    RegisterAccess,
    MemoryAccess,
    SyscallHandler,
    SYS_EXIT,
    SYS_WRITE,
    SYS_READ,
    SYS_YIELD,
    REG_A0,
    REG_A1,
    REG_A2,
    REG_A7,
    REG_SP,
)
from os_kernel.programs import (
    generate_idle_program,
    generate_hello_world_program,
    generate_hello_world_binary,
)

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

__all__ = [
    "Kernel",
    "KernelConfig",
    "DefaultKernelConfig",
    "ProcessControlBlock",
    "ProcessInfo",
    "ProcessState",
    "PROCESS_READY",
    "PROCESS_RUNNING",
    "PROCESS_BLOCKED",
    "PROCESS_TERMINATED",
    "Scheduler",
    "MemoryManager",
    "MemoryRegion",
    "PERM_READ",
    "PERM_WRITE",
    "PERM_EXECUTE",
    "RegisterAccess",
    "MemoryAccess",
    "SyscallHandler",
    "SYS_EXIT",
    "SYS_WRITE",
    "SYS_READ",
    "SYS_YIELD",
    "REG_A0",
    "REG_A1",
    "REG_A2",
    "REG_A7",
    "REG_SP",
    "generate_idle_program",
    "generate_hello_world_program",
    "generate_hello_world_binary",
    "DEFAULT_KERNEL_BASE",
    "DEFAULT_KERNEL_SIZE",
    "DEFAULT_IDLE_PROCESS_BASE",
    "DEFAULT_IDLE_PROCESS_SIZE",
    "DEFAULT_USER_PROCESS_BASE",
    "DEFAULT_USER_PROCESS_SIZE",
    "DEFAULT_KERNEL_STACK_TOP",
    "DEFAULT_KERNEL_STACK_BASE",
    "DEFAULT_KERNEL_STACK_SIZE",
    "INTERRUPT_TIMER",
    "INTERRUPT_KEYBOARD",
    "INTERRUPT_SYSCALL",
]
