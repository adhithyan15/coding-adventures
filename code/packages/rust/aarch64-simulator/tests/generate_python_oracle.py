#!/usr/bin/env python3
"""Generate deterministic one-step full-state vectors from the Python oracle."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "code/packages/python/aarch64-simulator/src"))
sys.path.insert(0, str(ROOT / "code/packages/python/simulator-protocol/src"))

from aarch64_simulator import AArch64Simulator  # noqa: E402
from aarch64_simulator.state import AArch64State  # noqa: E402
from aarch64_simulator.simulator import (  # noqa: E402
    branch_cond, branch_imm, branch_reg, cbz_cbnz, csel_enc, dp_imm, dp_reg,
    ldst_uoff, logic_imm, logic_reg, madd_msub, movwide, tbz_tbnz,
)

MASK64 = (1 << 64) - 1


def one_source(sf: int, operation: int, rn: int = 2, rd: int = 1) -> bytes:
    raw = sf << 31 | 1 << 30 | 0b11010110 << 21 | operation << 10 | rn << 5 | rd
    return raw.to_bytes(4, "big")


def two_source(sf: int, operation: int, rn: int = 2, rm: int = 3, rd: int = 1) -> bytes:
    raw = sf << 31 | 0b11010110 << 21 | rm << 16 | operation << 10 | rn << 5 | rd
    return raw.to_bytes(4, "big")


def svc(immediate: int) -> bytes:
    return (0xD4000001 | ((immediate & 0xFFFF) << 5)).to_bytes(4, "big")


def seed_register(seed: int, index: int) -> int:
    return ((seed + 1) * 0x9E37_79B9_7F4A_7C15 ^ index * 0xD1B5_4A32_D192_ED03) & MASK64


def fnv1a(parts: list[bytes]) -> int:
    value = 0xCBF2_9CE4_8422_2325
    for part in parts:
        for byte in part:
            value ^= byte
            value = value * 0x100_0000_01B3 & MASK64
    return value


def hash_state(sim: AArch64Simulator) -> int:
    state = sim.get_state()
    return fnv1a(
        [state.pc.to_bytes(8, "little")]
        + [value.to_bytes(8, "little") for value in state.gpr]
        + [state.sp.to_bytes(8, "little"), bytes((state.nzcv,)), bytes(state.memory), bytes((state.halted,))]
    )


def seeded(name: str, raw: bytes, seed: int) -> AArch64Simulator:
    sim = AArch64Simulator()
    registers = [seed_register(seed, index) for index in range(32)]
    registers[31] = 0
    registers[2] = 0x200
    if name.startswith("cbz"):
        registers[1] = 0 if seed == 0 else registers[1]
    if name.startswith("cbnz"):
        registers[1] = 1 if seed == 0 else 0
    if name.startswith("tbz"):
        registers[1] &= ~1
    if name.startswith("tbnz"):
        registers[1] |= 1
    if name.startswith(("br-", "blr-", "ret-")):
        registers[1] = 0x100
    if name.startswith(("udiv", "sdiv")) and seed == 0:
        registers[3] = 0
    memory = bytearray((address * 37 + seed * 53 + 11) & 0xFF for address in range(65_536))
    memory[:4] = raw
    sim._state = AArch64State(0, tuple(registers), 0x300, seed & 0xF, tuple(memory), False)
    return sim


def cases() -> list[tuple[str, bytes]]:
    out: list[tuple[str, bytes]] = [("halt", b"\0\0\0\0"), ("nop", bytes.fromhex("d503201f"))]
    out += [("b", branch_imm(0, 2)), ("bl", branch_imm(1, -1))]
    out += [(f"bcond-{condition:x}", branch_cond(2, condition)) for condition in range(15)]
    for sf in (0, 1):
        out += [(f"cbz-{sf}", cbz_cbnz(sf, 0, 2, 1)), (f"cbnz-{sf}", cbz_cbnz(sf, 1, -1, 1))]
    out += [("tbz-0", tbz_tbnz(0, 0, 0, 2, 1)), ("tbnz-0", tbz_tbnz(0, 1, 0, -1, 1))]
    out += [("tbz-63", tbz_tbnz(1, 0, 31, 2, 1)), ("tbnz-63", tbz_tbnz(1, 1, 31, -1, 1))]
    out += [("br-reg", branch_reg(0, 1)), ("blr-reg", branch_reg(1, 1)), ("ret-reg", branch_reg(2, 1))]
    for sf in (0, 1):
        for op in (0, 1):
            for flags in (0, 1):
                for shift in (0, 1):
                    out.append((f"dpi-{sf}{op}{flags}{shift}", dp_imm(sf, op, flags, 0xA5, shift, 2, 1)))
        for opcode in (0, 2, 3):
            for halfword in range(2 if sf == 0 else 4):
                out.append((f"mov-{sf}-{opcode}-{halfword}", movwide(sf, opcode, halfword, 0xA55A, 1)))
        for opcode in range(4):
            out.append((f"logi-{sf}-{opcode}", logic_imm(sf, opcode, sf, 7, 42 if sf else 21, 2, 1)))
        for opcode in range(4):
            for invert in (0, 1):
                for shift in range(4):
                    out.append((f"logr-{sf}-{opcode}-{invert}-{shift}", logic_reg(sf, opcode, shift, invert, 3, 17 if sf else 7, 2, 1)))
        for op in (0, 1):
            for flags in (0, 1):
                for shift in range(3):
                    out.append((f"dpr-{sf}-{op}-{flags}-{shift}", dp_reg(sf, op, flags, shift, 3, 17 if sf else 7, 2, 1)))
        for operation, name in ((2, "udiv"), (3, "sdiv"), (8, "lslv"), (9, "lsrv"), (10, "asrv"), (11, "rorv")):
            out.append((f"{name}-{sf}", two_source(sf, operation)))
        for operation, name in ((0, "rbit"), (1, "rev16"), (2, "rev"), (4, "clz")):
            out.append((f"{name}-{sf}", one_source(sf, operation)))
        if sf:
            out.append(("rev32-1", one_source(1, 3)))
        out += [
            (f"madd-{sf}", madd_msub(sf, 0, 3, 0, 4, 2, 1)),
            (f"msub-{sf}", madd_msub(sf, 0, 3, 1, 4, 2, 1)),
        ]
        for invert in (0, 1):
            for increment in (0, 1):
                out.append((f"csel-{sf}-{invert}-{increment}", csel_enc(sf, invert, 0, 3, 0, increment, 2, 1)))
    out += [("smulh", madd_msub(1, 1, 3, 0, 0, 2, 1)), ("umulh", madd_msub(1, 2, 3, 0, 0, 2, 1))]
    for size in range(4):
        for opcode in range(4):
            if (size, opcode) not in ((2, 3), (3, 2), (3, 3)):
                out.append((f"mem-{size}-{opcode}", ldst_uoff(size, 0, opcode, 1, 2, 1)))
    out.append(("svc", svc(0xA55A)))
    return out


def main() -> None:
    lines = ["# name|seed|raw_word|projected_full_state_fnv1a64"]
    for name, raw in cases():
        for seed in range(4):
            sim = seeded(name, raw, seed)
            sim.step()
            lines.append(f"{name}|{seed}|{raw.hex()}|{hash_state(sim):016x}")
    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
