#!/usr/bin/env python3
"""Generate deterministic one-step full-state vectors from the Python oracle.

R15 is normalized to the authoritative separate ``pc`` field documented by
Python's state object. Rust intentionally keeps those two public views coherent.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
for package in ("armv7a-simulator", "simulator-protocol"):
    sys.path.insert(0, str(ROOT / "code" / "packages" / "python" / package / "src"))

from armv7a_simulator import ARMv7ASimulator  # noqa: E402


def seed_register(seed: int, index: int) -> int:
    return (((seed + 1) * 0x9E37_79B9) ^ (index * 0xD1B5_4A33)) & 0xFFFF_FFFF


def fnv1a(parts: list[bytes]) -> int:
    value = 0xCBF2_9CE4_8422_2325
    for part in parts:
        for byte in part:
            value ^= byte
            value = (value * 0x100_0000_01B3) & 0xFFFF_FFFF_FFFF_FFFF
    return value


def hash_state(sim: ARMv7ASimulator) -> int:
    state = sim.get_state()
    registers = list(state.gpr)
    registers[15] = state.pc
    return fnv1a(
        [state.pc.to_bytes(4, "little")]
        + [value.to_bytes(4, "little") for value in registers]
        + [state.cpsr.to_bytes(4, "little"), bytes(state.memory), bytes((state.halted,))]
    )


def seeded(raw: bytes, seed: int) -> ARMv7ASimulator:
    sim = ARMv7ASimulator()
    sim.load(raw)
    cpu = sim._cpu
    for index in range(15):
        cpu.gpr[index] = seed_register(seed, index)
    cpu.pc = 0
    cpu.cpsr = (1 << 5) | ((seed & 1) << 31) | (((seed >> 1) & 1) << 30)
    cpu.cpsr |= (((seed + 1) & 1) << 29) | (((seed >> 1) & 1) << 28)
    for address in range(65_536):
        cpu.memory[address] = (address * 37 + seed * 53 + 11) & 0xFF
    cpu.memory[: len(raw)] = raw
    cpu.halted = False
    return sim


def halfword(raw: int) -> bytes:
    return struct.pack("<H", raw)


def word(first: int, second: int) -> bytes:
    return struct.pack("<HH", first, second)


def cases() -> list[tuple[str, bytes]]:
    output: list[tuple[str, bytes]] = [("halt", halfword(0))]
    for operation, name in enumerate(("lsl-i", "lsr-i", "asr-i")):
        for amount in (0, 1, 17, 31):
            raw = (operation << 11) | (amount << 6) | (2 << 3) | 1
            output.append((f"{name}-{amount}", halfword(raw)))
    for operation, name in enumerate(("add-r", "sub-r", "add-i3", "sub-i3")):
        output.append((name, halfword(0x1800 | (operation << 9) | (3 << 6) | (2 << 3) | 1)))
    for operation, name in enumerate(("mov-i8", "cmp-i8", "add-i8", "sub-i8")):
        output.append((name, halfword(0x2000 | (operation << 11) | (2 << 8) | 0xA5)))
    data_names = (
        "and", "eor", "lsl-r", "lsr-r", "asr-r", "adc", "sbc", "ror",
        "tst", "rsb", "cmp-r", "cmn", "orr", "mul", "bic", "mvn",
    )
    for operation, name in enumerate(data_names):
        output.append((name, halfword(0x4000 | (operation << 6) | (2 << 3) | 1)))
    for operation, name in enumerate(("add-hi", "cmp-hi", "mov-hi", "bx")):
        output.append((name, halfword(0x4400 | (operation << 8) | (2 << 3) | 1)))
    output.append(("blx", halfword(0x4780 | (2 << 3))))
    memory_names = ("str-r", "strh-r", "strb-r", "ldrsb", "ldr-r", "ldrh-r", "ldrb-r", "ldrsh")
    for operation, name in enumerate(memory_names):
        output.append((name, halfword(0x5000 | (operation << 9) | (3 << 6) | (2 << 3) | 1)))
    for operation, name in zip(range(0b01100, 0b10010), ("str-i", "ldr-i", "strb-i", "ldrb-i", "strh-i", "ldrh-i")):
        output.append((name, halfword((operation << 11) | (7 << 6) | (2 << 3) | 1)))
    output.extend((
        ("str-sp", halfword(0x9105)),
        ("ldr-sp", halfword(0x9905)),
        ("adr", halfword(0xA105)),
        ("add-sp", halfword(0xB005)),
        ("sub-sp", halfword(0xB085)),
        ("push", halfword(0xB507)),
        ("pop", halfword(0xBD07)),
        ("nop", halfword(0xBF00)),
        ("stm", halfword(0xC207)),
        ("ldm", halfword(0xCA07)),
    ))
    for condition in range(14):
        output.append((f"b-cond-{condition:x}", halfword(0xD000 | (condition << 8) | 3)))
    output.append(("b", halfword(0xE003)))

    # BL +2, MOVW/MOVT, then every Python data-processing immediate dispatch.
    output.append(("bl", word(0xF000, 0xF801)))
    output.append(("movw", word(0xF241, 0x2034)))
    output.append(("movt", word(0xF2C5, 0x6078)))
    for operation, name in ((0, "and-w"), (2, "orr-w"), (4, "eor-w"), (8, "add-w"), (10, "adc-w"), (13, "sub-w"), (14, "rsb-w")):
        for set_flags in (0, 1):
            first = 0xF000 | (operation << 5) | (set_flags << 4) | 2
            second = (2 << 12) | (1 << 8) | 0xA5
            output.append((f"{name}-s{set_flags}", word(first, second)))
    output.append(("mov-w", word(0xF04F, 0x21A5)))
    for size, suffix in enumerate(("b", "h", "")):
        for load, stem in ((0, "str"), (1, "ldr")):
            first = 0xF800 | (size << 5) | (load << 4) | 2
            second = (1 << 12) | 0x345
            output.append((f"{stem}{suffix}-w", word(first, second)))
    return output


def main() -> None:
    lines = ["# name|seed|raw_fetch_bytes|projected_full_state_fnv1a64"]
    for name, raw in cases():
        seeds = (0,) if name == "halt" else range(4)
        for seed in seeds:
            sim = seeded(raw, seed)
            sim.step()
            lines.append(f"{name}|{seed}|{raw.hex()}|{hash_state(sim):016x}")
    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
