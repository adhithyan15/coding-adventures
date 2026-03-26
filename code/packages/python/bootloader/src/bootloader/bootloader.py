"""Bootloader code generator -- produces RISC-V machine code that loads
the OS kernel from disk into RAM and transfers control to it.

The bootloader executes in four phases:

  Phase 1: Validate the boot protocol magic number (0xB007CAFE)
  Phase 2: Read boot parameters (kernel location, size, etc.)
  Phase 3: Copy kernel from disk to RAM (word-by-word loop)
  Phase 4: Set stack pointer and jump to kernel entry
"""

from __future__ import annotations

from dataclasses import dataclass

from riscv_simulator.encoding import (
    assemble,
    encode_addi,
    encode_beq,
    encode_bne,
    encode_jal,
    encode_jalr,
    encode_lui,
    encode_lw,
    encode_sw,
)

# =========================================================================
# Well-known addresses and constants
# =========================================================================

DEFAULT_ENTRY_ADDRESS: int = 0x00010000
DEFAULT_KERNEL_DISK_OFFSET: int = 0x00080000
DEFAULT_KERNEL_LOAD_ADDRESS: int = 0x00020000
DEFAULT_STACK_BASE: int = 0x0006FFF0
DISK_MEMORY_MAP_BASE: int = 0x10000000
BOOT_PROTOCOL_ADDRESS: int = 0x00001000
BOOT_PROTOCOL_MAGIC: int = 0xB007CAFE


# =========================================================================
# BootloaderConfig
# =========================================================================


@dataclass
class BootloaderConfig:
    """Configurable addresses for the bootloader.

    Attributes:
        entry_address: Where bootloader code lives (default: 0x00010000).
        kernel_disk_offset: Where the kernel starts in the disk image.
        kernel_load_address: Where to copy the kernel in RAM.
        kernel_size: Size of the kernel binary in bytes (must be word-aligned).
        stack_base: Initial stack pointer (default: 0x0006FFF0).
    """

    entry_address: int = DEFAULT_ENTRY_ADDRESS
    kernel_disk_offset: int = DEFAULT_KERNEL_DISK_OFFSET
    kernel_load_address: int = DEFAULT_KERNEL_LOAD_ADDRESS
    kernel_size: int = 0
    stack_base: int = DEFAULT_STACK_BASE


def DefaultBootloaderConfig() -> BootloaderConfig:  # noqa: ANN201, N802
    """Return a configuration with conventional addresses."""
    return BootloaderConfig()


# =========================================================================
# AnnotatedInstruction
# =========================================================================


@dataclass
class AnnotatedInstruction:
    """Pairs a 32-bit RISC-V instruction with human-readable explanation.

    Attributes:
        address: Memory location of this instruction.
        machine_code: Raw 32-bit RISC-V instruction word.
        assembly: Human-readable assembly mnemonic.
        comment: Explains what this instruction does in the boot sequence.
    """

    address: int
    machine_code: int
    assembly: str
    comment: str


# =========================================================================
# Helpers
# =========================================================================


def _sign_extend_12(val: int) -> int:
    """Sign-extend a 12-bit value to a full int.

    RISC-V ADDI treats its immediate as a signed 12-bit value,
    so values >= 0x800 are negative.
    """
    val = val & 0xFFF
    if val >= 0x800:
        return val - 0x1000
    return val


_REG_NAMES = {2: "sp", 5: "t0", 6: "t1", 7: "t2"}


def _emit_load_immediate(
    instructions: list[AnnotatedInstruction],
    address: int,
    rd: int,
    value: int,
    comment: str,
) -> int:
    """Emit instructions to load a 32-bit constant into a register.

    Returns the new address after emitting.
    """
    upper = (value >> 12) & 0xFFFFF
    lower = value & 0xFFF

    if lower >= 0x800:
        upper = (upper + 1) & 0xFFFFF

    reg_name = _REG_NAMES.get(rd, f"x{rd}")

    if upper != 0:
        instructions.append(AnnotatedInstruction(
            address=address,
            machine_code=encode_lui(rd, upper),
            assembly=f"lui {reg_name}, 0x{upper:05X}",
            comment=f"{comment} (upper: 0x{upper:05X}000)",
        ))
        address += 4

        if lower != 0:
            signed_lower = _sign_extend_12(lower)
            instructions.append(AnnotatedInstruction(
                address=address,
                machine_code=encode_addi(rd, rd, signed_lower),
                assembly=f"addi {reg_name}, {reg_name}, {signed_lower}",
                comment=f"{comment} (lower: {signed_lower})",
            ))
            address += 4
    elif lower != 0:
        signed_lower = _sign_extend_12(lower)
        instructions.append(AnnotatedInstruction(
            address=address,
            machine_code=encode_addi(rd, 0, signed_lower),
            assembly=f"addi {reg_name}, x0, {signed_lower}",
            comment=comment,
        ))
        address += 4
    else:
        instructions.append(AnnotatedInstruction(
            address=address,
            machine_code=encode_addi(rd, 0, 0),
            assembly=f"addi {reg_name}, x0, 0",
            comment=f"{comment} (value = 0)",
        ))
        address += 4

    return address


# =========================================================================
# Bootloader
# =========================================================================


class Bootloader:
    """Generates RISC-V machine code that loads the kernel from disk
    into RAM and transfers control to it.
    """

    def __init__(self: Bootloader, config: BootloaderConfig) -> None:
        self.config = config

    def generate(self: Bootloader) -> bytes:
        """Produce the bootloader as bytes of RISC-V machine code."""
        annotated = self.generate_with_comments()
        instruction_words = [a.machine_code for a in annotated]
        return assemble(instruction_words)

    def generate_with_comments(self: Bootloader) -> list[AnnotatedInstruction]:
        """Produce annotated instructions for debugging and education."""
        instructions: list[AnnotatedInstruction] = []
        address = self.config.entry_address

        def emit(code: int, asm: str, comment: str) -> None:
            nonlocal address
            instructions.append(AnnotatedInstruction(
                address=address,
                machine_code=code,
                assembly=asm,
                comment=comment,
            ))
            address += 4

        # =================================================================
        # Phase 1: Validate Boot Protocol
        # =================================================================
        emit(encode_lui(5, 1),
             "lui t0, 0x00001",
             "Phase 1: t0 = 0x00001000 (boot protocol address)")

        emit(encode_lw(6, 5, 0),
             "lw t1, 0(t0)",
             "Phase 1: t1 = memory[0x00001000] (magic number)")

        emit(encode_lui(7, 0xB007D),
             "lui t2, 0xB007D",
             "Phase 1: t2 upper = 0xB007D000 (compensated for sign extension)")

        emit(encode_addi(7, 7, _sign_extend_12(0xAFE)),
             f"addi t2, t2, {_sign_extend_12(0xAFE)}",
             "Phase 1: t2 = 0xB007CAFE (expected magic)")

        halt_branch_index = len(instructions)
        emit(encode_bne(6, 7, 0),
             "bne t1, t2, halt",
             "Phase 1: If magic wrong, halt (infinite loop)")

        # =================================================================
        # Phase 2: Read Boot Parameters
        # =================================================================
        source = DISK_MEMORY_MAP_BASE + self.config.kernel_disk_offset
        address = _emit_load_immediate(
            instructions, address, 5, source,
            "Phase 2: t0 = source (disk mapped kernel location)")

        address = _emit_load_immediate(
            instructions, address, 6, self.config.kernel_load_address,
            "Phase 2: t1 = destination (kernel load address)")

        address = _emit_load_immediate(
            instructions, address, 7, self.config.kernel_size,
            "Phase 2: t2 = bytes remaining (kernel size)")

        # =================================================================
        # Phase 3: Copy Kernel (word-by-word loop)
        # =================================================================
        emit(encode_beq(7, 0, 24),
             "beq t2, x0, +24",
             "Phase 3: Skip copy if kernel size is 0")

        copy_loop_addr = address

        emit(encode_lw(28, 5, 0),
             "lw t3, 0(t0)",
             "Phase 3: Load 4 bytes from disk [t0]")

        emit(encode_sw(28, 6, 0),
             "sw t3, 0(t1)",
             "Phase 3: Store 4 bytes to kernel RAM [t1]")

        emit(encode_addi(5, 5, 4),
             "addi t0, t0, 4",
             "Phase 3: Advance source pointer by 4")

        emit(encode_addi(6, 6, 4),
             "addi t1, t1, 4",
             "Phase 3: Advance destination pointer by 4")

        emit(encode_addi(7, 7, -4),
             "addi t2, t2, -4",
             "Phase 3: Decrement bytes remaining by 4")

        loop_offset = int(copy_loop_addr) - int(address)
        emit(encode_bne(7, 0, loop_offset),
             f"bne t2, x0, {loop_offset}",
             "Phase 3: Loop if bytes remain")

        # =================================================================
        # Phase 4: Set Stack and Jump to Kernel
        # =================================================================
        address = _emit_load_immediate(
            instructions, address, 2, self.config.stack_base,
            "Phase 4: sp = stack base")

        address = _emit_load_immediate(
            instructions, address, 5, self.config.kernel_load_address,
            "Phase 4: t0 = kernel entry address")

        emit(encode_jalr(0, 5, 0),
             "jalr x0, t0, 0",
             f"Phase 4: Jump to kernel at 0x{self.config.kernel_load_address:08X} (no return)")

        halt_addr = address
        emit(encode_jal(0, 0),
             "jal x0, 0",
             "Halt: Infinite loop (bad boot protocol magic)")

        # =================================================================
        # Patch the halt branch offset
        # =================================================================
        branch_pc = instructions[halt_branch_index].address
        halt_offset = int(halt_addr) - int(branch_pc)
        instructions[halt_branch_index].machine_code = encode_bne(6, 7, halt_offset)
        instructions[halt_branch_index].assembly = f"bne t1, t2, +{halt_offset}"

        return instructions

    def instruction_count(self: Bootloader) -> int:
        """Return the number of instructions in the bootloader."""
        return len(self.generate_with_comments())

    def estimate_cycles(self: Bootloader) -> int:
        """Estimate total cycles to copy the kernel."""
        iterations = self.config.kernel_size // 4
        return iterations * 6 + 20
