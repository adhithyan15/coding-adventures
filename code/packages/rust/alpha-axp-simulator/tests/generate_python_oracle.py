"""Generate deterministic one-step full-state vectors from the Python oracle."""

from __future__ import annotations

import struct
from pathlib import Path

from alpha_axp_simulator import AlphaSimulator


def operate(op: int, func: int, immediate: bool) -> int:
    word = (op << 26) | (1 << 21) | (func << 5) | 3
    if immediate:
        return word | (0xA5 << 13) | (1 << 12)
    return word | (2 << 16)


def memory(op: int) -> int:
    return (op << 26) | (1 << 21) | (2 << 16)


def branch(op: int) -> int:
    return (op << 26) | (1 << 21) | 1


def jump(func: int) -> int:
    return (0x1A << 26) | (4 << 21) | (2 << 16) | (func << 14)


def seeded(sim: AlphaSimulator, raw: int, seed: int) -> None:
    sim.load(struct.pack("<I", raw))
    for index in range(31):
        sim._regs[index] = (
            ((seed + 1) * 0x9E37_79B9_7F4A_7C15)
            ^ (index * 0xD1B5_4A32_D192_ED03)
        ) & 0xFFFF_FFFF_FFFF_FFFF
    sim._regs[1] = [0, 1, 0xFFFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000][seed]
    sim._regs[2] = 0x200 + seed * 8
    sim._regs[3] = 0x0123_4567_89AB_CDEF
    for address in range(65_536):
        sim._mem[address] = (address * 37 + seed * 53 + 11) & 0xFF
    sim._mem[:4] = struct.pack("<I", raw)


def hash_state(sim: AlphaSimulator) -> int:
    state = sim.get_state()
    value = 0xCBF2_9CE4_8422_2325

    def feed(data: bytes) -> None:
        nonlocal value
        for byte in data:
            value ^= byte
            value = (value * 0x100_0000_01B3) & 0xFFFF_FFFF_FFFF_FFFF

    feed(state.pc.to_bytes(8, "little"))
    feed(state.npc.to_bytes(8, "little"))
    for register in state.regs:
        feed(register.to_bytes(8, "little"))
    feed(bytes(state.memory))
    feed(bytes([state.halted]))
    return value


def main() -> None:
    words: list[tuple[str, int]] = [("halt", 0)]
    families = {
        0x10: [
            0x00, 0x40, 0x20, 0x60, 0x09, 0x49, 0x29, 0x69, 0x18, 0x58,
            0x38, 0x78, 0x02, 0x22, 0x0B, 0x2B, 0x12, 0x32, 0x1B, 0x3B,
            0x2D, 0x4D, 0x6D, 0x3D, 0x7D,
        ],
        0x11: [
            0x00, 0x08, 0x20, 0x28, 0x40, 0x48, 0x14, 0x16, 0x24, 0x26,
            0x44, 0x46, 0x64, 0x66, 0x61, 0x6C,
        ],
        0x12: [
            0x39, 0x34, 0x3C, 0x06, 0x16, 0x26, 0x36, 0x0B, 0x1B, 0x2B,
            0x3B, 0x02, 0x12, 0x22, 0x32, 0x30, 0x31, 0x00, 0x01,
        ],
        0x13: [0x00, 0x40, 0x20, 0x60, 0x30],
    }
    for op, funcs in families.items():
        for func in funcs:
            for immediate in (False, True):
                words.append((f"op{op:02x}-f{func:02x}-i{int(immediate)}", operate(op, func, immediate)))
    for op in [0x28, 0x29, 0x2A, 0x2B, 0x0A, 0x0C, 0x2C, 0x2D, 0x0E, 0x0D]:
        words.append((f"mem-{op:02x}", memory(op)))
    for op in [0x30, 0x34, 0x39, 0x3D, 0x3A, 0x3B, 0x3F, 0x3E, 0x38, 0x3C]:
        words.append((f"branch-{op:02x}", branch(op)))
    for func in range(4):
        words.append((f"jump-{func}", jump(func)))

    lines = ["# name|seed|raw|outcome|hash"]
    for name, raw in words:
        seeds = [0] if raw == 0 else range(4)
        for seed in seeds:
            sim = AlphaSimulator()
            seeded(sim, raw, seed)
            trace = sim.step()
            if trace.mnemonic.startswith("ERROR"):
                raise RuntimeError(f"valid vector failed: {name}/{seed}: {trace.mnemonic}")
            lines.append(f"{name}|{seed}|{raw:08x}|{trace.mnemonic}|{hash_state(sim):016x}")

    errors = [
        ("unsupported-pal", 0x0000_0001),
        ("unknown-op", 0x0400_0000),
        ("unknown-inta", operate(0x10, 0x7F, False)),
        ("unknown-intl", operate(0x11, 0x7F, False)),
        ("unknown-ints", operate(0x12, 0x7F, False)),
        ("unknown-intm", operate(0x13, 0x7F, False)),
        ("misaligned-ldq", memory(0x29) | 1),
    ]
    for name, raw in errors:
        sim = AlphaSimulator()
        seeded(sim, raw, 0)
        trace = sim.step()
        if not trace.mnemonic.startswith("ERROR"):
            raise RuntimeError(f"fault vector succeeded: {name}")
        lines.append(f"{name}|0|{raw:08x}|ERROR|0000000000000000")

    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
