#!/usr/bin/env python3
"""Regenerate test/scrypt_vectors.json deterministically.

48 seeded vectors over passwords (empty included), salts, small
cost parameters, and output lengths, derived with CPython's
hashlib.scrypt (OpenSSL provider) — a fully independent oracle from
a different implementation family than the Rust reference.

Usage: python3 test/generate_scrypt_vectors.py   (from the package dir)
"""
import hashlib
import json
import random
from pathlib import Path

SEED = 20261130
COUNT = 48

PW_LENS = [0, 1, 4, 8, 20, 32]
SALT_LENS = [0, 4, 8, 16, 32]
NS = [2, 4, 8, 16, 32, 64, 128, 256]
RS = [1, 2, 4, 8]
PS = [1, 2]
DK_LENS = [1, 16, 32, 48, 64, 100]


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    for _ in range(COUNT):
        pw = bytes(rng.randrange(256) for _ in range(rng.choice(PW_LENS)))
        salt = bytes(rng.randrange(256) for _ in range(rng.choice(SALT_LENS)))
        n = rng.choice(NS)
        r = rng.choice(RS)
        p = rng.choice(PS)
        dklen = rng.choice(DK_LENS)
        dk = hashlib.scrypt(pw, salt=salt, n=n, r=r, p=p, dklen=dklen)
        vectors.append({"password": pw.hex(), "salt": salt.hex(), "n": n,
                        "r": r, "p": p, "dk_len": dklen, "dk": dk.hex()})
    distinct = {v["dk"] for v in vectors}
    if len(distinct) < len(vectors):
        raise SystemExit("degenerate vectors: RNG broken?")
    out = Path(__file__).resolve().parent / "scrypt_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors},
                             indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
