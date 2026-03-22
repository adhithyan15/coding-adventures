"""sha256sum — compute and check SHA-256 message digest.

=== What This Program Does ===

This is a reimplementation of the GNU ``sha256sum`` utility. It computes
or verifies SHA-256 checksums for files.

=== What is SHA-256? ===

SHA-256 (Secure Hash Algorithm 256-bit) is a cryptographic hash function
from the SHA-2 family. It produces a 256-bit (32-byte) hash value,
expressed as a 64-character hexadecimal string. For example::

    $ echo "hello" | sha256sum
    5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03  -

=== SHA-256 vs MD5 ===

SHA-256 is the preferred hash for security applications:

+-----------+----------+-----+---------------------------+
| Algorithm | Bits     | Hex | Status                    |
+-----------+----------+-----+---------------------------+
| MD5       | 128      | 32  | BROKEN (collision attacks)|
| SHA-256   | 256      | 64  | Secure (as of 2024)       |
+-----------+----------+-----+---------------------------+

SHA-256 is slower than MD5 but dramatically more secure. Use SHA-256
for verifying software downloads, checking file integrity in security
contexts, and any application where collision resistance matters.

=== Check Mode (-c) ===

With ``-c``, sha256sum reads a file containing checksums and verifies
each file, just like md5sum::

    $ sha256sum -c checksums.txt
    file1.txt: OK
    file2.txt: FAILED

=== Output Format ===

Same as md5sum: ``hash  filename`` (two spaces in text mode) or
``hash *filename`` (space + asterisk in binary mode).

=== CLI Builder Integration ===

The entire CLI is defined in ``sha256sum.json``. CLI Builder handles
flag parsing, help text, and version output.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "sha256sum.json")


# ---------------------------------------------------------------------------
# Business logic: compute_sha256
# ---------------------------------------------------------------------------


def compute_sha256(filepath: str, *, binary: bool = False) -> str:
    """Compute the SHA-256 hash of a file.

    Reads the file in chunks to handle large files efficiently.

    Args:
        filepath: Path to the file to hash.
        binary: If True, open in binary mode (on Unix, no difference).

    Returns:
        The 64-character lowercase hexadecimal SHA-256 hash.

    Raises:
        FileNotFoundError: If the file does not exist.
        OSError: If the file cannot be read.
    """
    sha = hashlib.sha256()

    with open(filepath, "rb") as f:
        while True:
            chunk = f.read(8192)
            if not chunk:
                break
            sha.update(chunk)

    return sha.hexdigest()


def compute_sha256_stdin(*, binary: bool = False) -> str:
    """Compute the SHA-256 hash of standard input.

    Args:
        binary: Unused on Unix, but kept for API consistency.

    Returns:
        The 64-character lowercase hexadecimal SHA-256 hash.
    """
    sha = hashlib.sha256()
    stdin_bytes = sys.stdin.buffer.read()
    sha.update(stdin_bytes)
    return sha.hexdigest()


def format_checksum_line(
    digest: str,
    filename: str,
    *,
    binary: bool = False,
) -> str:
    """Format a checksum output line.

    The standard format is: ``hash  filename`` (two spaces).
    In binary mode: ``hash *filename`` (space + asterisk).

    Args:
        digest: The hex digest string.
        filename: The filename to display.
        binary: If True, use binary mode indicator.

    Returns:
        The formatted checksum line.
    """
    if binary:
        return f"{digest} *{filename}"
    return f"{digest}  {filename}"


def check_checksums(
    checkfile: str,
    *,
    quiet: bool = False,
    status: bool = False,
    strict: bool = False,
    warn: bool = False,
) -> tuple[int, int]:
    """Verify checksums from a checksum file.

    Reads a file containing lines of the format ``HASH  FILENAME``
    and verifies each file against its expected hash.

    Args:
        checkfile: Path to the checksum file.
        quiet: If True, don't print OK for successful verifications.
        status: If True, produce no output (use exit code only).
        strict: If True, return failure for improperly formatted lines.
        warn: If True, warn about improperly formatted lines.

    Returns:
        A tuple of (failures, total_checked).
    """
    failures = 0
    checked = 0
    format_errors = 0

    with open(checkfile) as f:
        for line in f:
            line = line.rstrip("\n\r")
            if not line:
                continue

            # Parse: HASH  FILENAME or HASH *FILENAME
            parts = line.split(None, 1)
            if len(parts) != 2:  # noqa: PLR2004
                format_errors += 1
                if warn:
                    print(
                        f"sha256sum: {checkfile}: improperly formatted line",
                        file=sys.stderr,
                    )
                continue

            expected_hash = parts[0]
            filename = parts[1]
            if filename.startswith(("*", " ")):
                filename = filename[1:]

            try:
                actual_hash = compute_sha256(filename)
            except FileNotFoundError:
                if not status:
                    print(f"{filename}: FAILED open or read")
                failures += 1
                checked += 1
                continue
            except OSError:
                if not status:
                    print(f"{filename}: FAILED open or read")
                failures += 1
                checked += 1
                continue

            checked += 1
            if actual_hash == expected_hash:
                if not quiet and not status:
                    print(f"{filename}: OK")
            else:
                if not status:
                    print(f"{filename}: FAILED")
                failures += 1

    if strict and format_errors > 0:
        failures += format_errors

    return failures, checked


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then compute/check checksums."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"sha256sum: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    check_mode = result.flags.get("check", False)
    binary = result.flags.get("binary", False)
    quiet = result.flags.get("quiet", False)
    status_flag = result.flags.get("status", False)
    strict = result.flags.get("strict", False)
    warn_flag = result.flags.get("warn", False)

    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]

    if check_mode:
        total_failures = 0
        for fname in files:
            try:
                failures, _ = check_checksums(
                    fname,
                    quiet=quiet,
                    status=status_flag,
                    strict=strict,
                    warn=warn_flag,
                )
                total_failures += failures
            except FileNotFoundError:
                print(
                    f"sha256sum: {fname}: No such file or directory",
                    file=sys.stderr,
                )
                total_failures += 1
        if total_failures > 0:
            raise SystemExit(1)
    else:
        try:
            for fname in files:
                if fname == "-":
                    digest = compute_sha256_stdin(binary=binary)
                    print(format_checksum_line(digest, "-", binary=binary))
                else:
                    try:
                        digest = compute_sha256(fname, binary=binary)
                        print(format_checksum_line(digest, fname, binary=binary))
                    except FileNotFoundError:
                        print(
                            f"sha256sum: {fname}: No such file or directory",
                            file=sys.stderr,
                        )
                        raise SystemExit(1) from None
        except BrokenPipeError:
            raise SystemExit(0) from None


if __name__ == "__main__":
    main()
