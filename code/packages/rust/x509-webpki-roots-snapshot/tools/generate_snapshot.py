#!/usr/bin/env python3
"""Generate the canonical data-only snapshot from reviewed webpki-roots source."""

import argparse
import ast
import hashlib
import pathlib
import re
import struct

MAGIC = b"webpki-roots-canonical-v1\0"
BYTE_STRING = r'b"(?:\\.|[^"\\])*"'
ANCHOR = re.compile(
    rf"TrustAnchor\s*\{{\s*subject:\s*Der::from_slice\((?P<subject>{BYTE_STRING})\),\s*"
    rf"subject_public_key_info:\s*Der::from_slice\((?P<spki>{BYTE_STRING})\),\s*"
    rf"name_constraints:\s*(?:(?:Some\(Der::from_slice\((?P<constraints>{BYTE_STRING})\)\))|"
    rf"(?P<none>None))\s*\}}",
    re.DOTALL,
)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=pathlib.Path, required=True)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--source", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def decode_literal(value: str) -> bytes:
    decoded = ast.literal_eval(value)
    if not isinstance(decoded, bytes):
        raise ValueError("anchor field is not a byte string")
    return decoded


def main() -> None:
    arguments = parse_arguments()
    archive_digest = hashlib.sha256(arguments.archive.read_bytes()).hexdigest()
    if archive_digest != arguments.archive_sha256:
        raise SystemExit(
            f"archive SHA-256 mismatch: expected {arguments.archive_sha256}, got {archive_digest}"
        )

    source = arguments.source.read_text(encoding="utf-8")
    anchors = []
    for match in ANCHOR.finditer(source):
        constraints = match.group("constraints")
        anchors.append(
            (
                decode_literal(match.group("subject")),
                decode_literal(match.group("spki")),
                None if constraints is None else decode_literal(constraints),
            )
        )
    declared_count = source.count("TrustAnchor {")
    if not anchors or len(anchors) != declared_count:
        raise SystemExit(
            f"parsed {len(anchors)} anchors but source declares {declared_count}"
        )

    encoded = bytearray(MAGIC)
    for subject, spki, constraints in anchors:
        if not subject or not spki or constraints == b"":
            raise SystemExit("upstream anchor contains an empty required field")
        encoded.extend(struct.pack(">Q", len(subject)))
        encoded.extend(subject)
        encoded.extend(struct.pack(">Q", len(spki)))
        encoded.extend(spki)
        encoded.append(0 if constraints is None else 1)
        if constraints is not None:
            encoded.extend(struct.pack(">Q", len(constraints)))
            encoded.extend(constraints)

    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes(encoded)
    print(f"archive_sha256={archive_digest}")
    print(f"root_count={len(anchors)}")
    print(f"constrained_root_count={sum(value is not None for _, _, value in anchors)}")
    print(f"root_set_sha256={hashlib.sha256(encoded).hexdigest()}")


if __name__ == "__main__":
    main()
