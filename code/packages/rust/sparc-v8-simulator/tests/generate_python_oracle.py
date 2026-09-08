#!/usr/bin/env python3
"""Generate deterministic Spec 07r one-step full-state hashes.

The Python port's UMULcc/SMULcc/UDIVcc/SDIVcc constants are impossible
seven-bit values (0x5A/0x5B/0x5E/0x5F) in a six-bit op3 field.  Those four
manual-corrected Rust encodings therefore have manual-backed Rust tests and
are deliberately excluded from this older-port oracle corpus.
"""

from sparc_v8_simulator import SPARCSimulator

MEM_SIZE = 65_536
PC = 0x100
MASK = 0xFFFF_FFFF

ALU_OPS = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x0A, 0x0B, 0x0C, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1C,
    0x24, 0x25, 0x26, 0x27, 0x28, 0x30, 0x38, 0x3C, 0x3D,
]
MEM_OPS = [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x09, 0x0A]


def fmt3(op: int, rd: int, op3: int, rs1: int, immediate: int) -> int:
    return (
        (op << 30)
        | (rd << 25)
        | (op3 << 19)
        | (rs1 << 14)
        | (1 << 13)
        | (immediate & 0x1FFF)
    )


def corpus() -> list[tuple[int, int, str]]:
    vectors: list[tuple[int, int, str]] = []
    for seed in range(4):
        vectors.append((seed, 0x0100_0000, "ok"))
        vectors.append((seed, (4 << 25) | (4 << 22) | (0x12340 + seed), "ok"))
        for cond in range(16):
            vectors.append((seed, (cond << 25) | (2 << 22) | (seed + 1), "ok"))
        vectors.append((seed, (1 << 30) | (seed + 1), "ok"))

        for op3 in ALU_OPS:
            if op3 == 0x38:  # JMPL: use the aligned memory-base register.
                word = fmt3(2, 4, op3, 1, seed * 4)
            else:
                word = fmt3(2, 4, op3, 2, 3 + seed)
            vectors.append((seed, word, "ok"))

        for op3 in MEM_OPS:
            if op3 in (0x00, 0x04):
                offset = seed * 4
            elif op3 in (0x02, 0x06, 0x0A):
                offset = seed * 2
            else:
                offset = seed
            vectors.append((seed, fmt3(3, 4, op3, 1, offset), "ok"))

    vectors.extend(
        [
            (0, 0x91D0_2000, "ok"),
            (0, 1 << 22, "error"),
            (0, fmt3(2, 4, 0x3F, 2, 3), "error"),
            (0, fmt3(3, 4, 0x3F, 1, 0), "error"),
            (0, fmt3(2, 1, 0x3A, 0, 0), "error"),
            (0, fmt3(2, 4, 0x0E, 2, 0), "error"),
            (4, fmt3(2, 4, 0x3C, 2, 3), "error"),
            (0, fmt3(3, 4, 0x00, 1, 1), "error"),
        ]
    )
    return vectors


def seeded_cpu(seed: int, word: int) -> SPARCSimulator:
    variant = seed & 3
    cpu = SPARCSimulator()
    cpu._regs = [((index + 1) * 0x1020_3041 ^ word) & MASK for index in range(56)]
    cpu._regs[0] = 0
    cpu._regs[1] = 0x800
    cpu._regs[2] = [0, 1, 0xFFFF_FFFF, 0x8000_0000][variant]
    cpu._regs[3] = 3
    cpu._cwp = variant % 3
    cpu._save_depth = 2 if seed == 4 else 0
    cpu._psr_n = bool(variant & 1)
    cpu._psr_z = bool(variant & 2)
    cpu._psr_v = variant == 3
    cpu._psr_c = variant in (1, 2)
    cpu._y = 0
    cpu._pc = PC
    cpu._npc = PC + 4
    cpu._halted = False
    cpu._mem[:] = bytes(((index * 29 + 0x47) & 0xFF) for index in range(MEM_SIZE))
    cpu._mem[PC : PC + 4] = word.to_bytes(4, "big")
    return cpu


def fnv_byte(value: int, byte: int) -> int:
    return ((value ^ byte) * 0x100000001B3) & 0xFFFF_FFFF_FFFF_FFFF


def state_hash(cpu: SPARCSimulator) -> int:
    value = 0xCBF29CE484222325
    for scalar in [cpu._pc, cpu._npc, *cpu._regs, cpu._cwp, cpu._save_depth]:
        for byte in (scalar & MASK).to_bytes(4, "little"):
            value = fnv_byte(value, byte)
    for flag in [cpu._psr_n, cpu._psr_z, cpu._psr_v, cpu._psr_c]:
        value = fnv_byte(value, int(flag))
    for byte in (cpu._y & MASK).to_bytes(4, "little"):
        value = fnv_byte(value, byte)
    for byte in cpu._mem:
        value = fnv_byte(value, byte)
    return fnv_byte(value, int(cpu._halted))


for seed, instruction, _expected in corpus():
    machine = seeded_cpu(seed, instruction)
    try:
        machine.step()
    except ValueError:
        print(f"{seed} {instruction:08x} ERROR")
    else:
        print(f"{seed} {instruction:08x} {state_hash(machine):016x}")
