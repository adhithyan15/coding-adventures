#!/usr/bin/env python3
"""Generate deterministic Spec 07q one-step full-state hashes."""

from mips_r2000_simulator import MIPSSimulator

MEM_SIZE = 65_536
PC = 0x100
MASK = 0xFFFF_FFFF

R_FUNCTS = [
    0x00, 0x02, 0x03, 0x04, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x10, 0x11,
    0x12, 0x13, 0x18, 0x19, 0x1A, 0x1B, 0x20, 0x21, 0x22, 0x23, 0x24,
    0x25, 0x26, 0x27, 0x2A, 0x2B,
]
I_OPS = [
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    0x0C, 0x0D, 0x0E, 0x0F, 0x20, 0x21, 0x23, 0x24, 0x25, 0x28,
    0x29, 0x2B,
]


def corpus() -> list[tuple[int, int, str]]:
    vectors: list[tuple[int, int, str]] = []
    for variant in range(4):
        for funct in R_FUNCTS:
            word = (2 << 21) | (3 << 16) | (4 << 11) | (variant << 6) | funct
            vectors.append((variant, word, "ok"))
        for rt in (0, 1, 0x10, 0x11):
            vectors.append((variant, (1 << 26) | (2 << 21) | (rt << 16) | 2, "ok"))
        for op in I_OPS:
            if op in (2, 3):
                word = (op << 26) | 0x80
            elif op in (4, 5):
                word = (op << 26) | (2 << 21) | (3 << 16) | 2
            elif op in (6, 7):
                word = (op << 26) | (2 << 21) | 2
            elif op in (0x20, 0x24, 0x28):
                word = (op << 26) | (1 << 21) | (4 << 16) | variant
            elif op in (0x21, 0x25, 0x29):
                word = (op << 26) | (1 << 21) | (4 << 16) | (variant * 2)
            elif op in (0x23, 0x2B):
                word = (op << 26) | (1 << 21) | (4 << 16) | (variant * 4)
            else:
                word = (op << 26) | (2 << 21) | (4 << 16) | (0x1234 + variant)
            vectors.append((variant, word & MASK, "ok"))

    vectors.extend(
        [
            (0, 0xFC00_0000, "error"),
            (0, (2 << 21) | (3 << 16) | (4 << 11) | 0x3F, "error"),
            (0, (1 << 26) | (2 << 21) | (2 << 16), "error"),
            (0, 0x0000_000D, "error"),
            (0, (2 << 21) | (0 << 16) | (4 << 11) | 0x1A, "error"),
            (0, (0x23 << 26) | (1 << 21) | (4 << 16) | 1, "error"),
        ]
    )
    return vectors


def seeded_cpu(variant: int, word: int) -> MIPSSimulator:
    cpu = MIPSSimulator()
    cpu._regs = [((index + 1) * 0x1020_3041 ^ word) & MASK for index in range(32)]
    cpu._regs[0] = 0
    cpu._regs[1] = 0x800
    cpu._regs[2] = [0, 1, 0xFFFF_FFFF, 0x8000_0000][variant]
    cpu._regs[3] = 3
    cpu._regs[31] = 0x300
    cpu._hi = 0x1357_9BDF ^ word
    cpu._lo = 0x2468_ACE0 ^ word
    cpu._pc = PC
    cpu._halted = False
    cpu._mem[:] = bytes(((index * 29 + 0x47) & 0xFF) for index in range(MEM_SIZE))
    cpu._mem[PC : PC + 4] = word.to_bytes(4, "big")
    return cpu


def fnv_byte(value: int, byte: int) -> int:
    return ((value ^ byte) * 0x100000001B3) & 0xFFFF_FFFF_FFFF_FFFF


def state_hash(cpu: MIPSSimulator) -> int:
    value = 0xCBF29CE484222325
    for scalar in [cpu._pc, *cpu._regs, cpu._hi, cpu._lo]:
        for byte in (scalar & MASK).to_bytes(4, "little"):
            value = fnv_byte(value, byte)
    for byte in cpu._mem:
        value = fnv_byte(value, byte)
    return fnv_byte(value, int(cpu._halted))


for variant, instruction, _expected in corpus():
    machine = seeded_cpu(variant, instruction)
    try:
        machine.step()
    except ValueError:
        print(f"{variant} {instruction:08x} ERROR")
    else:
        print(f"{variant} {instruction:08x} {state_hash(machine):016x}")
