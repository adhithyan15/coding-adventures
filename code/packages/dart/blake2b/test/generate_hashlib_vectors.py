#!/usr/bin/env python3
"""Regenerate test/hashlib_vectors.json deterministically.

200 seeded vectors over boundary-heavy message lengths, digest sizes,
and key/salt/personal shapes, hashed with CPython's hashlib.blake2b
(the same oracle the Rust reference vectors came from).

Usage: python3 test/generate_hashlib_vectors.py   (from the package dir)
"""
import hashlib
import json
import random
from pathlib import Path

SEED = 20261124
COUNT = 200

MSG_LENS = [0, 1, 2, 63, 64, 65, 127, 128, 129, 200, 255, 256, 300, 511, 512, 1024]
DIGEST_SIZES = [1, 2, 16, 20, 28, 31, 32, 33, 48, 63, 64]
KEY_LENS = [0, 0, 0, 1, 7, 8, 15, 16, 31, 32, 33, 63, 64]
SALT_LENS = [0, 0, 16]


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    for _ in range(COUNT):
        msg = bytes(rng.randrange(256) for _ in range(rng.choice(MSG_LENS)))
        ds = rng.choice(DIGEST_SIZES)
        key = bytes(rng.randrange(256) for _ in range(rng.choice(KEY_LENS)))
        salt = bytes(rng.randrange(256) for _ in range(rng.choice(SALT_LENS)))
        personal = bytes(rng.randrange(256) for _ in range(rng.choice(SALT_LENS)))
        h = hashlib.blake2b(
            msg, digest_size=ds, key=key, salt=salt or b"", person=personal or b""
        )
        vectors.append(
            {
                "msg": msg.hex(),
                "digest_size": ds,
                "key": key.hex(),
                "salt": salt.hex(),
                "personal": personal.hex(),
                "digest": h.hexdigest(),
            }
        )
    out = Path(__file__).resolve().parent / "hashlib_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors}, indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
