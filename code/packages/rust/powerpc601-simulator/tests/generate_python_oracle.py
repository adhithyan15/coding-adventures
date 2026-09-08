"""Generate deterministic one-step full-state PowerPC 601 oracle vectors."""

from __future__ import annotations

from pathlib import Path

from powerpc601_simulator import (
    PowerPC601Simulator,
    PowerPC601State,
    b_form,
    d_form,
    i_form,
    x_form,
    xfx_form,
    xl_form,
    xo_form,
)


def hash_state(state: PowerPC601State) -> int:
    value = 0xCBF2_9CE4_8422_2325

    def feed(data: bytes) -> None:
        nonlocal value
        for byte in data:
            value ^= byte
            value = (value * 0x100_0000_01B3) & 0xFFFF_FFFF_FFFF_FFFF

    feed(state.cia.to_bytes(4, "big"))
    for register in state.gpr:
        feed(register.to_bytes(4, "big"))
    for register in (state.lr, state.ctr, state.xer, state.cr):
        feed(register.to_bytes(4, "big"))
    feed(bytes(state.memory))
    feed(bytes([state.halted]))
    return value


def seeded(raw: int, seed: int) -> PowerPC601Simulator:
    sim = PowerPC601Simulator()
    sim.load(raw.to_bytes(4, "big"))
    registers = [
        (((seed + 1) * 0x9E37_79B9) ^ (index * 0xD1B5_4A33)) & 0xFFFF_FFFF
        for index in range(32)
    ]
    registers[1] = 0x200
    registers[2] = [0, 1, 0xFFFF_FFFF, 0x8000_0000][seed]
    registers[3] = [0, 3, 31, 63][seed]
    memory = [(address * 37 + seed * 53 + 11) & 0xFF for address in range(65_536)]
    memory[:4] = raw.to_bytes(4, "big")
    sim._state = PowerPC601State(
        cia=0,
        gpr=tuple(registers),
        lr=0x100 + seed * 4,
        ctr=seed + 1,
        xer=[0, 1 << 29, 1 << 31, (1 << 31) | (1 << 29)][seed],
        cr=[0, 0x8000_0000, 0x2000_0000, 0xF0F0_0F0F][seed],
        memory=tuple(memory),
        halted=False,
    )
    return sim


def main() -> None:
    words: list[tuple[str, bytes]] = [("halt", bytes(4))]
    for opcode in (8, 10, 11, 14, 15, 24, 25, 26, 28, 29):
        words.append((f"d-{opcode}", d_form(opcode, 4, 2, -7)))
    for opcode in (32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 44):
        words.append((f"memory-{opcode}", d_form(opcode, 4, 1, 0)))
    words.extend([
        ("b", i_form(18, 4)),
        ("bl", i_form(18, 4, LK=1)),
        ("bc-true", b_form(16, 18, 0, 4)),
        ("bc-false", b_form(16, 16, 0, 4)),
        ("bc-bdnz", b_form(16, 4, 0, 4)),
        ("bc-bdz", b_form(16, 12, 0, 4)),
        ("bclr", xl_form(19, 20, 0, 0, 16)),
        ("bcctr", xl_form(19, 20, 0, 0, 528)),
    ])
    for xo in (0, 32):
        words.append((f"compare-{xo}", x_form(31, 0, 2, 3, xo)))
    for xo in (24, 26, 28, 124, 316, 444, 476, 536, 792, 824):
        words.append((f"x-{xo}", x_form(31, 2, 4, 3, xo, rc=xo not in (26, 792, 824))))
    words.extend([
        ("mfcr", x_form(31, 4, 0, 0, 19)),
        ("mtcrf", (31 << 26 | 2 << 21 | 0xFF << 12 | 144 << 1).to_bytes(4, "big")),
    ])
    for spr in (1, 8, 9, 31):
        words.append((f"mfspr-{spr}", xfx_form(31, 4, spr, 339)))
        words.append((f"mtspr-{spr}", xfx_form(31, 2, spr, 467)))
    for xo in (10, 40, 104, 138, 235, 266, 459, 491):
        words.append((f"xo-{xo}", xo_form(31, 4, 2, 3, 0, xo)))

    lines = ["# name|seed|raw|hash"]
    for name, encoded in words:
        raw = int.from_bytes(encoded, "big")
        if raw == 0:
            seeds = [0]
        elif name in ("xo-459", "xo-491"):
            seeds = range(1, 4)
        else:
            seeds = range(4)
        for seed in seeds:
            sim = seeded(raw, seed)
            trace = sim.step()
            if trace.mnemonic.startswith("ERROR"):
                raise RuntimeError(f"valid vector failed: {name}/{seed}: {trace.mnemonic}")
            lines.append(f"{name}|{seed}|{raw:08x}|{hash_state(sim.get_state()):016x}")

    destination = Path(__file__).with_name("python_oracle_hashes.txt")
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} vectors to {destination}")


if __name__ == "__main__":
    main()
