#!/usr/bin/env python3
"""Regenerate test/hashlib_vectors.json deterministically.

200 seeded messages over boundary-heavy lengths (padding edges at
111/112/127/128/129, multi-block, empty), hashed with CPython's
hashlib.sha384.

Usage: python3 test/generate_hashlib_vectors.py   (from the package dir)
"""
import hashlib
import json
import random
from pathlib import Path

SEED = 20261126
COUNT = 200

MSG_LENS = [0, 1, 2, 55, 56, 63, 64, 110, 111, 112, 113, 127, 128, 129,
            200, 223, 224, 239, 240, 255, 256, 300, 511, 512, 1024, 2048]


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    for _ in range(COUNT):
        msg = bytes(rng.randrange(256) for _ in range(rng.choice(MSG_LENS)))
        vectors.append({"msg": msg.hex(), "digest": hashlib.sha384(msg).hexdigest()})
    out = Path(__file__).resolve().parent / "hashlib_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors}, indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
