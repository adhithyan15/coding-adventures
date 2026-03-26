"""cut — remove sections from each line of files.

=== What This Program Does ===

This is a reimplementation of the GNU ``cut`` utility. It selects
portions of each input line and writes only those portions to standard
output.

=== Three Modes of Operation ===

``cut`` works in exactly one of three modes, selected by a required flag:

1. **Byte mode** (``-b LIST``): Select specific byte positions.
2. **Character mode** (``-c LIST``): Select specific character positions.
   (In ASCII, bytes and characters are the same. For UTF-8 multi-byte
   characters, they differ.)
3. **Field mode** (``-f LIST``): Select specific fields, where fields
   are separated by a delimiter (default: TAB).

=== Range Lists ===

The LIST argument specifies which bytes, characters, or fields to
select. It's a comma-separated list of ranges::

    5       — just the 5th element
    3-7     — elements 3 through 7 (inclusive)
    -4      — elements 1 through 4
    8-      — elements 8 through the end
    1,3,5   — elements 1, 3, and 5
    1-3,7-  — elements 1 through 3, and 7 through the end

Positions are 1-based (the first byte/char/field is 1, not 0).

=== Field Mode Details ===

In field mode, the delimiter defaults to TAB. Lines that don't contain
the delimiter are passed through unchanged (unless ``-s`` is used, which
suppresses such lines entirely).

=== The --complement Flag ===

With ``--complement``, the selected set is inverted — you get everything
*except* the specified positions/fields.

=== CLI Builder Integration ===

The entire CLI is defined in ``cut.json``. The three selection modes
(``-b``, ``-c``, ``-f``) form a mutually exclusive group.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "cut.json")


# ---------------------------------------------------------------------------
# Helper: parse a range list like "1-3,5,7-"
# ---------------------------------------------------------------------------


def parse_range_list(spec: str, max_index: int) -> list[int]:
    """Parse a cut-style range specification into a sorted list of 1-based indices.

    The spec is a comma-separated list of ranges. Each range can be:
    - ``N``    — a single index
    - ``N-M``  — a range from N to M (inclusive)
    - ``N-``   — from N to the end (max_index)
    - ``-M``   — from 1 to M

    Args:
        spec: The range specification string (e.g., "1-3,5,7-").
        max_index: The maximum valid index (length of the line or
                   number of fields).

    Returns:
        A sorted list of 1-based indices.

    Example::

        >>> parse_range_list("1-3,5,7-", 10)
        [1, 2, 3, 5, 7, 8, 9, 10]
    """
    indices: set[int] = set()
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            # It's a range.
            left, right = part.split("-", 1)
            start = int(left) if left else 1
            end = int(right) if right else max_index
            for i in range(start, end + 1):
                if 1 <= i <= max_index:
                    indices.add(i)
        else:
            idx = int(part)
            if 1 <= idx <= max_index:
                indices.add(idx)
    return sorted(indices)


# ---------------------------------------------------------------------------
# Business logic: cut_line
# ---------------------------------------------------------------------------


def cut_line(
    line: str,
    *,
    bytes_list: str | None = None,
    chars_list: str | None = None,
    fields_list: str | None = None,
    delimiter: str = "\t",
    only_delimited: bool = False,
    output_delimiter: str | None = None,
    complement: bool = False,
) -> str | None:
    """Cut a single line according to the specified selection mode.

    Exactly one of ``bytes_list``, ``chars_list``, or ``fields_list``
    must be provided. This mirrors the mutually exclusive requirement
    of ``-b``, ``-c``, and ``-f`` in the real ``cut`` command.

    Args:
        line: The input line (without trailing newline).
        bytes_list: Range spec for byte selection (e.g., "1-3,5").
        chars_list: Range spec for character selection.
        fields_list: Range spec for field selection.
        delimiter: Field delimiter (default: TAB). Only used with fields.
        only_delimited: If True, suppress lines without the delimiter.
        output_delimiter: String to join selected parts (defaults to input delimiter).
        complement: If True, invert the selection.

    Returns:
        The cut portion of the line, or None if the line should be suppressed.
    """
    # --- Byte mode ---
    if bytes_list is not None:
        line_bytes = line.encode("utf-8")
        max_idx = len(line_bytes)
        selected = parse_range_list(bytes_list, max_idx)
        if complement:
            selected = [i for i in range(1, max_idx + 1) if i not in selected]
        out_delim = output_delimiter if output_delimiter is not None else ""
        if out_delim:
            # Group consecutive indices to separate with output delimiter.
            parts = [line_bytes[i - 1:i].decode("utf-8", errors="replace") for i in selected]
            return out_delim.join(parts)
        return b"".join(line_bytes[i - 1:i] for i in selected).decode("utf-8", errors="replace")

    # --- Character mode ---
    if chars_list is not None:
        max_idx = len(line)
        selected = parse_range_list(chars_list, max_idx)
        if complement:
            selected = [i for i in range(1, max_idx + 1) if i not in selected]
        out_delim = output_delimiter if output_delimiter is not None else ""
        if out_delim:
            parts = [line[i - 1] for i in selected]
            return out_delim.join(parts)
        return "".join(line[i - 1] for i in selected)

    # --- Field mode ---
    if fields_list is not None:
        # If the line doesn't contain the delimiter, either pass it
        # through or suppress it.
        if delimiter not in line:
            if only_delimited:
                return None
            return line

        fields = line.split(delimiter)
        max_idx = len(fields)
        selected = parse_range_list(fields_list, max_idx)
        if complement:
            selected = [i for i in range(1, max_idx + 1) if i not in selected]

        out_delim = output_delimiter if output_delimiter is not None else delimiter
        return out_delim.join(fields[i - 1] for i in selected)

    # Should never reach here if called correctly.
    msg = "One of bytes_list, chars_list, or fields_list must be provided"
    raise ValueError(msg)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then cut lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"cut: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    bytes_list = result.flags.get("bytes")
    chars_list = result.flags.get("characters")
    fields_list = result.flags.get("fields")
    delimiter = result.flags.get("delimiter", "\t") or "\t"
    only_delimited = result.flags.get("only_delimited", False)
    output_delimiter = result.flags.get("output_delimiter")
    complement = result.flags.get("complement", False)

    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]

    try:
        for fname in files:
            if fname == "-":
                for raw_line in sys.stdin:
                    line = raw_line.rstrip("\n")
                    out = cut_line(
                        line,
                        bytes_list=bytes_list,
                        chars_list=chars_list,
                        fields_list=fields_list,
                        delimiter=delimiter,
                        only_delimited=only_delimited,
                        output_delimiter=output_delimiter,
                        complement=complement,
                    )
                    if out is not None:
                        print(out)
            else:
                with open(fname) as f:
                    for raw_line in f:
                        line = raw_line.rstrip("\n")
                        out = cut_line(
                            line,
                            bytes_list=bytes_list,
                            chars_list=chars_list,
                            fields_list=fields_list,
                            delimiter=delimiter,
                            only_delimited=only_delimited,
                            output_delimiter=output_delimiter,
                            complement=complement,
                        )
                        if out is not None:
                            print(out)
    except FileNotFoundError:
        print(f"cut: {fname}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except BrokenPipeError:
        raise SystemExit(0) from None
    except KeyboardInterrupt:
        raise SystemExit(130) from None


if __name__ == "__main__":
    main()
