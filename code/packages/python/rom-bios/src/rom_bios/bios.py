"""BIOS Firmware Generator -- the first code that runs on power-on.

The BIOS firmware is a RISC-V program that initializes the hardware
and hands off control to the bootloader. It performs four steps:

1. **Memory probe**: Discover RAM size via write/read test patterns
2. **IDT initialization**: Set up 256 interrupt descriptor table entries
3. **HardwareInfo write**: Leave a status report at 0x00001000
4. **Jump to bootloader**: Transfer control to 0x00010000

The firmware is generated programmatically using RISC-V encoding helpers,
avoiding a dependency on the assembler package.

Analogy: The BIOS is like a building manager who arrives first thing
in the morning -- turns on lights, checks rooms, writes a status report,
then unlocks the front door for the tenants (the OS).
"""

from __future__ import annotations

from dataclasses import dataclass

from riscv_simulator.encoding import (
    assemble,
    encode_add,
    encode_addi,
    encode_beq,
    encode_blt,
    encode_bne,
    encode_jal,
    encode_jalr,
    encode_lui,
    encode_lw,
    encode_mret,
    encode_sw,
)

from rom_bios.rom import DEFAULT_ROM_BASE

# === Well-known addresses ===

IDT_BASE: int = 0x00000000
IDT_ENTRY_COUNT: int = 256
IDT_ENTRY_SIZE: int = 8
ISR_STUB_BASE: int = 0x00000800
DEFAULT_FAULT_HANDLER: int = 0x00000800
TIMER_ISR: int = 0x00000808
KEYBOARD_ISR: int = 0x00000810
SYSCALL_ISR: int = 0x00000818
PROBE_START: int = 0x00100000
PROBE_STEP: int = 0x00100000
PROBE_LIMIT: int = 0xFFFB0000
DEFAULT_BOOTLOADER_ENTRY: int = 0x00010000
DEFAULT_FRAMEBUFFER_BASE: int = 0xFFFB0000
HARDWARE_INFO_ADDR: int = 0x00001000


@dataclass
class BIOSConfig:
    """Configuration controlling BIOS firmware behavior.

    Attributes:
        memory_size: RAM size to report (0 = probe at boot).
        display_columns: Text display columns (default: 80).
        display_rows: Text display rows (default: 25).
        framebuffer_base: Framebuffer address (default: 0xFFFB0000).
        bootloader_entry: Where BIOS jumps after init (default: 0x00010000).
    """

    memory_size: int = 0
    display_columns: int = 80
    display_rows: int = 25
    framebuffer_base: int = DEFAULT_FRAMEBUFFER_BASE
    bootloader_entry: int = DEFAULT_BOOTLOADER_ENTRY


def DefaultBIOSConfig() -> BIOSConfig:  # noqa: ANN201
    """Return sensible default BIOS configuration."""
    return BIOSConfig()


@dataclass
class AnnotatedInstruction:
    """A machine code instruction paired with human-readable context.

    Attributes:
        address: Memory address where this instruction lives.
        machine_code: Raw 32-bit RISC-V instruction.
        assembly: Human-readable assembly (e.g., "lui x5, 0x100").
        comment: What this instruction does in the boot sequence.
    """

    address: int
    machine_code: int
    assembly: str
    comment: str


class BIOSFirmware:
    """Generates BIOS firmware as RISC-V machine code.

    Example::

        config = BIOSConfig(memory_size=64*1024*1024)
        bios = BIOSFirmware(config)
        machine_code = bios.generate()

        # For debugging:
        for inst in bios.generate_with_comments():
            print(f"0x{inst.address:08X}  {inst.machine_code:08X}  "
                  f"{inst.assembly:<30s}  ; {inst.comment}")
    """

    def __init__(self: BIOSFirmware, config: BIOSConfig) -> None:
        self.config = config

    def generate(self: BIOSFirmware) -> bytes:
        """Return BIOS firmware as raw RISC-V machine code bytes."""
        annotated = self.generate_with_comments()
        instructions = [a.machine_code for a in annotated]
        return assemble(instructions)

    def generate_with_comments(  # noqa: ANN201
        self: BIOSFirmware,
    ) -> list[AnnotatedInstruction]:
        """Return firmware as annotated instructions."""
        instructions: list[AnnotatedInstruction] = []
        address = DEFAULT_ROM_BASE

        def emit(code: int, asm: str, comment: str) -> None:
            nonlocal address
            instructions.append(AnnotatedInstruction(
                address=address,
                machine_code=code & 0xFFFFFFFF,
                assembly=asm,
                comment=comment,
            ))
            address += 4

        # === Step 1: Memory Probe ===
        if self.config.memory_size > 0:
            upper = (self.config.memory_size >> 12) & 0xFFFFF
            lower = self.config.memory_size & 0xFFF
            emit(encode_lui(8, upper),
                 f"lui x8, 0x{upper:05X}",
                 f"Step 1: Load configured memory size ({self.config.memory_size} bytes)")
            if lower != 0:
                emit(encode_addi(8, 8, _sign_extend_12(lower)),
                     f"addi x8, x8, 0x{lower:03X}",
                     "Step 1: Add lower 12 bits of memory size")
        else:
            emit(encode_lui(5, PROBE_START >> 12),
                 f"lui x5, 0x{PROBE_START >> 12:05X}",
                 "Step 1: x5 = 0x00100000 (probe start at 1 MB)")
            emit(encode_lui(6, 0xDEADC),
                 "lui x6, 0xDEADC",
                 "Step 1: x6 upper = 0xDEADC000 (compensated for sign extension)")
            emit(encode_addi(6, 6, _sign_extend_12(0xEEF)),
                 f"addi x6, x6, {_sign_extend_12(0xEEF)}",
                 "Step 1: x6 = 0xDEADBEEF (test pattern)")
            emit(encode_lui(9, PROBE_LIMIT >> 12),
                 f"lui x9, 0x{PROBE_LIMIT >> 12:05X}",
                 "Step 1: x9 = 0xFFFB0000 (probe limit)")
            emit(encode_lui(10, PROBE_STEP >> 12),
                 f"lui x10, 0x{PROBE_STEP >> 12:05X}",
                 "Step 1: x10 = 0x00100000 (1 MB probe step)")
            emit(encode_sw(6, 5, 0),
                 "sw x6, 0(x5)",
                 "Step 1: Write test pattern to [x5]")
            emit(encode_lw(7, 5, 0),
                 "lw x7, 0(x5)",
                 "Step 1: Read it back into x7")
            emit(encode_bne(6, 7, 12),
                 "bne x6, x7, +12",
                 "Step 1: If mismatch, memory ends here")
            emit(encode_add(5, 5, 10),
                 "add x5, x5, x10",
                 "Step 1: Advance probe address by 1 MB")
            emit(encode_blt(5, 9, -16),
                 "blt x5, x9, -16",
                 "Step 1: Loop back if below probe limit")
            emit(encode_add(8, 5, 0),
                 "add x8, x5, x0",
                 "Step 1: x8 = detected memory size")

        # === Step 2: IDT Initialization ===
        # Write ISR stubs at 0x800
        emit(encode_lui(11, ISR_STUB_BASE >> 12),
             f"lui x11, 0x{ISR_STUB_BASE >> 12:05X}",
             "Step 2a: x11 = ISR stub base (upper bits)")
        if ISR_STUB_BASE & 0xFFF:
            emit(encode_addi(11, 11, ISR_STUB_BASE & 0xFFF),
                 f"addi x11, x11, {ISR_STUB_BASE & 0xFFF}",
                 "Step 2a: Add lower bits of ISR stub base")

        # Write fault handler (jal x0, 0 = infinite loop)
        fault_instr = encode_jal(0, 0)
        upper_f = _li_upper(fault_instr)
        emit(encode_lui(12, upper_f),
             f"lui x12, 0x{upper_f:05X}",
             "Step 2a: Load fault handler instruction upper bits")
        if fault_instr & 0xFFF:
            emit(encode_addi(12, 12, _sign_extend_12(fault_instr & 0xFFF)),
                 f"addi x12, x12, {_sign_extend_12(fault_instr & 0xFFF)}",
                 "Step 2a: Load fault handler instruction lower bits")
        emit(encode_sw(12, 11, 0),
             "sw x12, 0(x11)",
             "Step 2a: Store fault handler at 0x800")
        emit(encode_sw(0, 11, 4),
             "sw x0, 4(x11)",
             "Step 2a: Store NOP at 0x804")

        # Write mret stubs
        mret_instr = encode_mret()
        upper_m = _li_upper(mret_instr)
        emit(encode_lui(12, upper_m),
             f"lui x12, 0x{upper_m:05X}",
             "Step 2a: Load mret instruction upper bits")
        if mret_instr & 0xFFF:
            emit(encode_addi(12, 12, _sign_extend_12(mret_instr & 0xFFF)),
                 f"addi x12, x12, {_sign_extend_12(mret_instr & 0xFFF)}",
                 "Step 2a: Load mret instruction lower bits")
        emit(encode_sw(12, 11, 8),
             "sw x12, 8(x11)",
             "Step 2a: Store timer_isr (mret) at 0x808")
        emit(encode_sw(12, 11, 16),
             "sw x12, 16(x11)",
             "Step 2a: Store keyboard_isr (mret) at 0x810")
        emit(encode_sw(12, 11, 24),
             "sw x12, 24(x11)",
             "Step 2a: Store syscall_isr (mret) at 0x818")

        # Step 2b: Write IDT entries
        emit(encode_addi(13, 0, 0),
             "addi x13, x0, 0",
             "Step 2b: x13 = 0 (IDT base)")
        emit(encode_lui(14, 1),
             "lui x14, 0x00001",
             "Step 2b: x14 = 0x1000")
        emit(encode_addi(14, 14, -2048),
             "addi x14, x14, -2048",
             "Step 2b: x14 = 0x800 (default fault handler)")
        emit(encode_lui(16, 1),
             "lui x16, 0x00001",
             "Step 2b: x16 = 0x1000")
        emit(encode_addi(16, 16, -2048),
             "addi x16, x16, -2048",
             "Step 2b: x16 = 0x800 (IDT end)")
        emit(encode_addi(17, 0, 1),
             "addi x17, x0, 1",
             "Step 2b: x17 = 1 (flags: present)")

        # Special ISR addresses
        emit(encode_lui(18, 1), "lui x18, 0x00001", "Step 2b: x18 = 0x1000")
        emit(encode_addi(18, 18, -2040), "addi x18, x18, -2040",
             "Step 2b: x18 = 0x808 (timer ISR)")
        emit(encode_lui(19, 1), "lui x19, 0x00001", "Step 2b: x19 = 0x1000")
        emit(encode_addi(19, 19, -2032), "addi x19, x19, -2032",
             "Step 2b: x19 = 0x810 (keyboard ISR)")
        emit(encode_lui(20, 1), "lui x20, 0x00001", "Step 2b: x20 = 0x1000")
        emit(encode_addi(20, 20, -2024), "addi x20, x20, -2024",
             "Step 2b: x20 = 0x818 (syscall ISR)")

        # Entry offsets
        emit(encode_addi(21, 0, 256), "addi x21, x0, 256",
             "Step 2b: x21 = 256 (entry 32 offset: timer)")
        emit(encode_addi(22, 0, 264), "addi x22, x0, 264",
             "Step 2b: x22 = 264 (entry 33 offset: keyboard)")
        emit(encode_addi(23, 0, 1024), "addi x23, x0, 1024",
             "Step 2b: x23 = 1024 (entry 128 offset: syscall)")

        # IDT loop
        loop_start = address
        emit(encode_beq(13, 21, 20), "beq x13, x21, +20",
             "Step 2b: If timer entry, jump to timer store")
        emit(encode_beq(13, 22, 24), "beq x13, x22, +24",
             "Step 2b: If keyboard entry, jump to keyboard store")
        emit(encode_beq(13, 23, 28), "beq x13, x23, +28",
             "Step 2b: If syscall entry, jump to syscall store")
        emit(encode_sw(14, 13, 0), "sw x14, 0(x13)",
             "Step 2b: Store default handler at IDT[x13]")
        emit(encode_jal(0, 24), "jal x0, +24",
             "Step 2b: Skip special stores")
        emit(encode_sw(18, 13, 0), "sw x18, 0(x13)",
             "Step 2b: Store timer ISR at IDT[32]")
        emit(encode_jal(0, 16), "jal x0, +16",
             "Step 2b: Skip to flags store")
        emit(encode_sw(19, 13, 0), "sw x19, 0(x13)",
             "Step 2b: Store keyboard ISR at IDT[33]")
        emit(encode_jal(0, 8), "jal x0, +8",
             "Step 2b: Skip to flags store")
        emit(encode_sw(20, 13, 0), "sw x20, 0(x13)",
             "Step 2b: Store syscall ISR at IDT[128]")
        emit(encode_sw(17, 13, 4), "sw x17, 4(x13)",
             "Step 2b: Store flags at IDT[x13]+4")
        emit(encode_addi(13, 13, 8), "addi x13, x13, 8",
             "Step 2b: Advance to next IDT entry")
        loop_offset = loop_start - address
        emit(encode_blt(13, 16, loop_offset),
             f"blt x13, x16, {loop_offset}",
             "Step 2b: Loop if more entries")

        # === Step 3: Write HardwareInfo ===
        emit(encode_lui(5, HARDWARE_INFO_ADDR >> 12),
             f"lui x5, 0x{HARDWARE_INFO_ADDR >> 12:05X}",
             "Step 3: x5 = 0x00001000 (HardwareInfo base)")
        emit(encode_sw(8, 5, 0), "sw x8, 0(x5)",
             "Step 3: HardwareInfo.MemorySize = x8")

        emit(encode_addi(6, 0, self.config.display_columns),
             f"addi x6, x0, {self.config.display_columns}",
             f"Step 3: x6 = {self.config.display_columns}")
        emit(encode_sw(6, 5, 4), "sw x6, 4(x5)",
             "Step 3: HardwareInfo.DisplayColumns")

        emit(encode_addi(6, 0, self.config.display_rows),
             f"addi x6, x0, {self.config.display_rows}",
             f"Step 3: x6 = {self.config.display_rows}")
        emit(encode_sw(6, 5, 8), "sw x6, 8(x5)",
             "Step 3: HardwareInfo.DisplayRows")

        fb_upper = self.config.framebuffer_base >> 12
        fb_lower = self.config.framebuffer_base & 0xFFF
        emit(encode_lui(6, fb_upper),
             f"lui x6, 0x{fb_upper:05X}",
             f"Step 3: x6 upper = 0x{fb_upper:05X}000")
        if fb_lower:
            emit(encode_addi(6, 6, _sign_extend_12(fb_lower)),
                 f"addi x6, x6, {_sign_extend_12(fb_lower)}",
                 "Step 3: x6 lower bits for FramebufferBase")
        emit(encode_sw(6, 5, 12), "sw x6, 12(x5)",
             f"Step 3: HardwareInfo.FramebufferBase = 0x{self.config.framebuffer_base:08X}")

        emit(encode_sw(0, 5, 16), "sw x0, 16(x5)",
             "Step 3: HardwareInfo.IDTBase = 0")
        emit(encode_addi(6, 0, 256), "addi x6, x0, 256",
             "Step 3: x6 = 256")
        emit(encode_sw(6, 5, 20), "sw x6, 20(x5)",
             "Step 3: HardwareInfo.IDTEntries = 256")

        bl_upper = self.config.bootloader_entry >> 12
        bl_lower = self.config.bootloader_entry & 0xFFF
        emit(encode_lui(6, bl_upper),
             f"lui x6, 0x{bl_upper:05X}",
             f"Step 3: x6 = bootloader entry upper")
        if bl_lower:
            emit(encode_addi(6, 6, _sign_extend_12(bl_lower)),
                 f"addi x6, x6, {_sign_extend_12(bl_lower)}",
                 "Step 3: x6 lower bits for BootloaderEntry")
        emit(encode_sw(6, 5, 24), "sw x6, 24(x5)",
             f"Step 3: HardwareInfo.BootloaderEntry = 0x{self.config.bootloader_entry:08X}")

        # === Step 4: Jump to Bootloader ===
        emit(encode_lui(6, self.config.bootloader_entry >> 12),
             f"lui x6, 0x{self.config.bootloader_entry >> 12:05X}",
             "Step 4: x6 = bootloader entry upper bits")
        if self.config.bootloader_entry & 0xFFF:
            emit(encode_addi(6, 6, _sign_extend_12(
                self.config.bootloader_entry & 0xFFF)),
                f"addi x6, x6, {_sign_extend_12(self.config.bootloader_entry & 0xFFF)}",
                "Step 4: Add lower bits of bootloader entry")
        emit(encode_jalr(0, 6, 0), "jalr x0, x6, 0",
             f"Step 4: Jump to bootloader at 0x{self.config.bootloader_entry:08X}")

        return instructions


def _sign_extend_12(val: int) -> int:
    """Sign-extend a 12-bit value to a full int."""
    val = val & 0xFFF
    if val >= 0x800:
        return val - 0x1000
    return val


def _li_upper(value: int) -> int:
    """Compute the upper 20-bit value for LUI, compensating for sign extension."""
    upper = (value >> 12) & 0xFFFFF
    if value & 0x800:
        upper = (upper + 1) & 0xFFFFF
    return upper
