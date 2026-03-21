"""Tests for the S04 OS Kernel package."""

from os_kernel import (
    DefaultKernelConfig,
    Kernel,
    KernelConfig,
    MemoryManager,
    MemoryRegion,
    PERM_EXECUTE,
    PERM_READ,
    PERM_WRITE,
    ProcessControlBlock,
    ProcessState,
    PROCESS_BLOCKED,
    PROCESS_READY,
    PROCESS_RUNNING,
    PROCESS_TERMINATED,
    REG_A0,
    REG_A2,
    REG_SP,
    Scheduler,
    SYS_EXIT,
    SYS_READ,
    SYS_WRITE,
    SYS_YIELD,
    generate_hello_world_binary,
    generate_hello_world_program,
    generate_idle_program,
)
from display import DisplayDriver, DisplayConfig, BYTES_PER_CELL
from interrupt_handler import InterruptController, InterruptFrame


# =========================================================================
# Mock accessors
# =========================================================================


class MockRegAccess:
    def __init__(self) -> None:
        self.regs: dict[int, int] = {}

    def read_register(self, index: int) -> int:
        return self.regs.get(index, 0)

    def write_register(self, index: int, value: int) -> None:
        self.regs[index] = value


class MockMemAccess:
    def __init__(self, data: dict[int, int] | None = None) -> None:
        self.data: dict[int, int] = data or {}

    def read_memory_byte(self, address: int) -> int:
        return self.data.get(address, 0)


# =========================================================================
# Helpers
# =========================================================================


def _new_test_kernel() -> Kernel:
    return Kernel(DefaultKernelConfig())


def _new_booted_kernel() -> Kernel:
    ic = InterruptController()
    k = Kernel(DefaultKernelConfig(), ic)
    k.boot()
    return k


def _new_booted_kernel_with_display(driver: DisplayDriver) -> Kernel:
    ic = InterruptController()
    k = Kernel(DefaultKernelConfig(), ic, driver)
    k.boot()
    return k


# =========================================================================
# Process Management Tests
# =========================================================================


class TestProcessManagement:
    def test_create_process(self) -> None:
        k = _new_test_kernel()
        pid = k.create_process("test", b"\x01\x02\x03\x04", 0x00040000, 0x10000)
        assert pid == 0
        assert len(k.process_table) == 1
        pcb = k.process_table[0]
        assert pcb.name == "test"
        assert pcb.state == PROCESS_READY
        assert pcb.memory_base == 0x00040000
        assert pcb.saved_pc == 0x00040000

    def test_multiple_processes(self) -> None:
        k = _new_test_kernel()
        pid0 = k.create_process("p0", b"", 0x30000, 0x10000)
        pid1 = k.create_process("p1", b"", 0x40000, 0x10000)
        assert pid0 != pid1
        assert k.process_count() == 2

    def test_max_processes(self) -> None:
        config = DefaultKernelConfig()
        config.max_processes = 2
        k = Kernel(config)
        k.create_process("p0", b"", 0x30000, 0x10000)
        k.create_process("p1", b"", 0x40000, 0x10000)
        pid = k.create_process("p2", b"", 0x50000, 0x10000)
        assert pid == -1

    def test_process_state_string(self) -> None:
        assert str(PROCESS_READY) == "Ready"
        assert str(PROCESS_RUNNING) == "Running"
        assert str(PROCESS_BLOCKED) == "Blocked"
        assert str(PROCESS_TERMINATED) == "Terminated"


# =========================================================================
# Scheduler Tests
# =========================================================================


class TestScheduler:
    def test_round_robin(self) -> None:
        procs = [
            ProcessControlBlock(pid=0, state=PROCESS_READY, name="idle"),
            ProcessControlBlock(pid=1, state=PROCESS_READY, name="hello"),
        ]
        sched = Scheduler(procs)
        sched.current = 0
        assert sched.schedule() == 1
        sched.current = 1
        assert sched.schedule() == 0

    def test_skip_terminated(self) -> None:
        procs = [
            ProcessControlBlock(pid=0, state=PROCESS_READY, name="idle"),
            ProcessControlBlock(pid=1, state=PROCESS_TERMINATED, name="hello"),
        ]
        sched = Scheduler(procs)
        sched.current = 0
        assert sched.schedule() == 0

    def test_context_switch(self) -> None:
        procs = [
            ProcessControlBlock(pid=0, state=PROCESS_RUNNING, name="idle"),
            ProcessControlBlock(pid=1, state=PROCESS_READY, name="hello"),
        ]
        sched = Scheduler(procs)
        sched.context_switch(0, 1)
        assert procs[0].state == PROCESS_READY
        assert procs[1].state == PROCESS_RUNNING
        assert sched.current == 1

    def test_empty_table(self) -> None:
        sched = Scheduler([])
        assert sched.schedule() == 0


# =========================================================================
# Memory Manager Tests
# =========================================================================


class TestMemoryManager:
    def test_find_region(self) -> None:
        regions = [
            MemoryRegion(base=0x1000, size=0x1000, permissions=PERM_READ, owner=-1, name="A"),
            MemoryRegion(base=0x3000, size=0x2000, permissions=PERM_READ | PERM_WRITE, owner=1, name="B"),
        ]
        mm = MemoryManager(regions)
        r = mm.find_region(0x1500)
        assert r is not None and r.name == "A"
        r = mm.find_region(0x4000)
        assert r is not None and r.name == "B"
        assert mm.find_region(0x6000) is None

    def test_check_access(self) -> None:
        regions = [
            MemoryRegion(base=0x1000, size=0x1000, permissions=PERM_READ | PERM_WRITE, owner=-1, name="Kernel"),
            MemoryRegion(base=0x3000, size=0x1000, permissions=PERM_READ | PERM_WRITE | PERM_EXECUTE, owner=1, name="P1"),
        ]
        mm = MemoryManager(regions)
        assert mm.check_access(0, 0x1000, PERM_READ)
        assert mm.check_access(1, 0x3000, PERM_READ | PERM_WRITE)
        assert not mm.check_access(0, 0x3000, PERM_READ)
        assert not mm.check_access(0, 0x9000, PERM_READ)

    def test_allocate_region(self) -> None:
        mm = MemoryManager()
        assert mm.region_count() == 0
        mm.allocate_region(1, 0x5000, 0x1000, PERM_READ | PERM_WRITE, "new")
        assert mm.region_count() == 1
        r = mm.find_region(0x5500)
        assert r is not None and r.name == "new"


# =========================================================================
# Syscall Tests
# =========================================================================


class TestSyscalls:
    def test_sys_exit(self) -> None:
        k = _new_booted_kernel()
        regs = MockRegAccess()
        regs.regs[REG_A0] = 42
        k.current_process = 1
        k.process_table[1].state = PROCESS_RUNNING
        k.handle_syscall(SYS_EXIT, regs, MockMemAccess())
        assert k.process_table[1].state == PROCESS_TERMINATED
        assert k.process_table[1].exit_code == 42

    def test_sys_write(self) -> None:
        config = DisplayConfig()
        display_mem = bytearray(config.columns * config.rows * BYTES_PER_CELL)
        driver = DisplayDriver(config, display_mem)
        k = _new_booted_kernel_with_display(driver)
        mem = MockMemAccess(data={0x00040100: ord("H"), 0x00040101: ord("i")})
        regs = MockRegAccess()
        regs.regs[REG_A0] = 1
        regs.regs[11] = 0x00040100
        regs.regs[12] = 2
        k.current_process = 1
        k.handle_syscall(SYS_WRITE, regs, mem)
        assert regs.regs[REG_A0] == 2
        snap = driver.snapshot()
        assert snap.contains("Hi")

    def test_sys_write_wrong_fd(self) -> None:
        k = _new_booted_kernel()
        regs = MockRegAccess()
        regs.regs[REG_A0] = 2
        regs.regs[11] = 0x40100
        regs.regs[12] = 5
        k.current_process = 1
        k.handle_syscall(SYS_WRITE, regs, MockMemAccess())
        assert regs.regs[REG_A0] == 0

    def test_sys_read(self) -> None:
        k = _new_booted_kernel()
        k.keyboard_buffer = [ord("A"), ord("B")]
        regs = MockRegAccess()
        regs.regs[REG_A0] = 0
        regs.regs[REG_A2] = 10
        k.current_process = 1
        k.handle_syscall(SYS_READ, regs, MockMemAccess())
        assert regs.regs[REG_A0] == 2
        assert len(k.keyboard_buffer) == 0

    def test_sys_read_empty(self) -> None:
        k = _new_booted_kernel()
        regs = MockRegAccess()
        regs.regs[REG_A0] = 0
        regs.regs[REG_A2] = 10
        k.current_process = 1
        k.handle_syscall(SYS_READ, regs, MockMemAccess())
        assert regs.regs[REG_A0] == 0

    def test_sys_yield(self) -> None:
        k = _new_booted_kernel()
        k.current_process = 1
        k.process_table[1].state = PROCESS_RUNNING
        regs = MockRegAccess()
        k.handle_syscall(SYS_YIELD, regs, MockMemAccess())
        assert k.process_table[1].state == PROCESS_READY

    def test_unknown_syscall(self) -> None:
        k = _new_booted_kernel()
        k.current_process = 1
        k.process_table[1].state = PROCESS_RUNNING
        regs = MockRegAccess()
        ok = k.handle_syscall(99, regs, MockMemAccess())
        assert not ok
        assert k.process_table[1].state == PROCESS_TERMINATED


# =========================================================================
# Kernel Boot Tests
# =========================================================================


class TestKernelBoot:
    def test_boot(self) -> None:
        ic = InterruptController()
        config = DisplayConfig()
        display_mem = bytearray(config.columns * config.rows * BYTES_PER_CELL)
        driver = DisplayDriver(config, display_mem)
        k = Kernel(DefaultKernelConfig(), ic, driver)
        k.boot()
        assert k.booted
        assert k.process_count() == 2
        assert k.process_table[0].name == "idle"
        assert k.process_table[1].name == "hello-world"
        assert k.current_process == 1
        assert k.process_table[1].state == PROCESS_RUNNING
        assert ic.registry.has_handler(32)
        assert ic.registry.has_handler(33)
        assert ic.registry.has_handler(128)

    def test_is_idle(self) -> None:
        k = _new_booted_kernel()
        assert not k.is_idle()
        k.process_table[1].state = PROCESS_TERMINATED
        assert k.is_idle()

    def test_process_info(self) -> None:
        k = _new_booted_kernel()
        info = k.process_info(1)
        assert info.pid == 1
        assert info.name == "hello-world"

    def test_add_keystroke(self) -> None:
        k = _new_booted_kernel()
        k.add_keystroke(ord("H"))
        k.add_keystroke(ord("i"))
        assert k.keyboard_buffer == [ord("H"), ord("i")]

    def test_default_kernel_config(self) -> None:
        config = DefaultKernelConfig()
        assert config.timer_interval == 100
        assert config.max_processes == 16
        assert len(config.memory_layout) > 0


# =========================================================================
# Program Generation Tests
# =========================================================================


class TestPrograms:
    def test_idle_program(self) -> None:
        binary = generate_idle_program()
        assert len(binary) > 0
        assert len(binary) % 4 == 0

    def test_hello_world_program(self) -> None:
        binary = generate_hello_world_program(0x00040000)
        assert len(binary) > 0
        message = b"Hello World\n"
        found = binary[0x100:0x100 + len(message)]
        assert found == message

    def test_hello_world_binary(self) -> None:
        binary = generate_hello_world_binary()
        assert len(binary) > 0
