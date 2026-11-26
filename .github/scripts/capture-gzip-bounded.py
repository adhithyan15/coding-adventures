#!/usr/bin/env python3
"""Stream stdin into a deterministic, bounded single-member gzip trace."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
import zlib
from pathlib import Path
from typing import BinaryIO

CHUNK_BYTES = 64 * 1024
GZIP_HEADER = b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\xff"


def _write_all(output: BinaryIO, data: bytes) -> None:
    view = memoryview(data)
    while view:
        written = output.write(view)
        if written is None or written <= 0:
            raise OSError("short write while storing gzip trace")
        view = view[written:]


def _hash_file(path: Path) -> str | None:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as stream:
            while chunk := stream.read(CHUNK_BYTES):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def _write_json_exclusive(path: Path, value: dict[str, object]) -> str | None:
    try:
        with path.open("x", encoding="utf-8", newline="\n") as stream:
            json.dump(value, stream, sort_keys=True, indent=2)
            stream.write("\n")
    except OSError as error:
        return str(error)
    return None


def capture(args: argparse.Namespace) -> int:
    errors: list[str] = []
    raw_observed = 0
    raw_fed = 0
    compressed_stored = 0
    chunks_read = 0
    raw_cap_reached = False
    compressed_cap_reached = False
    input_read_error = False
    gzip_finalized = False
    output: BinaryIO | None = None
    compressor: zlib.Compress | None = None
    raw_digest = hashlib.sha256()
    crc32 = 0

    try:
        output = args.output.open("xb")
    except OSError as error:
        errors.append(f"cannot create compressed trace: {error}")

    if output is not None:
        if len(GZIP_HEADER) > args.max_compressed_bytes:
            compressed_cap_reached = True
        else:
            try:
                _write_all(output, GZIP_HEADER)
                compressed_stored = len(GZIP_HEADER)
                compressor = zlib.compressobj(
                    level=6, method=zlib.DEFLATED, wbits=-zlib.MAX_WBITS
                )
            except OSError as error:
                errors.append(f"cannot initialize compressed trace: {error}")

    while True:
        try:
            chunk = os.read(sys.stdin.fileno(), CHUNK_BYTES)
        except OSError as error:
            errors.append(f"trace input read failed: {error}")
            input_read_error = True
            break
        if not chunk:
            break
        chunks_read += 1
        raw_observed += len(chunk)

        if output is None or compressor is None or errors or raw_cap_reached or compressed_cap_reached:
            continue

        remaining_raw = args.max_raw_bytes - raw_fed
        piece = chunk[: max(0, remaining_raw)]
        if piece:
            raw_digest.update(piece)
            crc32 = zlib.crc32(piece, crc32)
            raw_fed += len(piece)
            try:
                compressed = compressor.compress(piece)
            except zlib.error as error:
                errors.append(f"gzip compression failed: {error}")
                continue
            if compressed:
                if compressed_stored + len(compressed) > args.max_compressed_bytes:
                    compressed_cap_reached = True
                else:
                    try:
                        _write_all(output, compressed)
                        compressed_stored += len(compressed)
                    except OSError as error:
                        errors.append(f"compressed trace write failed: {error}")

        if raw_observed > args.max_raw_bytes:
            raw_cap_reached = True

    if not input_read_error and not errors and not raw_cap_reached and not compressed_cap_reached and output is not None and compressor is not None:
        try:
            final_block = compressor.flush(zlib.Z_FINISH)
            footer = struct.pack("<II", crc32 & 0xFFFFFFFF, raw_fed & 0xFFFFFFFF)
            final_bytes = final_block + footer
            if compressed_stored + len(final_bytes) > args.max_compressed_bytes:
                compressed_cap_reached = True
            else:
                _write_all(output, final_bytes)
                compressed_stored += len(final_bytes)
                gzip_finalized = True
        except (OSError, zlib.error, struct.error) as error:
            errors.append(f"gzip finalization failed: {error}")

    if output is not None:
        try:
            output.close()
        except OSError as error:
            errors.append(f"compressed trace close failed: {error}")

    try:
        if args.output.exists():
            compressed_stored = args.output.stat().st_size
    except OSError as error:
        errors.append(f"cannot stat compressed trace: {error}")
    compressed_sha256 = _hash_file(args.output) if args.output.exists() else None
    if args.output.exists() and compressed_sha256 is None:
        errors.append("cannot hash compressed trace")

    complete = (
        not input_read_error
        and not errors
        and not raw_cap_reached
        and not compressed_cap_reached
        and gzip_finalized
        and raw_observed == raw_fed
    )
    metadata: dict[str, object] = {
        "format": "gzip",
        "gzip_level": 6,
        "gzip_header_mtime": 0,
        "gzip_header_os": 255,
        "chunk_bytes": CHUNK_BYTES,
        "raw_cap_bytes": args.max_raw_bytes,
        "compressed_cap_bytes": args.max_compressed_bytes,
        "raw_bytes_observed": raw_observed,
        "raw_bytes_fed_to_compressor": raw_fed,
        "raw_bytes_retained": None,
        "compressed_bytes_stored": compressed_stored,
        "chunks_read": chunks_read,
        "raw_cap_reached": raw_cap_reached,
        "compressed_cap_reached": compressed_cap_reached,
        "input_read_error": input_read_error,
        "gzip_finalized": gzip_finalized,
        "capture_complete": complete,
        "raw_sha256_fed_to_compressor": raw_digest.hexdigest(),
        "raw_retained_sha256": None,
        "compressed_sha256": compressed_sha256,
        "errors": errors,
    }

    if not complete:
        try:
            with args.truncated_marker.open("x", encoding="utf-8", newline="\n") as marker:
                json.dump(metadata, marker, sort_keys=True, indent=2)
                marker.write("\n")
        except OSError as error:
            errors.append(f"cannot write incomplete-capture marker: {error}")
            metadata["errors"] = errors

    metadata_error = _write_json_exclusive(args.metadata, metadata)
    if metadata_error is not None:
        print(f"capture metadata write failed: {metadata_error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
    return 0 if complete else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-raw-bytes", type=int, required=True)
    parser.add_argument("--max-compressed-bytes", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--truncated-marker", type=Path, required=True)
    args = parser.parse_args()
    if args.max_raw_bytes <= 0 or args.max_compressed_bytes <= 0:
        parser.error("both byte limits must be positive")
    return capture(args)


if __name__ == "__main__":
    raise SystemExit(main())
