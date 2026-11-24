#!/usr/bin/env python3
"""Generate fuzz/reference_vectors_test.go from the independent Python oracle.

The fuzz-v1 corpus has weak spots (VER06e section 9, item 14): its single splice
case has equal splits and its single mutate_sequence case does not pin every
constant (interest values, deltas, the growth cap).  This script feeds seeded
random inputs through the reference algorithm in `code/scripts/fuzz_conformance.py`
and writes the results as a Go table, so the Go mutators are checked against an
oracle that was written apart from them.

    python3 code/packages/go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py

Huge flip counts (which the reference would loop over forever) use a closed form
derived independently here: unit u of n is hit  count//n + (u < count%n)  times.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / "code/scripts"))
import fuzz_conformance as ref  # noqa: E402

rng = random.Random(0xF022)


def rbytes(maxlen: int = 12) -> bytes:
    return bytes(rng.randrange(256) for _ in range(rng.randrange(maxlen + 1)))


def rint() -> int:
    kind = rng.randrange(5)
    if kind == 0:
        return rng.randrange(-20, 40)
    if kind == 1:
        return rng.randrange(-(1 << 40), 1 << 40)
    if kind == 2:
        return rng.randrange(-(10**30), 10**30)
    if kind == 3:
        return rng.choice([0, 1, -1, 255, 256, 2**63, -(2**63), 2**64, 2**64 - 1])
    return rng.randrange(0, 9)


def flips(data: bytes, off: int, count: int, bits: bool) -> bytes:
    if not data or count <= 0:
        return data
    n = len(data) * (8 if bits else 1)
    if count <= 300 and abs(off) < 10**9:
        return (ref.bit_flip if bits else ref.byte_flip)(data, off, count)
    buf = bytearray(data)
    base = off % n
    for u in range(n):
        k = count // n + (1 if (u - base) % n < count % n else 0)
        if k % 2:
            if bits:
                buf[u // 8] ^= 1 << (u % 8)
            else:
                buf[u] ^= 0xFF
    return bytes(buf)


def arith(data, off, delta, w, e):
    if w not in (1, 2, 4, 8) or len(data) < w:
        return data
    return ref.byte_arithmetic(data, off, delta, w, e)


def boundary(data, off, w, val, e):
    if w not in (1, 2, 4, 8) or len(data) < w:
        return data
    return ref.boundary_insert(data, off, w, val % (1 << (8 * w)), e)


def ref_sequence(data: bytes, seed: int, steps: int, max_len: int) -> bytes:
    """Reference mutate_sequence plus the VER06 section 3.5 growth cap, ported
    line for line from fuzz_conformance.mutate_sequence (which has no cap)."""
    r = ref.SplitMix64(seed)
    cur = data
    for _ in range(steps):
        if not cur:
            cur = b"\x00"
        c = r.bounded(7)
        if c == 0:
            cur = ref.bit_flip(cur, r.bounded(len(cur) * 8), 1)
        elif c == 1:
            cur = ref.byte_flip(cur, r.bounded(len(cur)), 1)
        elif c == 2:
            d = [-35, -8, -1, 1, 8, 35][r.bounded(6)]
            cur = ref.byte_arithmetic(cur, r.bounded(len(cur)), d, 1)
        elif c == 3:
            v = [0, 1, 127, 128, 255][r.bounded(5)]
            cur = ref.boundary_insert(cur, r.bounded(len(cur)), 1, v)
        elif c == 4:
            if len(cur) > 1:
                dl = 1 + r.bounded(max(1, len(cur) // 2))
                cur = ref.block_delete(cur, r.bounded(len(cur)), dl)
        elif c == 5:
            ins = bytes([r.bounded(256)])
            at = r.bounded(len(cur) + 1)
            if len(cur) + 1 <= max_len:
                cur = ref.block_insert(cur, at, ins)
        else:
            if len(cur) > 1:
                dl = 1 + r.bounded(max(1, len(cur) // 2))
                src = r.bounded(len(cur))
                dst = r.bounded(len(cur) + 1)
                if len(cur) + min(dl, len(cur) - src) <= max_len:
                    cur = ref.block_duplicate(cur, src, dl, dst)
    return cur


def q(n: int) -> str:
    return f'"{n}"'


rows: list[str] = []


def row(op: str, args: list[str], out: bytes) -> None:
    rows.append(f'\t{{"{op}", []string{{{", ".join(args)}}}, "{out.hex()}"}},')


def hx(b: bytes) -> str:
    return f'"{b.hex()}"'


for _ in range(70):
    d, o = rbytes(), rint()
    c = rng.choice([1, 2, 3, 7, 8, 9, 16, 17, 33, 300, 10**18, 10**18 + 7, 2**64 + 5, 10**30 + 1, 0, -3])
    row("bit_flip", [hx(d), q(o), q(c)], flips(d, o, c, True))
for _ in range(70):
    d, o = rbytes(), rint()
    c = rng.choice([1, 2, 3, 4, 5, 9, 300, 10**18, 10**18 + 1, 2**64 + 3, 10**30 + 1, 0, -2])
    row("byte_flip", [hx(d), q(o), q(c)], flips(d, o, c, False))
for _ in range(120):
    d, o, dl = rbytes(20), rint(), rint()
    w, e = rng.choice([1, 2, 4, 8, 3, 0, 16]), rng.choice(["little", "big"])
    row("arithmetic", [hx(d), q(o), q(dl), q(w), f'"{e}"'], arith(d, o, dl, w, e))
for _ in range(120):
    d, o = rbytes(20), rint()
    w, e = rng.choice([1, 2, 4, 8, 3, 0, 16]), rng.choice(["little", "big"])
    val = rng.choice(ref_vals := [0, 1, 127, 128, 255, 256, 65535, 65536, 2**31, 2**32 - 1, 2**63, 2**64 - 1, rng.randrange(2**64), -1])
    row("boundary", [hx(d), q(o), q(w), q(val), f'"{e}"'], boundary(d, o, w, val, e))
for _ in range(60):
    d, o, ln = rbytes(), rint(), rng.choice([-1, 0, 1, 2, 3, 5, 100, 10**20, rint()])
    row("block_delete", [hx(d), q(o), q(ln)], ref.block_delete(d, o, ln))
for _ in range(60):
    d, o, ins = rbytes(), rint(), rbytes(4)
    row("block_insert", [hx(d), q(o), hx(ins)], ref.block_insert(d, o, ins))
for _ in range(60):
    d, s, ln, ds = rbytes(), rint(), rng.choice([-1, 0, 1, 2, 3, 5, 100, 10**20]), rint()
    row("block_duplicate", [hx(d), q(s), q(ln), q(ds)], ref.block_duplicate(d, s, ln, ds))
for _ in range(60):
    t, dn, ts, ds = rbytes(), rbytes(), rint(), rint()
    row("splice", [hx(t), hx(dn), q(ts), q(ds)], ref.splice(t, dn, ts, ds))
for i in range(120):
    d = rbytes(10)
    seed = rng.choice([0, 1, 42, 2**64 - 1, rng.randrange(2**64)])
    steps = rng.choice([0, 1, 2, 3, 4, 8, 16, 40])
    cap = rng.choice([1 << 20, 1 << 20, 1 << 20, 24, 16, 5, 1])
    row("sequence", [hx(d), q(seed), q(steps), q(cap)], ref_sequence(d, seed, steps, cap))
# one-byte buffers need the stream word for Bounded(1)
for seed in range(40):
    row("sequence", [hx(b"\x07"), q(seed), q(6), q(1 << 20)], ref_sequence(b"\x07", seed, 6, 1 << 20))
    row("sequence", [hx(b""), q(seed), q(6), q(1 << 20)], ref_sequence(b"", seed, 6, 1 << 20))

out = ROOT / "code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"
out.write_text(
    "// Code generated by testdata/gen_reference_vectors.py; DO NOT EDIT.\n"
    "// Oracle: code/scripts/fuzz_conformance.py (see the generator's docstring).\n\n"
    "package fuzz\n\n"
    "// refVector is one oracle result: op, its arguments (data as hex, integers as\n"
    "// decimal text, endian as text) and the expected output as hex.\n"
    "type refVector struct {\n\top   string\n\targs []string\n\tout  string\n}\n\n"
    "var referenceVectors = []refVector{\n" + "\n".join(rows) + "\n}\n",
    encoding="utf-8",
)
print(len(rows), "vectors")
