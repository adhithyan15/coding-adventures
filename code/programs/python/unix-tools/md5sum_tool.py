"""md5sum — compute and check MD5 message digest.

=== What This Program Does ===

This is a reimplementation of the GNU ``md5sum`` utility. It computes
or verifies MD5 checksums for files.

=== What is MD5? ===

MD5 (Message Digest Algorithm 5) is a cryptographic hash function that
produces a 128-bit (16-byte) hash value, typically expressed as a
32-character hexadecimal string. For example::

    $ echo "hello" | md5sum
    b1946ac92492d2347c6235b4d2611184  -

=== Security Warning ===

MD5 is **cryptographically broken** — it is vulnerable to collision
attacks. Do NOT use MD5 for security purposes (like verifying software
downloads from untrusted sources). Use SHA-256 or better instead.

MD5 is still useful for:
- Verifying data integrity (accidental corruption detection).
- Fast checksums for non-security applications.
- Legacy compatibility with existing checksum files.

=== Check Mode (-c) ===

With ``-c``, md5sum reads a file containing checksums (one per line
in the format ``HASH  FILENAME``) and verifies each file::

    $ md5sum -c checksums.txt
    file1.txt: OK
    file2.txt: FAILED

=== Output Format ===

The output format is: ``hash  filename`` (two spaces between hash and
filename). In binary mode (``-b``), an asterisk replaces the first
space: ``hash *filename``.

=== CLI Builder Integration ===

The entire CLI is defined in ``md5sum.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "md5sum.json")


# ---------------------------------------------------------------------------
# Business logic: compute_md5
# ---------------------------------------------------------------------------


def compute_md5(filepath: str, *, binary: bool = False) -> str:
    """Compute the MD5 hash of a file.

    Reads the file in chunks to handle large files efficiently without
    loading the entire file into memory.

    Args:
        filepath: Path to the file to hash.
        binary: If True, open the file in binary mode (matters on
                Windows where text mode translates line endings).

    Returns:
        The 32-character lowercase hexadecimal MD5 hash.

    Raises:
        FileNotFoundError: If the file does not exist.
        OSError: If the file cannot be read.
    """
    md5 = hashlib.md5()  # noqa: S324

    # We always read in binary mode for consistent hashing.
    # The -b/-t flags only affect the output format indicator,
    # not the actual reading (on Unix, there's no difference).
    with open(filepath, "rb") as f:
        while True:
            chunk = f.read(8192)
            if not chunk:
                break
            md5.update(chunk)

    return md5.hexdigest()


def compute_md5_stdin(*, binary: bool = False) -> str:
    """Compute the MD5 hash of standard input.

    Reads stdin as bytes and computes the hash.

    Args:
        binary: Unused on Unix, but kept for API consistency.

    Returns:
        The 32-character lowercase hexadecimal MD5 hash.
    """
    md5 = hashlib.md5()  # noqa: S324
    stdin_bytes = sys.stdin.buffer.read()
    md5.update(stdin_bytes)
    return md5.hexdigest()


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
    (or ``HASH *FILENAME`` for binary mode) and verifies each file.

    Args:
        checkfile: Path to the checksum file.
        quiet: If True, don't print OK for successful verifications.
        status: If True, don't print any output (use return code only).
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

            # Parse the checksum line.
            # Format: HASH  FILENAME or HASH *FILENAME
            parts = line.split(None, 1)
            if len(parts) != 2:  # noqa: PLR2004
                format_errors += 1
                if warn:
                    print(
                        f"md5sum: {checkfile}: improperly formatted line",
                        file=sys.stderr,
                    )
                continue

            expected_hash = parts[0]
            filename = parts[1]
            # Remove leading * or space from filename.
            if filename.startswith(("*", " ")):
                filename = filename[1:]

            try:
                actual_hash = compute_md5(filename)
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
            print(f"md5sum: {error.message}", file=sys.stderr)
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
                    f"md5sum: {fname}: No such file or directory",
                    file=sys.stderr,
                )
                total_failures += 1
        if total_failures > 0:
            raise SystemExit(1)
    else:
        try:
            for fname in files:
                if fname == "-":
                    digest = compute_md5_stdin(binary=binary)
                    print(format_checksum_line(digest, "-", binary=binary))
                else:
                    try:
                        digest = compute_md5(fname, binary=binary)
                        print(format_checksum_line(digest, fname, binary=binary))
                    except FileNotFoundError:
                        print(
                            f"md5sum: {fname}: No such file or directory",
                            file=sys.stderr,
                        )
                        raise SystemExit(1) from None
        except BrokenPipeError:
            raise SystemExit(0) from None


if __name__ == "__main__":
    main()
