#!/usr/bin/env python3
"""Generate deterministic Python ARM1 one-step full-state hashes."""

from arm1_simulator import ARM1

MEMORY_SIZE = 16_384
PC = 0x100
MASK = 0xFFFF_FFFF


def corpus() -> list[int]:
    words: list[int] = []
    for cond in range(16):
        for opcode in range(16):
            s = (cond ^ opcode) & 1
            immediate = (
                (cond << 28)
                | (1 << 25)
                | (opcode << 21)
                | (s << 20)
                | (1 << 16)
                | (4 << 12)
                | (((cond + opcode) & 0xF) << 8)
                | ((0x35 + opcode) & 0xFF)
            )
            register = (
                (cond << 28)
                | (opcode << 21)
                | (s << 20)
                | (1 << 16)
                | (5 << 12)
                | (((opcode >> 2) & 0x3) << 5)
                | (((cond + opcode) & 0x1F) << 7)
                | 2
            )
            words.extend((immediate & MASK, register & MASK))

    for pre in range(2):
        for up in range(2):
            for byte in range(2):
                for write_back in range(2):
                    for load in range(2):
                        word = 0xE400_0020 | (1 << 16) | (6 << 12)
                        word |= pre << 24 | up << 23 | byte << 22
                        word |= write_back << 21 | load << 20
                        words.append(word & MASK)
                        words.append((word | (1 << 25) | 2) & MASK)

    for pre in range(2):
        for up in range(2):
            for write_back in range(2):
                for load in range(2):
                    word = 0xE800_0055 | (1 << 16)
                    word |= pre << 24 | up << 23 | write_back << 21 | load << 20
                    words.append(word & MASK)

    words.extend(
        [
            0xEA00_0002,
            0xEAFF_FFFE,
            0xEB00_0002,
            0xEBFF_FFFE,
            0xEF12_3456,
            0xEF00_0042,
            0xEE00_0010,
        ]
    )
    return words


def seeded_cpu(word: int) -> ARM1:
    cpu = ARM1(MEMORY_SIZE)
    cpu._regs = [((index + 1) * 0x1020_3041 ^ word) & MASK for index in range(27)]
    flags = ((word >> 4) & 0xF) << 28
    cpu._regs[15] = flags | PC | 3
    cpu._regs[1] = 0x800
    cpu._regs[2] = 0x20
    cpu._regs[3] = 3
    cpu._memory[:] = bytes(((index * 29 + 0x47) & 0xFF) for index in range(MEMORY_SIZE))
    cpu._memory[PC : PC + 4] = word.to_bytes(4, "little")
    cpu._halted = False
    return cpu


def fnv_byte(value: int, byte: int) -> int:
    return ((value ^ byte) * 0x100000001B3) & 0xFFFF_FFFF_FFFF_FFFF


def state_hash(cpu: ARM1) -> int:
    value = 0xCBF29CE484222325
    for register in cpu._regs:
        for byte in register.to_bytes(4, "little"):
            value = fnv_byte(value, byte)
    for byte in cpu._memory:
        value = fnv_byte(value, byte)
    return fnv_byte(value, int(cpu._halted))


for instruction in corpus():
    machine = seeded_cpu(instruction)
    machine.step()
    print(f"{instruction:08x} {state_hash(machine):016x}")
