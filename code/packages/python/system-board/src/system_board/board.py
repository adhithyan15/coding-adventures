"""SystemBoard -- the complete simulated computer.

Composes all hardware and software components into a working computer.
Uses a simple RISC-V simulator to execute instructions. After each
instruction, checks for ecall traps and dispatches to the kernel.
"""

from __future__ import annotations

from bootloader import (
    BOOT_PROTOCOL_MAGIC,
    Bootloader,
    DiskImage,
)
from display import BYTES_PER_CELL, DisplayDriver
from interrupt_handler import InterruptController
from os_kernel import (
    DEFAULT_IDLE_PROCESS_BASE,
    DEFAULT_USER_PROCESS_BASE,
    INTERRUPT_KEYBOARD,
    Kernel,
    ProcessState,
    REG_A7,
    REG_SP,
    SYS_EXIT,
    SYS_WRITE,
    SYS_YIELD,
    generate_hello_world_program,
    generate_idle_program,
)
from riscv_simulator import RiscVSimulator
from riscv_simulator.csr import CSR_MEPC, CSR_MSTATUS, CSR_MTVEC, MIE
from rom_bios import BIOSFirmware, HardwareInfo, ROM, DefaultROMConfig

from system_board.boot_trace import BootPhase, BootTrace
from system_board.config import (
    BOOT_PROTOCOL_ADDR,
    BOOTLOADER_BASE,
    DISK_MAPPED_BASE,
    KERNEL_BASE,
    USER_PROCESS_BASE,
    SystemConfig,
)


class SystemBoard:
    """The complete simulated computer."""

    def __init__(self: SystemBoard, config: SystemConfig) -> None:
        self.config = config
        self.cpu: RiscVSimulator | None = None
        self.rom: ROM | None = None
        self.disk_image: DiskImage | None = None
        self.display: DisplayDriver | None = None
        self.interrupt_ctrl: InterruptController | None = None
        self.kernel: Kernel | None = None
        self.trace = BootTrace()
        self.powered = False
        self.cycle = 0
        self.current_phase = BootPhase.POWER_ON
        self._kernel_booted = False
        self._previous_pc = 0

    def power_on(self: SystemBoard) -> None:
        """Initialize all components and begin the boot sequence."""
        if self.powered:
            return

        config = self.config

        # 1. Create CPU
        mem_size = 0x10200000
        self.cpu = RiscVSimulator(memory_size=mem_size)

        # 2. Create Interrupt Controller
        self.interrupt_ctrl = InterruptController()

        # 3. Create Display
        display_mem = bytearray(
            config.display_config.columns * config.display_config.rows * BYTES_PER_CELL
        )
        self.display = DisplayDriver(config.display_config, display_mem)

        # 4. Generate BIOS firmware
        bios_firmware = BIOSFirmware(config.bios_config)
        bios_bytes = bios_firmware.generate()
        self.rom = ROM(DefaultROMConfig(), bios_bytes)

        # 5. Create Disk Image
        self.disk_image = DiskImage()

        # 6. Create Kernel
        self.kernel = Kernel(config.kernel_config, self.interrupt_ctrl, self.display)

        # 7. Generate and prepare binaries
        if config.user_program is not None:
            user_program = config.user_program
        else:
            user_program = generate_hello_world_program(USER_PROCESS_BASE)

        idle_binary = generate_idle_program()
        kernel_stub_size = 16
        total_size = kernel_stub_size + len(idle_binary) + len(user_program)
        if total_size % 4 != 0:
            total_size += 4 - (total_size % 4)

        bl_config = config.bootloader_config
        bl_config.kernel_size = total_size
        bl = Bootloader(bl_config)
        bootloader_code = bl.generate()

        # 8. Pre-load everything into memory
        # Write boot protocol
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 0, BOOT_PROTOCOL_MAGIC)
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 4, config.memory_size)
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 8, bl_config.kernel_disk_offset)
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 12, bl_config.kernel_size)
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 16, bl_config.kernel_load_address)
        _write_word(self.cpu, BOOT_PROTOCOL_ADDR + 20, bl_config.stack_base)

        # Load bootloader code
        for i, b in enumerate(bootloader_code):
            self.cpu.cpu.memory.write_byte(BOOTLOADER_BASE + i, b)

        # Build kernel disk data
        kernel_disk_data = bytearray(total_size)
        kernel_disk_data[kernel_stub_size:kernel_stub_size + len(idle_binary)] = idle_binary
        kernel_disk_data[kernel_stub_size + len(idle_binary):kernel_stub_size + len(idle_binary) + len(user_program)] = user_program

        self.disk_image.load_kernel(bytes(kernel_disk_data))

        # Memory-map disk
        disk_data = self.disk_image.data()
        for i in range(len(disk_data)):
            addr = DISK_MAPPED_BASE + i
            if addr < mem_size:
                self.cpu.cpu.memory.write_byte(addr, disk_data[i])

        # Pre-load binaries at final locations
        for i, b in enumerate(idle_binary):
            self.cpu.cpu.memory.write_byte(DEFAULT_IDLE_PROCESS_BASE + i, b)
        for i, b in enumerate(user_program):
            self.cpu.cpu.memory.write_byte(DEFAULT_USER_PROCESS_BASE + i, b)

        # Set PC to bootloader entry
        self.cpu.cpu.pc = BOOTLOADER_BASE

        # Configure CSR for trap handling
        self.cpu.csr.write(CSR_MTVEC, 0xDEAD0000)

        self.powered = True
        self.current_phase = BootPhase.POWER_ON
        self.trace.add_event(BootPhase.POWER_ON, 0, "System powered on")
        self.trace.add_event(BootPhase.BIOS, 0, "BIOS phase simulated (hardware info written to boot protocol)")
        self.current_phase = BootPhase.BIOS

    def step(self: SystemBoard) -> None:
        """Execute one CPU cycle and check for traps/phase transitions."""
        if not self.powered:
            return

        self._previous_pc = self.cpu.cpu.pc
        self.cycle += 1
        self.cpu.step()
        self._detect_phase_transition()
        self._handle_trap()

    def run(self: SystemBoard, max_cycles: int) -> BootTrace:
        """Execute until idle or max_cycles exhausted."""
        if not self.powered:
            return self.trace

        for _ in range(max_cycles):
            self.step()

            if self._kernel_booted and self.kernel.is_idle():
                if self.current_phase != BootPhase.IDLE:
                    self.current_phase = BootPhase.IDLE
                    self.trace.add_event(
                        BootPhase.IDLE, self.cycle,
                        "System idle -- all user programs terminated")
                break

            if self.cpu.cpu.halted:
                break

        return self.trace

    def inject_keystroke(self: SystemBoard, char: int) -> None:
        """Simulate a keyboard press."""
        if self.kernel is not None:
            self.kernel.add_keystroke(char)
        if self.interrupt_ctrl is not None:
            self.interrupt_ctrl.raise_interrupt(INTERRUPT_KEYBOARD)

    def display_snapshot(self: SystemBoard):  # noqa: ANN201
        """Return the current state of the text display."""
        if self.display is None:
            return None
        return self.display.snapshot()

    def get_boot_trace(self: SystemBoard) -> BootTrace:
        """Return the accumulated boot trace."""
        return self.trace

    def is_idle(self: SystemBoard) -> bool:
        """Return True when only the idle process remains."""
        return self._kernel_booted and self.kernel is not None and self.kernel.is_idle()

    def get_cycle_count(self: SystemBoard) -> int:
        """Return total CPU cycles since PowerOn."""
        return self.cycle

    def get_current_phase(self: SystemBoard) -> BootPhase:
        """Return the current boot phase."""
        return self.current_phase

    # =====================================================================
    # Internal: Phase detection
    # =====================================================================

    def _detect_phase_transition(self: SystemBoard) -> None:
        pc = self.cpu.cpu.pc

        if self.current_phase == BootPhase.BIOS:
            if BOOTLOADER_BASE <= pc < BOOTLOADER_BASE + 0x10000:
                self.current_phase = BootPhase.BOOTLOADER
                self.trace.add_event(
                    BootPhase.BOOTLOADER, self.cycle,
                    "Bootloader executing: copying kernel from disk to RAM")

        elif self.current_phase == BootPhase.BOOTLOADER:
            if KERNEL_BASE <= pc < KERNEL_BASE + 0x10000:
                self.current_phase = BootPhase.KERNEL_INIT
                self.trace.add_event(
                    BootPhase.KERNEL_INIT, self.cycle,
                    "Kernel entry reached: initializing subsystems")
                self._initialize_kernel()

        elif self.current_phase == BootPhase.KERNEL_INIT:
            if USER_PROCESS_BASE <= pc < USER_PROCESS_BASE + 0x10000:
                self.current_phase = BootPhase.USER_PROGRAM
                self.trace.add_event(
                    BootPhase.USER_PROGRAM, self.cycle,
                    "User program (hello-world) executing")

    def _initialize_kernel(self: SystemBoard) -> None:
        if self._kernel_booted:
            return

        self.kernel.boot()
        self._kernel_booted = True

        self.trace.add_event(
            BootPhase.KERNEL_INIT, self.cycle,
            f"Kernel booted: {self.kernel.process_count()} processes created")

        if len(self.kernel.process_table) > 1:
            pcb = self.kernel.process_table[1]
            self.cpu.cpu.pc = pcb.saved_pc
            self.cpu.cpu.registers.write(REG_SP, pcb.stack_pointer)

    # =====================================================================
    # Internal: Trap handling
    # =====================================================================

    def _handle_trap(self: SystemBoard) -> None:
        pc = self.cpu.cpu.pc

        if pc != 0xDEAD0000:
            return

        if not self._kernel_booted:
            mepc = self.cpu.csr.read(CSR_MEPC)
            self.cpu.cpu.pc = mepc + 4
            self.cpu.csr.write(CSR_MSTATUS, self.cpu.csr.read(CSR_MSTATUS) | MIE)
            return

        syscall_num = self.cpu.cpu.registers.read(REG_A7)
        mepc = self.cpu.csr.read(CSR_MEPC)

        reg_access = _CpuRegAccess(self.cpu)
        mem_access = _CpuMemAccess(self.cpu)

        self.kernel.handle_syscall(syscall_num, reg_access, mem_access)

        current_pcb = self.kernel.get_current_pcb()
        if current_pcb is not None:
            if current_pcb.state == ProcessState.RUNNING:
                self.cpu.cpu.pc = mepc + 4
            elif current_pcb.state in (ProcessState.READY, ProcessState.TERMINATED):
                next_pcb = self.kernel.get_current_pcb()
                if next_pcb is not None and next_pcb.state == ProcessState.RUNNING:
                    self.cpu.cpu.pc = next_pcb.saved_pc
                    self.cpu.cpu.registers.write(REG_SP, next_pcb.stack_pointer)
                else:
                    if len(self.kernel.process_table) > 0:
                        idle_pcb = self.kernel.process_table[0]
                        self.cpu.cpu.pc = idle_pcb.saved_pc
        else:
            self.cpu.cpu.pc = mepc + 4

        self.cpu.csr.write(CSR_MSTATUS, self.cpu.csr.read(CSR_MSTATUS) | MIE)

        if syscall_num == SYS_WRITE:
            self.trace.add_event(self.current_phase, self.cycle, "sys_write: bytes written to display")
        elif syscall_num == SYS_EXIT:
            self.trace.add_event(self.current_phase, self.cycle, "sys_exit: process terminated")
        elif syscall_num == SYS_YIELD:
            self.trace.add_event(self.current_phase, self.cycle, "sys_yield: voluntary context switch")


# =========================================================================
# CPU Access Adapters
# =========================================================================


class _CpuRegAccess:
    def __init__(self: _CpuRegAccess, cpu: RiscVSimulator) -> None:
        self._cpu = cpu

    def read_register(self: _CpuRegAccess, index: int) -> int:
        return self._cpu.cpu.registers.read(index)

    def write_register(self: _CpuRegAccess, index: int, value: int) -> None:
        self._cpu.cpu.registers.write(index, value)


class _CpuMemAccess:
    def __init__(self: _CpuMemAccess, cpu: RiscVSimulator) -> None:
        self._cpu = cpu

    def read_memory_byte(self: _CpuMemAccess, address: int) -> int:
        return self._cpu.cpu.memory.read_byte(address)


# =========================================================================
# Helper
# =========================================================================


def _write_word(cpu: RiscVSimulator, address: int, value: int) -> None:
    """Write a 32-bit little-endian word to simulator memory."""
    cpu.cpu.memory.write_byte(address, value & 0xFF)
    cpu.cpu.memory.write_byte(address + 1, (value >> 8) & 0xFF)
    cpu.cpu.memory.write_byte(address + 2, (value >> 16) & 0xFF)
    cpu.cpu.memory.write_byte(address + 3, (value >> 24) & 0xFF)
