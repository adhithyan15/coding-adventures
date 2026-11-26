#!/usr/bin/env python3
"""Regenerate test/hmac_vectors.json deterministically.

40 seeded (variant, key, message) cases per HMAC variant (md5, sha1,
sha256, sha384, sha512) over boundary-heavy key lengths (block-size
edges at 63/64/65 and 127/128/129, empty key excluded — the lane
rejects it) and message lengths, tagged with CPython's hmac module.

Usage: python3 test/generate_hmac_vectors.py   (from the package dir)
"""
import hashlib
import hmac as hmac_module
import json
import random
from pathlib import Path

SEED = 20261127
PER_VARIANT = 40

VARIANTS = ["md5", "sha1", "sha256", "sha384", "sha512"]

KEY_LENS = [1, 2, 7, 16, 19, 20, 31, 32, 55, 63, 64, 65, 66, 80,
            100, 111, 127, 128, 129, 130, 131, 200, 256, 257]
MSG_LENS = [0, 1, 2, 7, 8, 50, 55, 56, 64, 111, 112, 128, 200,
            256, 300, 511, 512]


def main() -> None:
    rng = random.Random(SEED)
    vectors = []
    for variant in VARIANTS:
        digestmod = getattr(hashlib, variant)
        for _ in range(PER_VARIANT):
            key = bytes(rng.randrange(256)
                        for _ in range(rng.choice(KEY_LENS)))
            msg = bytes(rng.randrange(256)
                        for _ in range(rng.choice(MSG_LENS)))
            tag = hmac_module.new(key, msg, digestmod).hexdigest()
            vectors.append({"variant": variant, "key": key.hex(),
                            "msg": msg.hex(), "tag": tag})
    out = Path(__file__).resolve().parent / "hmac_vectors.json"
    out.write_text(json.dumps({"seed": SEED, "vectors": vectors},
                             indent=1) + "\n")
    print(f"wrote {len(vectors)} vectors to {out}")


if __name__ == "__main__":
    main()
