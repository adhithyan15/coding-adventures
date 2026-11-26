#!/usr/bin/env python3
"""Regenerate test/pbkdf2_vectors.json deterministically.

90 seeded vectors (30 per PRF) over passwords (empty included),
salts, iteration counts, and output lengths, derived with CPython's
hashlib.pbkdf2_hmac — a fully independent oracle from a different
implementation family than the Rust reference.

Usage: python3 test/generate_pbkdf2_vectors.py   (from the package dir)
"""
import hashlib
import json
import random
from pathlib import Path

SEED = 20261130
PER_PRF = 30

PRFS = ["sha1", "sha256", "sha512"]
PW_LENS = [0, 1, 4, 8, 20, 32, 64, 100]
SALT_LENS = [0, 4, 8, 16, 32, 64]
ITERATIONS = [1, 2, 3, 5, 10, 100, 1000]
DK_LENS = [1, 16, 19, 20, 32, 48, 64, 100, 128]


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    for prf in PRFS:
        for _ in range(PER_PRF):
            pw = bytes(rng.randrange(256)
                       for _ in range(rng.choice(PW_LENS)))
            salt = bytes(rng.randrange(256)
                         for _ in range(rng.choice(SALT_LENS)))
            iters = rng.choice(ITERATIONS)
            dklen = rng.choice(DK_LENS)
            dk = hashlib.pbkdf2_hmac(prf, pw, salt, iters, dklen)
            vectors.append({"prf": prf, "password": pw.hex(),
                            "salt": salt.hex(), "iterations": iters,
                            "key_length": dklen, "dk": dk.hex()})
    distinct = {v["dk"] for v in vectors}
    if len(distinct) < len(vectors):
        raise SystemExit("degenerate vectors: RNG broken?")
    out = Path(__file__).resolve().parent / "pbkdf2_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors},
                             indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
