#!/usr/bin/env python3
"""Generate deterministic one-step full-state vectors from the Python oracle."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
for package in ("riscv-simulator", "cpu-simulator", "simulator-protocol"):
    sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / package / "src"))

from riscv_simulator import RiscVSimulator  # noqa: E402
from riscv_simulator.csr import (  # noqa: E402
    CSR_MCAUSE,
    CSR_MEPC,
    CSR_MSCRATCH,
    CSR_MSTATUS,
    CSR_MTVEC,
)
from riscv_simulator.encoding import *  # noqa: E402, F403


CSRS = (CSR_MSTATUS, CSR_MTVEC, CSR_MSCRATCH, CSR_MEPC, CSR_MCAUSE)


def seed_register(seed: int, index: int) -> int:
    return (((seed + 1) * 0x9E37_79B9) ^ (index * 0xD1B5_4A33)) & 0xFFFF_FFFF


def fnv1a(parts: list[bytes]) -> int:
    value = 0xCBF2_9CE4_8422_2325
    for part in parts:
        for byte in part:
            value ^= byte
            value = (value * 0x100_0000_01B3) & 0xFFFF_FFFF_FFFF_FFFF
    return value


def hash_state(sim: RiscVSimulator) -> int:
    state = sim.get_state()
    return fnv1a(
        [state.pc.to_bytes(4, "little")]
        + [value.to_bytes(4, "little") for value in state.registers]
        + [
            state.csr_mstatus.to_bytes(4, "little"),
            state.csr_mtvec.to_bytes(4, "little"),
            state.csr_mscratch.to_bytes(4, "little"),
            state.csr_mepc.to_bytes(4, "little"),
            state.csr_mcause.to_bytes(4, "little"),
            state.memory,
            bytes((state.halted,)),
        ]
    )


def seeded(name: str, raw: int, seed: int) -> RiscVSimulator:
    sim = RiscVSimulator(65_536)
    for index in range(1, 32):
        sim.cpu.registers.write(index, seed_register(seed, index))
    sim.cpu.registers.write(2, 0x200)
    sim.cpu.pc = 0
    sim.cpu.halted = False
    for address in range(65_536):
        sim.cpu.memory._data[address] = (address * 37 + seed * 53 + 11) & 0xFF
    sim.cpu.memory._data[:4] = raw.to_bytes(4, "little")
    for index, address in enumerate(CSRS):
        sim.csr.write(address, seed_register(seed + 7, index))
    if name == "ecall-halt":
        sim.csr.write(CSR_MTVEC, 0)
    elif name == "ecall-trap":
        sim.csr.write(CSR_MTVEC, 0x100)
    elif name == "mret":
        sim.csr.write(CSR_MEPC, 0x100)
    return sim


def cases() -> list[tuple[str, int]]:
    output: list[tuple[str, int]] = []
    for name, encode in (
        ("addi", encode_addi),
        ("slti", encode_slti),
        ("sltiu", encode_sltiu),
        ("xori", encode_xori),
        ("ori", encode_ori),
        ("andi", encode_andi),
    ):
        output.append((name, encode(1, 2, -17)))
    for name, encode in (("slli", encode_slli), ("srli", encode_srli), ("srai", encode_srai)):
        for amount in (0, 1, 17, 31):
            output.append((f"{name}-{amount}", encode(1, 2, amount)))
    for name, encode in (
        ("add", encode_add),
        ("sub", encode_sub),
        ("sll", encode_sll),
        ("slt", encode_slt),
        ("sltu", encode_sltu),
        ("xor", encode_xor),
        ("srl", encode_srl),
        ("sra", encode_sra),
        ("or", encode_or),
        ("and", encode_and),
    ):
        output.append((name, encode(1, 2, 3)))
    for name, encode in (
        ("lb", encode_lb),
        ("lh", encode_lh),
        ("lw", encode_lw),
        ("lbu", encode_lbu),
        ("lhu", encode_lhu),
    ):
        output.append((name, encode(1, 2, 0)))
    for name, encode in (("sb", encode_sb), ("sh", encode_sh), ("sw", encode_sw)):
        output.append((name, encode(3, 2, 0)))
    for name, encode in (
        ("beq", encode_beq),
        ("bne", encode_bne),
        ("blt", encode_blt),
        ("bge", encode_bge),
        ("bltu", encode_bltu),
        ("bgeu", encode_bgeu),
    ):
        output.append((name, encode(2, 3, 8)))
    output.extend(
        (
            ("jal", encode_jal(1, 8)),
            ("jalr", encode_jalr(1, 2, 4)),
            ("lui", encode_lui(1, 0x81234)),
            ("auipc", encode_auipc(1, 0x81234)),
            ("ecall-halt", encode_ecall()),
            ("ecall-trap", encode_ecall()),
            ("mret", encode_mret()),
        )
    )
    for csr in CSRS:
        for name, encode in (("csrrw", encode_csrrw), ("csrrs", encode_csrrs), ("csrrc", encode_csrrc)):
            output.append((f"{name}-{csr:03x}", encode(1, csr, 3)))
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
