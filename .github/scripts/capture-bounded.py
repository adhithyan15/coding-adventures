#!/usr/bin/env python3
"""Save at most a fixed number of bytes while draining the full input stream."""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-bytes", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--truncated-marker", type=Path, required=True)
    args = parser.parse_args()
    if args.max_bytes <= 0:
        parser.error("--max-bytes must be positive")

    total = 0
    saved = 0
    capture_error: str | None = None
    output = None
    try:
        output = args.output.open("wb")
    except OSError as error:
        capture_error = str(error)

    while True:
        try:
            # os.read returns currently available pipe data rather than
            # buffering across separate FIFO writers while the keeper FD is
            # intentionally suppressing EOF between test invocations.
            chunk = os.read(sys.stdin.fileno(), 64 * 1024)
        except OSError as error:
            capture_error = str(error)
            break
        if not chunk:
            break
        total += len(chunk)
        remaining = args.max_bytes - saved
        if remaining > 0 and output is not None:
            piece = chunk[:remaining]
            try:
                written = output.write(piece)
                if written != len(piece):
                    raise OSError(f"short write: {written} of {len(piece)} bytes")
                saved += written
            except OSError as error:
                capture_error = str(error)
                try:
                    output.close()
                except OSError as close_error:
                    capture_error = f"{capture_error}; close failed: {close_error}"
                output = None

    if output is not None:
        try:
            output.close()
        except OSError as error:
            capture_error = str(error)

    if total > args.max_bytes or capture_error is not None:
        details = [
            f"captured={saved}",
            f"observed_total={total}",
            f"limit={args.max_bytes}",
        ]
        if capture_error is not None:
            details.append(f"capture_error={capture_error}")
        try:
            args.truncated_marker.write_text("\n".join(details) + "\n", encoding="utf-8")
        except OSError as error:
            print(f"bounded capture marker failed after draining input: {error}", file=sys.stderr)
            return 1
    if capture_error is not None:
        print(f"bounded capture failed after draining input: {capture_error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
