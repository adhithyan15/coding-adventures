#!/usr/bin/env python3
"""Generate deterministic full-state Apple M1 FP/NEON oracle vectors."""

from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "code/packages/python/apple-m1-simulator/src"))
sys.path.insert(0, str(ROOT / "code/packages/python/simulator-protocol/src"))

from apple_m1_simulator import AppleM1Simulator  # noqa: E402
from apple_m1_simulator.state import AppleM1State, MASK64, MASK128  # noqa: E402
from apple_m1_simulator.simulator import (  # noqa: E402
    fcvtzs, fmov_fp_to_gpr_d, fmov_fp_to_gpr_s, fmov_gpr_to_fp_d,
    fmov_gpr_to_fp_s, fp_cmp, fp_dp1src, fp_dp2src, fp_ldst_uoff,
    neon_3reg_same, neon_dup_gpr, scvtf, ucvtf,
)


def f32(value: float) -> int:
    return int.from_bytes(struct.pack(">f", value), "big")


def f64(value: float) -> int:
    return int.from_bytes(struct.pack(">d", value), "big")


def fnv(parts: list[bytes]) -> int:
    value = 0xCBF29CE484222325
    for part in parts:
        for byte in part:
            value = ((value ^ byte) * 0x100000001B3) & MASK64
    return value


def hash_state(sim: AppleM1Simulator) -> int:
    state = sim.get_state()
    return fnv(
        [state.pc.to_bytes(8, "little")]
        + [value.to_bytes(8, "little") for value in state.gpr]
        + [state.sp.to_bytes(8, "little"), bytes((state.nzcv,))]
        + [value.to_bytes(16, "little") for value in state.vreg]
        + [bytes(state.memory), bytes((state.halted,))]
    )


def seeded(name: str, raw: bytes, seed: int) -> AppleM1Simulator:
    sim = AppleM1Simulator()
    gpr = [((seed + 1) * 0x9E3779B97F4A7C15 ^ i * 0xD1B54A32D192ED03) & MASK64 for i in range(32)]
    gpr[31] = 0
    gpr[1] = ((-37 - seed) & MASK64) if "signed" in name else 0x1122334455667788 + seed
    gpr[2] = 0x200
    vectors = [((i + 1) * 0x0102030405060708090A0B0C0D0E0F11 + seed) & MASK128 for i in range(32)]
    if "-d" in name:
        vectors[1], vectors[2], vectors[3] = f64(1.25 + seed), f64(2.5 - seed), f64(-3.75)
    else:
        vectors[1], vectors[2], vectors[3] = f32(1.25 + seed), f32(2.5 - seed), f32(-3.75)
    if name.startswith("div") and seed == 0:
        vectors[2] = 0
    memory = bytearray((address * 37 + seed * 53 + 11) & 0xFF for address in range(65_536))
    memory[:4] = raw
    sim._state = AppleM1State(0, tuple(gpr), 0x300, seed & 0xF, tuple(vectors), tuple(memory), False)
    return sim


def cases() -> list[tuple[str, bytes]]:
    out: list[tuple[str, bytes]] = []
    for ftype, suffix in ((0, "s"), (1, "d")):
        out += [(f"one-{op}-{suffix}", fp_dp1src(ftype, op, 1, 3)) for op in range(5)]
        out += [(f"{'div' if op == 1 else 'two'}-{op}-{suffix}", fp_dp2src(ftype, 2, op, 1, 3)) for op in range(4)]
        out += [(f"cmp-reg-{suffix}", fp_cmp(ftype, 2, 1)), (f"cmp-zero-{suffix}", fp_cmp(ftype, 0, 1, 3))]
        for sf in (0, 1):
            out += [
                (f"fcvtzs-{sf}-{suffix}", fcvtzs(sf, ftype, 1, 3)),
                (f"scvtf-signed-{sf}-{suffix}", scvtf(sf, ftype, 1, 3)),
                (f"ucvtf-{sf}-{suffix}", ucvtf(sf, ftype, 1, 3)),
            ]
    out += [
        ("move-gpr-fp-d", fmov_gpr_to_fp_d(1, 3)), ("move-fp-gpr-d", fmov_fp_to_gpr_d(1, 3)),
        ("move-gpr-fp-s", fmov_gpr_to_fp_s(1, 3)), ("move-fp-gpr-s", fmov_fp_to_gpr_s(1, 3)),
    ]
    for size, suffix in ((2, "s"), (3, "d")):
        out += [(f"ldst-{opc}-{suffix}", fp_ldst_uoff(size, opc, 1, 2, 3)) for opc in (0, 1)]
    for q in (0, 1):
        for imm5 in (1, 2, 4, 8, 16):
            out.append((f"dup-{q}-{imm5}", neon_dup_gpr(q, imm5, 1, 3)))
        for size in range(4):
            out += [
                (f"vadd-{q}-{size}", neon_3reg_same(q, 0, size, 2, 0x10, 1, 3)),
                (f"vsub-{q}-{size}", neon_3reg_same(q, 1, size, 2, 0x10, 1, 3)),
            ]
        for size in range(3):
            out.append((f"vmul-{q}-{size}", neon_3reg_same(q, 0, size, 2, 0x13, 1, 3)))
        for size, suffix in ((0, "s"), (1, "d")):
            out += [
                (f"vfadd-{q}-{suffix}", neon_3reg_same(q, 0, size, 2, 0x1A, 1, 3)),
                (f"vfsub-{q}-{suffix}", neon_3reg_same(q, 1, size, 2, 0x1A, 1, 3)),
                (f"vfmul-{q}-{suffix}", neon_3reg_same(q, 0, size, 2, 0x1B, 1, 3)),
                (f"fmla-{q}-{suffix}", neon_3reg_same(q, 0, size, 2, 0x19, 1, 3)),
            ]
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
