#!/usr/bin/env python3
"""Generate (or check) the synthetic WX-E vector tables of this directory.

stdlib only.  Expected answers come from Python's `hashlib`, never from the Rust
implementation under test, so the tables are independent data (VER04j section
5.5: a table cannot *derive* an answer, only pin data authored elsewhere).  The
*messages* are made up here; no third-party vector set (NIST CAVS, RFC 7748, ...)
is used or copied.

    python3 generate.py            write the .tfv files and print their SHA-256 pins
    python3 generate.py --check    fail if a checked-in file differs from the output
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def hexs(data: bytes) -> str:
    return data.hex()


def message(length: int) -> bytes:
    """A made-up message: a byte pattern that is neither text nor a known vector."""
    return bytes((i * 7 + length) % 256 for i in range(length))


# Lengths on either side of the SHA-256 padding boundaries (55/56 and 63/64,
# 119/120 and 127/128) plus a few arbitrary ones.
LENGTHS = [0, 1, 2, 3, 31, 32, 33, 55, 56, 57, 63, 64, 65, 111, 112, 119, 120,
           127, 128, 129, 255, 256, 1000]


def digests() -> str:
    lines = ["tfv 1", "# synthetic SHA-256 vectors: made-up messages, hashlib digests"]
    for n in LENGTHS:
        msg = message(n)
        md = hashlib.sha256(msg).digest()
        lines.append(f"len-{n} R2 Kmsg X{hexs(msg)} Kmd X{hexs(md)}")
    return "\n".join(lines) + "\n"


def ok(value: int) -> str:
    return f"M1 S{hexs(b'ok')} I{value}"


def err(case: str) -> str:
    return f"M1 S{hexs(b'err')} S{hexs(case.encode())}"


def first_bytes() -> str:
    lines = ["tfv 1", "# synthetic Result vectors: the first byte of a message, or EMPTY"]
    cases = [b"", b"\x00", b"\x01", b"\x7f", b"\x80", b"\xff", b"\x41\x42", b"\xfe" * 9]
    for i, msg in enumerate(cases):
        out = err("EMPTY") if not msg else ok(msg[0])
        lines.append(f"case-{i} R2 Kmsg X{hexs(msg)} Kout {out}")
    return "\n".join(lines) + "\n"


FILES = {"digests.tfv": digests, "first-bytes.tfv": first_bytes}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    bad = False
    for name, render in FILES.items():
        text = render().encode()
        path = HERE / name
        pin = hashlib.sha256(text).hexdigest()
        if args.check:
            if not path.is_file() or path.read_bytes() != text:
                print(f"{name}: drifted from generate.py", file=sys.stderr)
                bad = True
            else:
                print(f"{name}: current ({pin})")
        else:
            path.write_bytes(text)
            print(f"{name}: sha256 {pin}")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
