"""tail — output the last part of files.

=== What This Program Does ===

This is a reimplementation of the GNU ``tail`` utility. It reads one or more
files (or standard input) and prints the last N lines (default 10) or the
last N bytes to standard output.

=== The +N Syntax ===

What makes tail interesting compared to head is the ``+N`` syntax:

- ``tail -n 5 file``  — print the last 5 lines
- ``tail -n +5 file`` — print starting from line 5 (1-indexed)

The ``+`` prefix changes the semantics completely. Instead of "last N",
it means "starting from line N". This is extremely useful for skipping
headers::

    $ tail -n +2 data.csv   # Skip the header row

=== Lines vs Bytes ===

Like head, tail supports both line-based (``-n``) and byte-based (``-c``)
modes. The ``+N`` syntax works for both.

=== Follow Mode (-f) ===

The ``-f`` flag makes tail wait for new data to be appended to the file.
This is commonly used for monitoring log files::

    $ tail -f /var/log/syslog

Our implementation supports a basic version of follow mode. The ``--retry``
flag is accepted but treated as a no-op in this implementation.

=== CLI Builder Integration ===

The entire CLI is defined in ``tail.json``. Note that ``-n`` and ``-c``
use string type (not integer) because they need to support the ``+N``
prefix syntax. We parse the numeric value ourselves in the business logic.
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "tail.json")


def parse_count(value: str) -> tuple[int, bool]:
    """Parse a tail count value, handling the +N syntax.

    Tail's ``-n`` and ``-c`` flags accept values like:
    - ``"10"``  — last 10 (from_start=False)
    - ``"+10"`` — starting from position 10 (from_start=True)
    - ``"-10"`` — last 10 (same as ``"10"``, from_start=False)

    Args:
        value: The raw string value from the command line.

    Returns:
        A tuple of (count, from_start). ``from_start`` is True when the
        value had a ``+`` prefix, meaning "start from this position."
    """
    if value.startswith("+"):
        return int(value[1:]), True
    if value.startswith("-"):
        return int(value[1:]), False
    return int(value), False


def read_input_lines(filename: str) -> list[str]:
    """Read all lines from a file or stdin.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        A list of lines with newline characters preserved.

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            content = sys.stdin.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        return content.splitlines(keepends=True) if content else []

    try:
        with open(filename) as f:
            return f.readlines()
    except FileNotFoundError:
        print(f"tail: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"tail: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"tail: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def read_input_bytes(filename: str) -> bytes:
    """Read all bytes from a file or stdin.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        The raw bytes of the file content.

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            return sys.stdin.buffer.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None

    try:
        return Path(filename).read_bytes()
    except FileNotFoundError:
        print(f"tail: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"tail: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"tail: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def tail_lines(lines: list[str], count: int, *, from_start: bool) -> str:
    """Extract the requested lines from the input.

    Two modes of operation:

    1. **from_start=False** (default): Return the last ``count`` lines.
       This is the classic tail behavior: ``tail -n 5`` gives you the
       last 5 lines.

    2. **from_start=True**: Return all lines starting from position
       ``count`` (1-indexed). ``tail -n +1`` gives you the entire file.
       ``tail -n +2`` skips the first line.

    Args:
        lines: All lines from the input.
        count: The count value.
        from_start: Whether to start from this position (True) or take
            the last N (False).

    Returns:
        The concatenation of the selected lines.
    """
    if from_start:
        # +N means "start from line N" (1-indexed).
        # +1 = entire file, +2 = skip first line, etc.
        return "".join(lines[count - 1 :])
    # Last N lines.
    return "".join(lines[-count:]) if count > 0 else ""


def tail_bytes(data: bytes, count: int, *, from_start: bool) -> bytes:
    """Extract the requested bytes from the input.

    Args:
        data: The raw input bytes.
        count: The count value.
        from_start: Whether to start from this position or take the last N.

    Returns:
        The selected bytes.
    """
    if from_start:
        return data[count - 1 :]
    return data[-count:] if count > 0 else b""


def should_print_header(
    *,
    num_files: int,
    quiet: bool,
    verbose: bool,
) -> bool:
    """Determine whether to print the ``==> filename <==`` header.

    Same rules as head: quiet suppresses, verbose forces, otherwise
    headers appear only for multiple files.
    """
    if quiet:
        return False
    if verbose:
        return True
    return num_files > 1


def main() -> None:
    """Entry point: parse args via CLI Builder, then output last lines/bytes."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"tail: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    assert isinstance(result, ParseResult)

    # Extract flags.
    lines_str = result.flags.get("lines", "10")
    bytes_str = result.flags.get("bytes")
    follow = result.flags.get("follow", False)
    quiet = result.flags.get("quiet", False)
    verbose = result.flags.get("verbose", False)
    use_bytes = bytes_str is not None

    if use_bytes:
        count, from_start = parse_count(str(bytes_str))
    else:
        count, from_start = parse_count(str(lines_str))

    # Get the list of files.
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    print_header = should_print_header(
        num_files=len(files), quiet=quiet, verbose=verbose
    )

    for i, filename in enumerate(files):
        if print_header:
            if i > 0:
                print()
            display_name = "standard input" if filename == "-" else filename
            print(f"==> {display_name} <==")

        if use_bytes:
            data = read_input_bytes(filename)
            sys.stdout.buffer.write(tail_bytes(data, count, from_start=from_start))
            sys.stdout.buffer.flush()
        else:
            lines = read_input_lines(filename)
            sys.stdout.write(tail_lines(lines, count, from_start=from_start))

    # --- Follow mode (-f) --------------------------------------------------
    # Basic follow: poll the last file for new content every second.
    if follow and files[-1] != "-":
        last_file = files[-1]
        try:
            last_size = Path(last_file).stat().st_size
            while True:
                time.sleep(1)
                current_size = Path(last_file).stat().st_size
                if current_size > last_size:
                    with open(last_file, "rb") as f:
                        f.seek(last_size)
                        new_data = f.read()
                    sys.stdout.buffer.write(new_data)
                    sys.stdout.buffer.flush()
                    last_size = current_size
        except KeyboardInterrupt:
            raise SystemExit(0) from None
        except FileNotFoundError:
            pass


if __name__ == "__main__":
    main()
