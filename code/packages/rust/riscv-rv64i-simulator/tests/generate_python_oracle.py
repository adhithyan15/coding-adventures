#!/usr/bin/env python3
"""Generate deterministic one-step full-state vectors from the Python RV64I oracle."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(
    0, str(ROOT / "code" / "packages" / "python" / "riscv-rv64i-simulator" / "src")
)

from riscv_rv64i_simulator import RV64ISimulator  # noqa: E402


MASK64 = (1 << 64) - 1


def r(opcode: int, funct3: int, funct7: int, rd: int, rs1: int, rs2: int) -> int:
    return funct7 << 25 | rs2 << 20 | rs1 << 15 | funct3 << 12 | rd << 7 | opcode


def i(opcode: int, funct3: int, rd: int, rs1: int, immediate: int) -> int:
    return (immediate & 0xFFF) << 20 | rs1 << 15 | funct3 << 12 | rd << 7 | opcode


def s(funct3: int, rs1: int, rs2: int, immediate: int) -> int:
    immediate &= 0xFFF
    return (
        immediate >> 5 << 25
        | rs2 << 20
        | rs1 << 15
        | funct3 << 12
        | (immediate & 31) << 7
        | 0x23
    )


def b(funct3: int, rs1: int, rs2: int, offset: int) -> int:
    immediate = offset & 0x1FFF
    return (
        (immediate >> 12 & 1) << 31
        | (immediate >> 5 & 0x3F) << 25
        | rs2 << 20
        | rs1 << 15
        | funct3 << 12
        | (immediate >> 1 & 0xF) << 8
        | (immediate >> 11 & 1) << 7
        | 0x63
    )


def j(rd: int, offset: int) -> int:
    immediate = offset & 0x1F_FFFF
    return (
        (immediate >> 20 & 1) << 31
        | (immediate >> 1 & 0x3FF) << 21
        | (immediate >> 11 & 1) << 20
        | (immediate >> 12 & 0xFF) << 12
        | rd << 7
        | 0x6F
    )


def seed_register(seed: int, index: int) -> int:
    return (
        (seed + 1) * 0x9E37_79B9_7F4A_7C15
        ^ index * 0xD1B5_4A32_D192_ED03
    ) & MASK64


def fnv1a(parts: list[bytes]) -> int:
    value = 0xCBF2_9CE4_8422_2325
    for part in parts:
        for byte in part:
            value ^= byte
            value = value * 0x100_0000_01B3 & MASK64
    return value


def hash_state(sim: RV64ISimulator) -> int:
    state = sim.get_state()
    return fnv1a(
        [state.pc.to_bytes(8, "little")]
        + [value.to_bytes(8, "little") for value in state.gpr]
        + [bytes(state.memory), bytes((state.halted,))]
    )


def seeded(name: str, raw: int, seed: int) -> RV64ISimulator:
    sim = RV64ISimulator()
    cpu = sim._cpu
    for index in range(1, 32):
        cpu.gpr[index] = seed_register(seed, index)
    cpu.gpr[2] = 0x200
    if name.startswith(("beq", "bne")) and seed == 0:
        cpu.gpr[3] = cpu.gpr[2]
    if name.startswith(("div", "rem")):
        if seed == 0:
            cpu.gpr[3] = 0
        elif seed == 1:
            cpu.gpr[2] = 1 << 63
            cpu.gpr[3] = MASK64
    cpu.pc = 0
    cpu.halted = False
    for address in range(65_536):
        cpu.memory[address] = (address * 37 + seed * 53 + 11) & 0xFF
    cpu.memory[:4] = raw.to_bytes(4, "little")
    return sim


def cases() -> list[tuple[str, int]]:
    output: list[tuple[str, int]] = []
    for name, funct3 in (
        ("addi", 0), ("slti", 2), ("sltiu", 3),
        ("xori", 4), ("ori", 6), ("andi", 7),
    ):
        output.append((name, i(0x13, funct3, 1, 2, -17)))
    for name, funct3, upper in (("slli", 1, 0), ("srli", 5, 0), ("srai", 5, 0x400)):
        for amount in (0, 1, 17, 31, 32, 63):
            output.append((f"{name}-{amount}", i(0x13, funct3, 1, 2, upper | amount)))
    for name, funct3, funct7 in (
        ("add", 0, 0), ("sub", 0, 0x20), ("sll", 1, 0), ("slt", 2, 0),
        ("sltu", 3, 0), ("xor", 4, 0), ("srl", 5, 0), ("sra", 5, 0x20),
        ("or", 6, 0), ("and", 7, 0),
    ):
        output.append((name, r(0x33, funct3, funct7, 1, 2, 3)))
    for name, funct3 in (
        ("lb", 0), ("lh", 1), ("lw", 2), ("ld", 3),
        ("lbu", 4), ("lhu", 5), ("lwu", 6),
    ):
        output.append((name, i(0x03, funct3, 1, 2, 0)))
    for name, funct3 in (("sb", 0), ("sh", 1), ("sw", 2), ("sd", 3)):
        output.append((name, s(funct3, 2, 3, 0)))
    for name, funct3 in (
        ("beq", 0), ("bne", 1), ("blt", 4),
        ("bge", 5), ("bltu", 6), ("bgeu", 7),
    ):
        output.append((name, b(funct3, 2, 3, 8)))
    output.extend((
        ("lui", 0x81234 << 12 | 1 << 7 | 0x37),
        ("auipc", 0x81234 << 12 | 1 << 7 | 0x17),
        ("jal", j(1, 8)),
        ("jalr", i(0x67, 0, 1, 2, 4)),
        ("addiw", i(0x1B, 0, 1, 2, -17)),
    ))
    for name, funct3, upper in (("slliw", 1, 0), ("srliw", 5, 0), ("sraiw", 5, 0x400)):
        for amount in (0, 1, 17, 31):
            output.append((f"{name}-{amount}", i(0x1B, funct3, 1, 2, upper | amount)))
    for name, funct3, funct7 in (
        ("addw", 0, 0), ("subw", 0, 0x20), ("sllw", 1, 0),
        ("srlw", 5, 0), ("sraw", 5, 0x20),
    ):
        output.append((name, r(0x3B, funct3, funct7, 1, 2, 3)))
    for name, funct3 in (
        ("mul", 0), ("mulh", 1), ("mulhsu", 2), ("mulhu", 3),
        ("div", 4), ("divu", 5), ("rem", 6), ("remu", 7),
    ):
        output.append((name, r(0x33, funct3, 1, 1, 2, 3)))
    for name, funct3 in (
        ("mulw", 0), ("divw", 4), ("divuw", 5), ("remw", 6), ("remuw", 7),
    ):
        output.append((name, r(0x3B, funct3, 1, 1, 2, 3)))
    output.extend((
        ("fence", 0x0000_000F), ("fence.i", 0x0000_100F),
        ("ecall", 0x0000_0073), ("ebreak", 0x0010_0073), ("zero-halt", 0),
    ))
    return output


def main() -> None:
    lines = ["# name|seed|raw_word|projected_full_state_fnv1a64"]
    for name, raw in cases():
        for seed in range(4):
            sim = seeded(name, raw, seed)
            sim.step()
            lines.append(f"{name}|{seed}|{raw:08x}|{hash_state(sim):016x}")
    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
