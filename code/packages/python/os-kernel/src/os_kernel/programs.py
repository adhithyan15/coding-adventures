"""RISC-V program generators for idle and hello-world processes.

These functions generate small RISC-V programs as raw machine code bytes.
"""

from __future__ import annotations

from riscv_simulator.encoding import (
    assemble,
    encode_addi,
    encode_ecall,
    encode_jal,
    encode_lui,
)

from os_kernel.syscall import REG_A0, REG_A1, REG_A2, REG_A7, SYS_EXIT, SYS_WRITE, SYS_YIELD

DEFAULT_USER_PROCESS_BASE: int = 0x00040000


def generate_idle_program() -> bytes:
    """Create the idle process binary.

    The idle process is an infinite loop that calls sys_yield:

        loop:
            li  a7, 3       # a7 = 3 (sys_yield)
            ecall           # trap to kernel
            jal x0, loop    # loop forever
    """
    instructions = [
        encode_addi(REG_A7, 0, SYS_YIELD),  # li a7, 3
        encode_ecall(),                       # ecall
        encode_jal(0, -8),                    # jal x0, -8 (back to li)
    ]
    return assemble(instructions)


def generate_hello_world_program(mem_base: int) -> bytes:
    """Create the hello-world process binary.

    The program stores "Hello World\\n" at offset 0x100, then:
      1. Loads the address of the string data
      2. Calls sys_write(fd=1, buf=&data, len=12)
      3. Calls sys_exit(0)
    """
    data_offset = 0x100
    data_addr = mem_base + data_offset
    message = b"Hello World\n"

    instructions: list[int] = []

    # Load data address into a1 (x11)
    upper = (data_addr >> 12) & 0xFFFFF
    lower = data_addr & 0xFFF
    if lower >= 0x800:
        upper = (upper + 1) & 0xFFFFF

    instructions.append(encode_lui(REG_A1, upper))
    if lower != 0:
        signed_lower = lower if lower < 0x800 else lower - 0x1000
        instructions.append(encode_addi(REG_A1, REG_A1, signed_lower))

    # a0 = 1 (stdout)
    instructions.append(encode_addi(REG_A0, 0, 1))
    # a2 = 12 (length)
    instructions.append(encode_addi(REG_A2, 0, len(message)))
    # a7 = 1 (sys_write)
    instructions.append(encode_addi(REG_A7, 0, SYS_WRITE))
    # ecall
    instructions.append(encode_ecall())
    # a0 = 0 (exit code)
    instructions.append(encode_addi(REG_A0, 0, 0))
    # a7 = 0 (sys_exit)
    instructions.append(encode_addi(REG_A7, 0, SYS_EXIT))
    # ecall
    instructions.append(encode_ecall())

    code = assemble(instructions)

    binary = bytearray(data_offset + len(message))
    binary[:len(code)] = code
    binary[data_offset:data_offset + len(message)] = message

    return bytes(binary)


def generate_hello_world_binary() -> bytes:
    """Alias using the default user process base address."""
    return generate_hello_world_program(DEFAULT_USER_PROCESS_BASE)
