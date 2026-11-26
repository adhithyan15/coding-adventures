#!/usr/bin/env python3
"""Regenerate test/argon2_vectors.json deterministically.

60 seeded vectors over passwords, ASCII salts, and cost-parameter
sweeps, hashed with the reference C `argon2` CLI (RFC 9106 reference
implementation) in Argon2i mode — a fully independent oracle from a
different implementation family than the Rust reference.

The CLI takes string salts and supports no key/AD flags, so vectors
cover unkeyed hashing; key/AD paths ride the RFC 5.2 unit vector plus
binding properties.

Usage: python3 test/generate_argon2_vectors.py   (from the package dir)
Requires: the `argon2` CLI on PATH.
"""
import json
import random
import subprocess
from pathlib import Path

SEED = 20261129
COUNT = 60

# NOTE: length 0 excluded — the CLI refuses empty passwords ("no
# password read"). Empty-password hashing is covered by unit tests.
PW_LENS = [1, 7, 8, 32, 100]
SALTS = ["saltsalt", "somesalt12345678", "0123456789abcdef01234567"]
TIME_COSTS = [1, 2, 3]
MEMORIES = [8, 16, 32]
PARALLELISMS = [1, 2, 4]
TAG_LENS = [4, 16, 32, 48, 64, 128]


def reference_tag(password: bytes, salt: str, t: int, m: int, p: int, tag: int) -> str:
    proc = subprocess.run(
        ["argon2", salt, "-i", "-t", str(t), "-k", str(m),
         "-p", str(p), "-l", str(tag), "-r"],
        input=password,
        capture_output=True,
        check=True,
        timeout=120,
    )
    return proc.stdout.decode("ascii").strip()


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    while len(vectors) < COUNT:
        pw = bytes(rng.randrange(256) for _ in range(rng.choice(PW_LENS)))
        salt = rng.choice(SALTS)
        t = rng.choice(TIME_COSTS)
        m = rng.choice(MEMORIES)
        p = rng.choice(PARALLELISMS)
        if m < 8 * p:
            continue
        tag = rng.choice(TAG_LENS)
        digest = reference_tag(pw, salt, t, m, p, tag)
        vectors.append(
            {
                "password": pw.hex(),
                "salt": salt,
                "time_cost": t,
                "memory_cost": m,
                "parallelism": p,
                "tag_length": tag,
                "digest": digest,
            }
        )
    out = Path(__file__).resolve().parent / "argon2_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors}, indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
