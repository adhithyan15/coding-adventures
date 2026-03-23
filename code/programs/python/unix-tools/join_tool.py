"""join -- join lines of two files on a common field.

=== What This Program Does ===

This is a reimplementation of the GNU ``join`` utility. It performs a
relational join operation on two sorted text files, similar to a SQL
``JOIN`` on a shared key column.

=== How Join Works ===

Given two files sorted on a common field (by default, the first field),
``join`` outputs lines where the join field matches.

Example::

    File 1 (students.txt):      File 2 (grades.txt):
    1 Alice                     1 A+
    2 Bob                       2 B
    3 Charlie                   4 C

    $ join students.txt grades.txt
    1 Alice A+
    2 Bob B

Notice that "3 Charlie" and "4 C" are omitted because their keys don't
appear in both files.

=== The Merge-Join Algorithm ===

Since both files must be sorted on the join field, we can use an
efficient merge-join algorithm (like merge sort's merge step):

1. Read one line from each file.
2. Compare their join fields.
3. If equal: output the joined line, advance both.
4. If file1's key < file2's key: advance file1.
5. If file1's key > file2's key: advance file2.

This runs in O(n + m) time where n and m are the file sizes.

However, the real ``join`` is more complex because multiple consecutive
lines can share the same key. In that case, it produces the Cartesian
product of all lines with that key.

=== Unpaired Lines (-a and -v) ===

- ``-a 1``: Also output unpairable lines from file 1.
- ``-a 2``: Also output unpairable lines from file 2.
- ``-v 1``: Output *only* unpairable lines from file 1.
- ``-v 2``: Output *only* unpairable lines from file 2.

=== Output Format (-o) ===

By default, ``join`` outputs the join field followed by remaining
fields from file 1, then remaining fields from file 2. The ``-o``
flag lets you specify exactly which fields to output::

    -o "1.2,2.1"   # Field 2 of file 1, then field 1 of file 2

=== CLI Builder Integration ===

The entire CLI is defined in ``join.json``.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import TextIO

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "join.json")


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------


class JoinOptions:
    """Container for join option flags."""

    def __init__(self, **kwargs: int | bool | str | list[str] | None) -> None:
        self.field1: int = int(kwargs.get("field1", 1) or 1)
        self.field2: int = int(kwargs.get("field2", 1) or 1)
        self.separator: str | None = kwargs.get("separator", None)  # type: ignore[assignment]
        self.output_format: str | None = kwargs.get("output_format", None)  # type: ignore[assignment]
        self.empty: str = str(kwargs.get("empty", "") or "")
        self.ignore_case: bool = bool(kwargs.get("ignore_case", False))
        self.header: bool = bool(kwargs.get("header", False))
        self.unpaired: list[str] = kwargs.get("unpaired", []) or []  # type: ignore[assignment]
        self.only_unpaired: str | None = kwargs.get("only_unpaired", None)  # type: ignore[assignment]
        self.zero_terminated: bool = bool(kwargs.get("zero_terminated", False))


# ---------------------------------------------------------------------------
# Business logic — field extraction
# ---------------------------------------------------------------------------


def split_line(line: str, separator: str | None = None) -> list[str]:
    """Split a line into fields.

    If a separator is specified, split on that exact character.
    Otherwise, split on runs of whitespace (the default for ``join``).

    Args:
        line: The line to split.
        separator: Optional field separator character.

    Returns:
        A list of field strings.

    Examples::

        >>> split_line("Alice  Bob  Charlie")
        ['Alice', 'Bob', 'Charlie']
        >>> split_line("Alice:Bob:Charlie", separator=":")
        ['Alice', 'Bob', 'Charlie']
    """
    if separator is not None:
        return line.split(separator)
    return line.split()


def get_field(fields: list[str], field_num: int) -> str:
    """Get a 1-based field from a field list.

    Args:
        fields: The list of fields.
        field_num: 1-based field index.

    Returns:
        The field value, or empty string if out of range.
    """
    idx = field_num - 1
    if 0 <= idx < len(fields):
        return fields[idx]
    return ""


def get_key(fields: list[str], field_num: int, ignore_case: bool = False) -> str:
    """Extract the join key from a field list.

    Args:
        fields: The split fields.
        field_num: 1-based field number for the join key.
        ignore_case: If True, fold to lowercase for comparison.

    Returns:
        The key string.
    """
    key = get_field(fields, field_num)
    if ignore_case:
        key = key.lower()
    return key


# ---------------------------------------------------------------------------
# Business logic — output formatting
# ---------------------------------------------------------------------------


def format_output_line(
    join_key: str,
    fields1: list[str],
    fields2: list[str],
    opts: JoinOptions,
) -> str:
    """Format a joined output line.

    If ``-o`` (output format) is specified, use that format. Otherwise,
    output the join key followed by remaining fields from both files.

    Default output format
    ~~~~~~~~~~~~~~~~~~~~~~
    ::

        join_key field1[0] field1[1] ... field2[0] field2[1] ...

    Where field1 and field2 exclude the join field itself.

    Args:
        join_key: The common join field value.
        fields1: All fields from file 1.
        fields2: All fields from file 2.
        opts: Join options.

    Returns:
        Formatted output line.
    """
    sep = opts.separator if opts.separator is not None else " "

    if opts.output_format:
        # Parse the format string: comma-separated list of FILENUM.FIELDNUM
        # or "0" for the join field.
        parts: list[str] = []
        for spec in opts.output_format.replace(",", " ").split():
            if spec == "0":
                parts.append(join_key)
            elif "." in spec:
                file_num_str, field_num_str = spec.split(".", 1)
                file_num = int(file_num_str)
                field_num = int(field_num_str)
                if file_num == 1:
                    parts.append(get_field(fields1, field_num) or opts.empty)
                elif file_num == 2:
                    parts.append(get_field(fields2, field_num) or opts.empty)
        return sep.join(parts)

    # Default format: join key + remaining fields from both files.
    result_fields: list[str] = [join_key]

    for i, f in enumerate(fields1):
        if i != opts.field1 - 1:
            result_fields.append(f)

    for i, f in enumerate(fields2):
        if i != opts.field2 - 1:
            result_fields.append(f)

    return sep.join(result_fields)


def format_unpaired_line(
    fields: list[str],
    opts: JoinOptions,
    file_num: int,
) -> str:
    """Format an unpaired line (from -a or -v).

    For unpaired lines, the fields from the other file are empty.

    Args:
        fields: Fields from the file with the unpaired line.
        opts: Join options.
        file_num: Which file (1 or 2) the line comes from.

    Returns:
        Formatted output line.
    """
    sep = opts.separator if opts.separator is not None else " "

    if opts.output_format:
        parts: list[str] = []
        for spec in opts.output_format.replace(",", " ").split():
            if spec == "0":
                parts.append(get_field(fields, opts.field1 if file_num == 1 else opts.field2))
            elif "." in spec:
                file_num_str, field_num_str = spec.split(".", 1)
                fnum = int(file_num_str)
                field_idx = int(field_num_str)
                if fnum == file_num:
                    parts.append(get_field(fields, field_idx) or opts.empty)
                else:
                    parts.append(opts.empty)
        return sep.join(parts)

    return sep.join(fields)


# ---------------------------------------------------------------------------
# Business logic — merge join
# ---------------------------------------------------------------------------


def join_files(
    lines1: list[str],
    lines2: list[str],
    opts: JoinOptions,
) -> list[str]:
    """Perform a merge-join on two sorted lists of lines.

    This implements the core join algorithm. Both inputs must be sorted
    on the join field (field1 for lines1, field2 for lines2).

    The algorithm handles duplicate keys by producing the Cartesian
    product of matching lines from both files.

    Args:
        lines1: Lines from file 1 (sorted on join field).
        lines2: Lines from file 2 (sorted on join field).
        opts: Join options.

    Returns:
        List of formatted output lines.

    Algorithm walkthrough
    ~~~~~~~~~~~~~~~~~~~~~~
    ::

        lines1: ["1 A", "2 B", "2 C", "4 D"]
        lines2: ["2 X", "2 Y", "3 Z"]
        join field: 1 (default)

        i=0, j=0: key1="1" < key2="2" → unpaired from file1
        i=1, j=0: key1="2" = key2="2" → match group
          File 1 has keys "2": lines at i=1,2
          File 2 has keys "2": lines at j=0,1
          Cartesian product: (1,0), (1,1), (2,0), (2,1)
          Output: "2 B X", "2 B Y", "2 C X", "2 C Y"
        i=3, j=2: key1="4" > key2="3" → unpaired from file2
        i=3, j=3: key1="4", no more file2 → unpaired from file1
    """
    output: list[str] = []
    show_paired = opts.only_unpaired is None
    show_unpaired_1 = "1" in opts.unpaired or opts.only_unpaired == "1"
    show_unpaired_2 = "2" in opts.unpaired or opts.only_unpaired == "2"

    # Parse lines into fields.
    parsed1 = [split_line(line, opts.separator) for line in lines1]
    parsed2 = [split_line(line, opts.separator) for line in lines2]

    # Handle header line.
    if opts.header and parsed1 and parsed2:
        header_out = format_output_line(
            get_field(parsed1[0], opts.field1),
            parsed1[0],
            parsed2[0],
            opts,
        )
        output.append(header_out)
        parsed1 = parsed1[1:]
        parsed2 = parsed2[1:]

    i = 0
    j = 0

    while i < len(parsed1) and j < len(parsed2):
        key1 = get_key(parsed1[i], opts.field1, opts.ignore_case)
        key2 = get_key(parsed2[j], opts.field2, opts.ignore_case)

        if key1 < key2:
            if show_unpaired_1:
                output.append(format_unpaired_line(parsed1[i], opts, 1))
            i += 1
        elif key1 > key2:
            if show_unpaired_2:
                output.append(format_unpaired_line(parsed2[j], opts, 2))
            j += 1
        else:
            # Keys match — find all lines with this key in both files.
            # This handles the case where multiple consecutive lines
            # share the same join key.
            i_start = i
            j_start = j

            while i < len(parsed1) and get_key(parsed1[i], opts.field1, opts.ignore_case) == key1:
                i += 1
            while j < len(parsed2) and get_key(parsed2[j], opts.field2, opts.ignore_case) == key2:
                j += 1

            # Cartesian product of matching lines.
            if show_paired:
                for ii in range(i_start, i):
                    for jj in range(j_start, j):
                        join_key = get_field(parsed1[ii], opts.field1)
                        line = format_output_line(
                            join_key, parsed1[ii], parsed2[jj], opts,
                        )
                        output.append(line)

            # If -a is specified, also output the lines as unpaired.
            # (This is unusual but matches GNU behavior for duplicate keys.)

    # Handle remaining unpaired lines.
    while i < len(parsed1):
        if show_unpaired_1:
            output.append(format_unpaired_line(parsed1[i], opts, 1))
        i += 1

    while j < len(parsed2):
        if show_unpaired_2:
            output.append(format_unpaired_line(parsed2[j], opts, 2))
        j += 1

    return output


# ---------------------------------------------------------------------------
# Business logic — file reading
# ---------------------------------------------------------------------------


def read_lines(source: str | TextIO, zero_terminated: bool = False) -> list[str]:
    """Read lines from a file path or stream.

    Args:
        source: A file path string, or ``-`` for stdin, or a TextIO.
        zero_terminated: If True, split on NUL instead of newline.

    Returns:
        A list of lines (without terminators).
    """
    if isinstance(source, str):
        if source == "-":
            content = sys.stdin.read()
        else:
            with open(source) as f:
                content = f.read()
    else:
        content = source.read()

    if zero_terminated:
        return content.split("\0")

    return [line.rstrip("\n").rstrip("\r") for line in content.splitlines()]


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then join files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"join: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Extract flags ---------------------------------------------
    assert isinstance(result, ParseResult)

    # The -j flag sets both field1 and field2.
    field1 = result.flags.get("field1", None)
    field2 = result.flags.get("field2", None)
    join_field = result.flags.get("join_field", None)

    if join_field is not None:
        if field1 is None:
            field1 = join_field
        if field2 is None:
            field2 = join_field

    unpaired = result.flags.get("unpaired", [])
    if isinstance(unpaired, str):
        unpaired = [unpaired]

    opts = JoinOptions(
        field1=field1 or 1,
        field2=field2 or 1,
        separator=result.flags.get("separator", None),
        output_format=result.flags.get("format", None),
        empty=result.flags.get("empty", ""),
        ignore_case=result.flags.get("ignore_case", False),
        header=result.flags.get("header", False),
        unpaired=unpaired,
        only_unpaired=result.flags.get("only_unpaired", None),
        zero_terminated=result.flags.get("zero_terminated", False),
    )

    # --- Step 4: Read input files ------------------------------------------
    file1_path = result.arguments.get("file1", "")
    file2_path = result.arguments.get("file2", "")

    lines1 = read_lines(file1_path, opts.zero_terminated)
    lines2 = read_lines(file2_path, opts.zero_terminated)

    # --- Step 5: Join and output -------------------------------------------
    output = join_files(lines1, lines2, opts)
    terminator = "\0" if opts.zero_terminated else "\n"
    for line in output:
        sys.stdout.write(line + terminator)


if __name__ == "__main__":
    main()
