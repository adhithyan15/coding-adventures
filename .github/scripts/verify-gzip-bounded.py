#!/usr/bin/env python3
"""Verify a complete bounded gzip trace and its diagnostic markers."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import sys
import zlib
from pathlib import Path

CHUNK_BYTES = 64 * 1024
MAX_LINE_BYTES = 64 * 1024
RUN_MARKER = re.compile(rb"^=== outer strace invocation pid=[0-9]+ ===$")
PTRACE_CALL = re.compile(rb"\bptrace\(PTRACE_[A-Z0-9_]+")
LISTEN_EIO = re.compile(rb"ptrace\(PTRACE_LISTEN,.*\)\s*=\s*-1\s+EIO\b")


def _hash_file(path: Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        while chunk := stream.read(CHUNK_BYTES):
            digest.update(chunk)
            size += len(chunk)
    return digest.hexdigest(), size


def verify(args: argparse.Namespace) -> int:
    result: dict[str, object] = {
        "valid": False,
        "gzip_integrity_verified": False,
        "errors": [],
        "raw_bytes_decompressed": 0,
        "raw_bytes_retained": None,
        "raw_retained_sha256": None,
        "compressed_bytes_verified": 0,
        "outer_invocation_markers": 0,
        "ptrace_calls": 0,
        "ptrace_listen_eio_calls": 0,
    }
    errors: list[str] = []
    result["errors"] = errors

    try:
        capture = json.loads(args.capture_metadata.read_text(encoding="utf-8"))
        if not isinstance(capture, dict):
            raise TypeError("capture metadata root is not an object")
        compressed_hash, compressed_size = _hash_file(args.trace)
        result["compressed_bytes_verified"] = compressed_size
        if capture.get("capture_complete") is not True:
            errors.append("capture metadata does not mark a complete capture")
        if capture.get("gzip_finalized") is not True:
            errors.append("capture metadata does not mark a finalized gzip member")
        if capture.get("errors") != []:
            errors.append("capture metadata reports errors")
        if capture.get("raw_cap_reached") or capture.get("compressed_cap_reached"):
            errors.append("capture metadata reports a reached cap")
        if capture.get("raw_bytes_observed") != capture.get("raw_bytes_fed_to_compressor"):
            errors.append("observed and compressed raw byte counts differ")
        if capture.get("raw_bytes_retained") is not None:
            errors.append("capture metadata claims raw bytes are retained before verification")
        if capture.get("raw_retained_sha256") is not None:
            errors.append("capture metadata claims a retained digest before verification")
        if compressed_size != capture.get("compressed_bytes_stored"):
            errors.append("compressed byte count differs from capture metadata")
        if compressed_hash != capture.get("compressed_sha256"):
            errors.append("compressed SHA-256 differs from capture metadata")
        metadata_raw_limit = capture.get("raw_cap_bytes")
        if type(metadata_raw_limit) is not int or metadata_raw_limit <= 0:
            errors.append("invalid raw byte ceiling")
            raw_limit = args.max_raw_bytes
        else:
            raw_limit = min(args.max_raw_bytes, metadata_raw_limit)
        compressed_limit = capture.get("compressed_cap_bytes")
        if type(compressed_limit) is not int or compressed_limit <= 0:
            errors.append("invalid compressed byte ceiling")
            compressed_limit = 0
        else:
            if compressed_limit > args.max_compressed_bytes:
                errors.append("capture metadata compressed ceiling exceeds verifier ceiling")
            if compressed_size > min(compressed_limit, args.max_compressed_bytes):
                errors.append("compressed trace exceeds an independent byte ceiling")

        raw_hash = hashlib.sha256()
        raw_count = 0
        line = bytearray()
        line_too_long = False
        markers = 0
        ptrace_calls = 0
        listen_eio_calls = 0
        decompression_finished = False

        with args.trace.open("rb") as compressed, gzip.GzipFile(fileobj=compressed, mode="rb") as stream:
            while True:
                # Read at most one byte past the ceiling to detect expansion bombs.
                allowance = raw_limit - raw_count
                chunk = stream.read(min(CHUNK_BYTES, allowance + 1))
                if not chunk:
                    decompression_finished = True
                    break
                if raw_count + len(chunk) > raw_limit:
                    errors.append("decompressed trace exceeds the configured raw byte ceiling")
                    break
                raw_count += len(chunk)
                raw_hash.update(chunk)
                for byte in chunk:
                    if line_too_long:
                        if byte == 0x0A:
                            line_too_long = False
                        continue
                    if byte == 0x0A:
                        current = bytes(line)
                        if RUN_MARKER.fullmatch(current):
                            markers += 1
                        ptrace_calls += len(PTRACE_CALL.findall(current))
                        listen_eio_calls += len(LISTEN_EIO.findall(current))
                        line.clear()
                    else:
                        line.append(byte)
                        if len(line) > MAX_LINE_BYTES:
                            errors.append("trace contains a line over the verification limit")
                            line.clear()
                            line_too_long = True

        if line or line_too_long:
            errors.append("trace ends with an unterminated line")
        result["raw_bytes_decompressed"] = raw_count
        result["outer_invocation_markers"] = markers
        result["ptrace_calls"] = ptrace_calls
        result["ptrace_listen_eio_calls"] = listen_eio_calls
        if raw_count != capture.get("raw_bytes_fed_to_compressor"):
            errors.append("decompressed byte count differs from capture metadata")
        if raw_hash.hexdigest() != capture.get("raw_sha256_fed_to_compressor"):
            errors.append("decompressed SHA-256 differs from capture metadata")
        if markers != args.expected_runs:
            errors.append(f"expected {args.expected_runs} outer invocation markers, found {markers}")
        if ptrace_calls == 0:
            errors.append("trace contains no ptrace requests")
        gzip_integrity_verified = (
            decompression_finished
            and capture.get("capture_complete") is True
            and capture.get("gzip_finalized") is True
            and capture.get("raw_cap_reached") is False
            and capture.get("compressed_cap_reached") is False
            and capture.get("errors") == []
            and capture.get("raw_bytes_retained") is None
            and capture.get("raw_retained_sha256") is None
            and raw_count == capture.get("raw_bytes_fed_to_compressor")
            and raw_hash.hexdigest() == capture.get("raw_sha256_fed_to_compressor")
            and compressed_size == capture.get("compressed_bytes_stored")
            and compressed_hash == capture.get("compressed_sha256")
            and raw_count <= raw_limit
            and compressed_size <= args.max_compressed_bytes
            and compressed_limit <= args.max_compressed_bytes
            and compressed_size <= compressed_limit
        )
        result["gzip_integrity_verified"] = gzip_integrity_verified
        if gzip_integrity_verified:
            result["raw_bytes_retained"] = raw_count
            result["raw_retained_sha256"] = raw_hash.hexdigest()
    except (OSError, EOFError, gzip.BadGzipFile, json.JSONDecodeError, TypeError, ValueError, zlib.error) as error:
        errors.append(f"trace verification failed: {error}")

    result["valid"] = not errors
    try:
        with args.output.open("x", encoding="utf-8", newline="\n") as output:
            json.dump(result, output, sort_keys=True, indent=2)
            output.write("\n")
    except OSError as error:
        print(f"verification report write failed: {error}", file=sys.stderr)
        return 2
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
    return 0 if not errors else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--capture-metadata", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-raw-bytes", type=int, required=True)
    parser.add_argument("--max-compressed-bytes", type=int, required=True)
    parser.add_argument("--expected-runs", type=int, default=5)
    args = parser.parse_args()
    if args.max_raw_bytes <= 0 or args.max_compressed_bytes <= 0 or args.expected_runs <= 0:
        parser.error("byte limits and expected run count must be positive")
    return verify(args)


if __name__ == "__main__":
    raise SystemExit(main())
