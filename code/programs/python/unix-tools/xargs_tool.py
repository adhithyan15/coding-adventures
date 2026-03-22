"""xargs -- build and execute command lines from standard input.

=== What This Program Does ===

This is a reimplementation of the GNU ``xargs`` utility. It reads items
from standard input (separated by whitespace or newlines), and executes
a command with those items as arguments::

    find . -name "*.txt" | xargs grep "hello"
    echo "a b c" | xargs -n 1 echo        # One arg per invocation
    find . -print0 | xargs -0 rm           # Handle filenames with spaces

=== Why xargs Exists ===

Shell commands have a maximum argument length (``ARG_MAX``). If you try
to pass too many arguments, you get "Argument list too long". ``xargs``
solves this by splitting the input into manageable batches and running
the command multiple times if needed.

=== Input Parsing ===

By default, ``xargs`` splits input on whitespace and respects quoting
(single and double quotes, backslash escapes). Special modes change
the delimiter:

- ``-0`` / ``--null``: Split on null bytes (``\\0``). This is essential
  for handling filenames that contain spaces, quotes, or newlines.
- ``-d DELIM``: Split on a custom single-character delimiter.

=== Batching ===

By default, ``xargs`` puts as many items as possible into each command
invocation. You can limit this:

- ``-n MAX``: At most MAX arguments per invocation.
- ``-L MAX``: At most MAX input lines per invocation.
- ``-I STR``: Replace STR in the command with each input item (implies
  ``-n 1``). For example::

    echo -e "a\\nb\\nc" | xargs -I {} echo "File: {}"

=== Parallelism ===

- ``-P N``: Run up to N processes in parallel. When N is 0, xargs runs
  as many as possible. This is useful for CPU-intensive operations::

    find . -name "*.jpg" | xargs -P 4 -n 1 convert -resize 800x600

=== Safety ===

- ``-r`` / ``--no-run-if-empty``: Don't run the command if the input
  is empty. By default, xargs runs the command once even with no input.
- ``-t`` / ``--verbose``: Print each command to stderr before running it.
- ``-p`` / ``--interactive``: Prompt the user before each command.

=== CLI Builder Integration ===

The CLI is defined in ``xargs.json``. CLI Builder handles flag parsing.
"""

from __future__ import annotations

import shlex
import subprocess
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "xargs.json")


# ---------------------------------------------------------------------------
# Input parsing -- split input into items.
# ---------------------------------------------------------------------------


def parse_items(
    input_text: str,
    *,
    null_delimiter: bool = False,
    delimiter: str | None = None,
    eof_str: str | None = None,
) -> list[str]:
    """Parse input text into a list of items.

    The parsing behavior depends on the delimiter mode:

    +------------------+--------------------------------------------------+
    | Mode             | Behavior                                         |
    +==================+==================================================+
    | Default          | Split on whitespace, respecting quotes/escapes   |
    | ``-0`` (null)    | Split on null bytes, no quote processing          |
    | ``-d DELIM``     | Split on custom delimiter, no quote processing    |
    +------------------+--------------------------------------------------+

    If ``eof_str`` is set, input processing stops when a line matching
    the EOF string is encountered.

    Args:
        input_text: The raw input text.
        null_delimiter: If True, split on null bytes.
        delimiter: Custom delimiter character.
        eof_str: Stop processing when this string is encountered.

    Returns:
        List of parsed items.
    """
    if null_delimiter:
        # Split on null bytes, filter out empty strings.
        items = input_text.split("\0")
        items = [item for item in items if item]
    elif delimiter is not None:
        # Split on custom delimiter.
        items = input_text.split(delimiter)
        items = [item for item in items if item]
    else:
        # Default: use shell-like splitting (handles quotes and escapes).
        try:
            items = shlex.split(input_text)
        except ValueError:
            # If parsing fails (e.g., unmatched quotes), fall back to
            # simple whitespace splitting.
            items = input_text.split()

    # Apply EOF string filter.
    if eof_str is not None:
        filtered: list[str] = []
        for item in items:
            if item == eof_str:
                break
            filtered.append(item)
        return filtered

    return items


# ---------------------------------------------------------------------------
# Command execution -- run a command with arguments.
# ---------------------------------------------------------------------------


def run_command(
    command: list[str],
    args: list[str],
    *,
    replace_str: str | None = None,
    verbose: bool = False,
) -> int:
    """Execute a command with the given arguments.

    If ``replace_str`` is set, each occurrence of replace_str in the
    command template is replaced with the concatenated arguments.
    Otherwise, args are appended to the command.

    Args:
        command: The command and its initial arguments.
        args: Additional arguments from stdin.
        replace_str: String to replace in command with args.
        verbose: If True, print the command to stderr.

    Returns:
        The exit code of the executed command.
    """
    if replace_str is not None:
        # Replace mode: substitute replace_str with each arg.
        joined = " ".join(args)
        full_cmd = [part.replace(replace_str, joined) for part in command]
    else:
        # Append mode: add args after the command.
        full_cmd = command + args

    if verbose:
        print(" ".join(full_cmd), file=sys.stderr)

    try:
        result = subprocess.run(full_cmd, check=False)
        return result.returncode
    except FileNotFoundError:
        print(f"xargs: {full_cmd[0]}: No such file or directory", file=sys.stderr)
        return 127
    except PermissionError:
        print(f"xargs: {full_cmd[0]}: Permission denied", file=sys.stderr)
        return 126


# ---------------------------------------------------------------------------
# Batching -- split items into groups and execute.
# ---------------------------------------------------------------------------


def execute_batches(
    command: list[str],
    items: list[str],
    *,
    max_args: int | None = None,
    replace_str: str | None = None,
    verbose: bool = False,
    no_run_if_empty: bool = False,
    max_procs: int = 1,
) -> int:
    """Split items into batches and execute the command for each batch.

    This is the main orchestration function. It groups items according
    to ``max_args`` and runs the command for each group.

    Args:
        command: The command template.
        items: All items parsed from stdin.
        max_args: Maximum arguments per invocation.
        replace_str: Replacement string for ``-I`` mode.
        verbose: Print commands before executing.
        no_run_if_empty: Skip execution if no items.
        max_procs: Maximum parallel processes.

    Returns:
        The worst (highest) exit code from all invocations.

    Batching logic
    ~~~~~~~~~~~~~~

    When ``-I`` is used, each item is processed separately (implies
    ``-n 1``). When ``-n`` is used, items are grouped into batches
    of that size. Otherwise, all items go in a single batch.
    """
    if not items:
        if no_run_if_empty:
            return 0
        # Run command once with no extra args.
        return run_command(command, [], verbose=verbose)

    # Determine batch size.
    if replace_str is not None:
        batch_size = 1
    elif max_args is not None:
        batch_size = max_args
    else:
        batch_size = len(items)

    # Split into batches.
    batches: list[list[str]] = []
    for i in range(0, len(items), batch_size):
        batches.append(items[i : i + batch_size])

    # Execute batches.
    worst_exit = 0

    if max_procs != 1:
        # Parallel execution using subprocess directly.
        import concurrent.futures

        actual_procs = max_procs if max_procs > 0 else len(batches)

        with concurrent.futures.ThreadPoolExecutor(max_workers=actual_procs) as pool:
            futures = []
            for batch in batches:
                fut = pool.submit(
                    run_command,
                    command,
                    batch,
                    replace_str=replace_str,
                    verbose=verbose,
                )
                futures.append(fut)

            for fut in concurrent.futures.as_completed(futures):
                code = fut.result()
                if code > worst_exit:
                    worst_exit = code
    else:
        # Sequential execution.
        for batch in batches:
            code = run_command(
                command,
                batch,
                replace_str=replace_str,
                verbose=verbose,
            )
            if code > worst_exit:
                worst_exit = code

    return worst_exit


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then run xargs."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"xargs: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    null_delim = result.flags.get("null", False)
    delimiter = result.flags.get("delimiter", None)
    eof_str = result.flags.get("eof", None)
    replace_str = result.flags.get("replace", None)
    max_args = result.flags.get("max_args", None)
    max_procs = result.flags.get("max_procs", 1)
    verbose = result.flags.get("verbose", False)
    no_run_if_empty = result.flags.get("no_run_if_empty", False)
    arg_file = result.flags.get("arg_file", None)

    # --- Determine the command ---
    command_args = result.arguments.get("command", None)
    if command_args is None or command_args == []:
        command = ["/bin/echo"]
    elif isinstance(command_args, str):
        command = [command_args]
    else:
        command = list(command_args)

    # --- Read input ---
    try:
        if arg_file:
            with open(arg_file) as f:
                input_text = f.read()
        else:
            input_text = sys.stdin.read()
    except FileNotFoundError:
        print(f"xargs: {arg_file}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Parse items ---
    items = parse_items(
        input_text,
        null_delimiter=null_delim,
        delimiter=delimiter,
        eof_str=eof_str,
    )

    # --- Execute ---
    exit_code = execute_batches(
        command,
        items,
        max_args=max_args,
        replace_str=replace_str,
        verbose=verbose,
        no_run_if_empty=no_run_if_empty,
        max_procs=max_procs,
    )

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
