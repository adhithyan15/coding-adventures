#!/usr/bin/env python3
"""Generate the canonical data-only snapshot from reviewed webpki-roots source."""

import argparse
import ast
import hashlib
import pathlib
import re
import struct
import tarfile

MAGIC = b"webpki-roots-canonical-v1\0"
SOURCE_MEMBER = "webpki-roots-1.0.9/src/lib.rs"
MAX_SOURCE_BYTES = 1024 * 1024
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
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def decode_literal(value: str) -> bytes:
    decoded = ast.literal_eval(value)
    if not isinstance(decoded, bytes):
        raise ValueError("anchor field is not a byte string")
    return decoded


def read_authenticated_source(archive_path: pathlib.Path) -> str:
    """Read the one exact regular source member without extracting the archive."""
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            members = [member for member in archive.getmembers() if member.name == SOURCE_MEMBER]
            if len(members) != 1:
                raise ValueError(
                    f"archive must contain exactly one {SOURCE_MEMBER}; found {len(members)}"
                )
            member = members[0]
            if not member.isfile() or member.issym() or member.islnk():
                raise ValueError(f"archive member {SOURCE_MEMBER} is not a regular file")
            if member.size <= 0 or member.size > MAX_SOURCE_BYTES:
                raise ValueError(
                    f"archive member {SOURCE_MEMBER} has invalid size {member.size}"
                )
            extracted = archive.extractfile(member)
            if extracted is None:
                raise ValueError(f"archive member {SOURCE_MEMBER} cannot be read")
            source_bytes = extracted.read(MAX_SOURCE_BYTES + 1)
            if len(source_bytes) != member.size:
                raise ValueError(f"archive member {SOURCE_MEMBER} size does not match metadata")
    except (OSError, tarfile.TarError) as error:
        raise ValueError(f"cannot read authenticated archive: {error}") from error

    try:
        return source_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"archive member {SOURCE_MEMBER} is not UTF-8") from error


def main() -> None:
    arguments = parse_arguments()
    archive_digest = hashlib.sha256(arguments.archive.read_bytes()).hexdigest()
    if archive_digest != arguments.archive_sha256:
        raise SystemExit(
            f"archive SHA-256 mismatch: expected {arguments.archive_sha256}, got {archive_digest}"
        )

    try:
        source = read_authenticated_source(arguments.archive)
    except ValueError as error:
        raise SystemExit(str(error)) from error
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
