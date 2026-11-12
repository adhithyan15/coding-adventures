#!/usr/bin/env python3
"""Generate tests/data/sha256_vectors.txt for the Rust SHA-256 test suite.

Expected digests come from Python's hashlib (OpenSSL), an implementation that is
independent of the hand-rolled sha256.rs under test.  The message for length L is

    byte[j] = (j * 167 + L * 13 + 5) % 256      for j in 0..L

so the Rust test can regenerate the same bytes without storing them.

Usage (from the crate root):  python3 tests/data/gen_sha256_vectors.py

The script also re-verifies the FIPS 180-4 / NIST example vectors that the Rust
tests embed, so a typo in either place is caught here.
"""

import hashlib
import pathlib
import sys

NIST = {
    b"": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    b"abc": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq":
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    (b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn"
     b"hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"):
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    b"a" * 1_000_000:
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
}

LENGTHS = list(range(0, 201)) + [
    247, 248, 255, 256, 257, 311, 312, 319, 320, 511, 512, 513,
    1000, 1023, 1024, 1025, 4095, 4096, 4097, 65535, 65536, 65537, 100_000,
]


def message(length: int) -> bytes:
    return bytes((j * 167 + length * 13 + 5) % 256 for j in range(length))


def main() -> int:
    for msg, want in NIST.items():
        got = hashlib.sha256(msg).hexdigest()
        if got != want:
            print(f"NIST vector mismatch for {msg[:16]!r}...: {got} != {want}", file=sys.stderr)
            return 1
    out = pathlib.Path(__file__).with_name("sha256_vectors.txt")
    lines = ["# length hex-digest   (message byte j = (j*167 + length*13 + 5) % 256)"]
    for n in LENGTHS:
        lines.append(f"{n} {hashlib.sha256(message(n)).hexdigest()}")
    out.write_text("\n".join(lines) + "\n")
    print(f"wrote {len(LENGTHS)} vectors to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
