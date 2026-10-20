#!/usr/bin/env python3
"""Verify the reviewed snapshot and provenance pins using the Python standard library."""

import hashlib
import json
import pathlib
import struct

PACKAGE = pathlib.Path(__file__).resolve().parent.parent
MAGIC = b"webpki-roots-canonical-v1\0"
EXPECTED_METADATA = {
    "format": "webpki-roots-canonical-v1",
    "upstream_repository": "https://github.com/rustls/webpki-roots",
    "upstream_release": "1.0.9",
    "upstream_tag": "v/1.0.9",
    "upstream_commit": "0a553dbc8b3f18ea05c4f881cffa3f2d005d0d30",
    "crates_io_archive_sha256": "7dcd9d09a39985f5344844e66b0c530a33843579125f23e21e9f0f220850f22a",
    "root_count": 121,
    "root_set_sha256": "0aa729fbb339061e8aad2e49fefd83674cd6b2e45eeb586c0d456cf33b70dc01",
    "reviewed_at": "2026-10-18",
    "maximum_review_interval_days": 90,
    "security_removal_review_days": 14,
    "includes_os_enterprise_or_local_roots": False,
}


def take_u64(encoded: bytes, offset: int) -> tuple[int, int]:
    end = offset + 8
    if end > len(encoded):
        raise ValueError(f"truncated length at {offset}")
    return struct.unpack(">Q", encoded[offset:end])[0], end


def take_bytes(encoded: bytes, offset: int, length: int) -> tuple[bytes, int]:
    end = offset + length
    if end > len(encoded):
        raise ValueError(f"truncated value at {offset}")
    return encoded[offset:end], end


def main() -> None:
    metadata = json.loads((PACKAGE / "snapshot.json").read_text(encoding="utf-8"))
    if metadata != EXPECTED_METADATA:
        raise SystemExit("snapshot provenance metadata differs from reviewed pins")
    encoded = (PACKAGE / "data" / "webpki-roots-1.0.9.bin").read_bytes()
    if not encoded.startswith(MAGIC):
        raise SystemExit("snapshot magic mismatch")

    offset = len(MAGIC)
    count = 0
    while offset < len(encoded):
        subject_length, offset = take_u64(encoded, offset)
        subject, offset = take_bytes(encoded, offset, subject_length)
        key_length, offset = take_u64(encoded, offset)
        key, offset = take_bytes(encoded, offset, key_length)
        if not subject or not key:
            raise SystemExit(f"empty required anchor field at root {count}")
        if offset >= len(encoded):
            raise SystemExit(f"missing constraints flag at root {count}")
        flag = encoded[offset]
        offset += 1
        if flag == 1:
            constraints_length, offset = take_u64(encoded, offset)
            constraints, offset = take_bytes(encoded, offset, constraints_length)
            if not constraints:
                raise SystemExit(f"empty present constraints at root {count}")
        elif flag != 0:
            raise SystemExit(f"invalid constraints flag at root {count}")
        count += 1

    digest = hashlib.sha256(encoded).hexdigest()
    if count != metadata["root_count"]:
        raise SystemExit(f"root count mismatch: expected {metadata['root_count']}, got {count}")
    if digest != metadata["root_set_sha256"]:
        raise SystemExit(
            f"root-set SHA-256 mismatch: expected {metadata['root_set_sha256']}, got {digest}"
        )
    print(f"verified {count} reviewed roots with digest {digest}")


if __name__ == "__main__":
    main()
