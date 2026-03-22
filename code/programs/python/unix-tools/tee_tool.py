"""tee — read from standard input and write to standard output and files.

=== What This Program Does ===

This is a reimplementation of the GNU ``tee`` utility. It reads from
standard input and writes to both standard output *and* one or more files
simultaneously. The name comes from plumbing: a T-shaped pipe fitting
that splits a flow into two directions.

=== Why tee Exists ===

In Unix pipelines, data flows in one direction. Sometimes you want to
both see the data and save it::

    $ make 2>&1 | tee build.log

This runs ``make``, shows the output on screen, and simultaneously saves
it to ``build.log``. Without ``tee``, you'd have to choose between seeing
the output and saving it.

=== Append Mode (-a) ===

By default, tee overwrites the output files. The ``-a`` flag switches to
append mode, which adds to the end of existing files instead::

    $ echo "run 1" | tee results.log
    $ echo "run 2" | tee -a results.log

After this, ``results.log`` contains both "run 1" and "run 2".

=== Signal Handling (-i) ===

The ``-i`` flag tells tee to ignore the SIGINT signal (Ctrl+C). This is
useful in long-running pipelines where you want tee to keep writing even
if the user accidentally presses Ctrl+C.

=== CLI Builder Integration ===

The entire CLI is defined in ``tee.json``. CLI Builder handles flag parsing,
help text, and version output. This file implements the I/O multiplexing
logic.
"""

from __future__ import annotations

import signal
import sys
from pathlib import Path
from typing import IO

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "tee.json")


def tee_copy(
    input_stream: IO[bytes],
    output_files: list[IO[bytes]],
) -> None:
    """Copy bytes from input to stdout and all output files.

    This function reads from the input stream in chunks and writes each
    chunk to standard output and every open output file. Using chunks
    (rather than reading the entire input at once) keeps memory usage
    bounded for large inputs.

    The chunk size of 8192 bytes is a common choice that balances
    system call overhead against memory usage.

    Args:
        input_stream: The input byte stream (typically stdin).
        output_files: List of open file objects to write to.
    """
    while True:
        chunk = input_stream.read(8192)
        if not chunk:
            break

        # Write to stdout.
        sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.flush()

        # Write to each output file.
        for f in output_files:
            f.write(chunk)
            f.flush()


def main() -> None:
    """Entry point: parse args via CLI Builder, then tee stdin to files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"tee: {error.message}", file=sys.stderr)
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

    append = result.flags.get("append", False)
    ignore_interrupts = result.flags.get("ignore_interrupts", False)

    # Get the list of output files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]
    if files is None:
        files = []

    # Handle SIGINT if requested.
    if ignore_interrupts:
        signal.signal(signal.SIGINT, signal.SIG_IGN)

    # Open all output files.
    mode = "ab" if append else "wb"
    open_files: list[IO[bytes]] = []

    try:
        for filename in files:
            try:
                open_files.append(open(filename, mode))  # noqa: SIM115
            except PermissionError:
                print(f"tee: {filename}: Permission denied", file=sys.stderr)
            except IsADirectoryError:
                print(f"tee: {filename}: Is a directory", file=sys.stderr)

        # Copy stdin to stdout and all files.
        try:
            tee_copy(sys.stdin.buffer, open_files)
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        except BrokenPipeError:
            raise SystemExit(0) from None
    finally:
        # Close all output files.
        for f in open_files:
            f.close()


if __name__ == "__main__":
    main()
