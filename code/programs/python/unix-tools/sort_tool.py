"""sort — sort lines of text files.

=== What This Program Does ===

This is a reimplementation of the GNU ``sort`` utility. It reads lines
from one or more input files (or standard input), sorts them according
to the specified ordering, and writes the result to standard output.

=== How Sorting Works ===

By default, ``sort`` compares entire lines using the current locale's
collating sequence. In our implementation, we use Python's built-in
``sorted()`` function, which performs a stable, Timsort-based sort.

The ordering can be modified by several flags:

- ``-r``: Reverse the comparison (descending order).
- ``-n``: Numeric sort — compare lines by their leading numeric value.
- ``-f``: Fold lowercase to uppercase (case-insensitive).
- ``-u``: Output only the first of a run of equal lines.
- ``-k``: Sort by a specific key field (e.g., ``-k2,2`` sorts by field 2).
- ``-t``: Use a specific field separator (default: runs of blanks).

=== Key Definitions ===

The ``-k`` flag accepts a KEYDEF of the form ``start[,stop]``, where
``start`` and ``stop`` are field numbers (1-based). For example::

    sort -k2,2 file.txt      # Sort by field 2 only
    sort -k1,1 -k3,3n file.txt  # Sort by field 1, break ties by field 3 numerically

Each key can have modifier suffixes: ``n`` (numeric), ``r`` (reverse),
``f`` (fold case), ``b`` (ignore leading blanks), ``d`` (dictionary order).

=== Numeric Sort ===

With ``-n``, sort treats the beginning of each line (or key field) as
a number. Non-numeric lines sort before all numeric lines. This is
different from ``-g`` (general numeric sort), which handles scientific
notation like ``1.5e3``.

=== Stability ===

Python's ``sorted()`` is stable by default — equal elements preserve
their original order. This matches ``sort -s`` behavior.

=== CLI Builder Integration ===

The entire CLI is defined in ``sort.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "sort.json")


# ---------------------------------------------------------------------------
# Helper: parse a numeric value from the start of a string.
# ---------------------------------------------------------------------------

def _numeric_key(s: str) -> tuple[int, float, str]:
    """Extract a numeric sort key from a string.

    GNU sort's ``-n`` mode parses leading whitespace, an optional sign,
    and digits (with optional decimal point). Non-numeric strings sort
    as 0 but before actual zeros.

    Returns a tuple (has_number, numeric_value, original_string) so that
    non-numeric lines sort before numeric ones, and ties fall back to
    lexicographic comparison.
    """
    stripped = s.lstrip()
    match = re.match(r"[+-]?\d+\.?\d*", stripped)
    if match:
        return (1, float(match.group()), s)
    return (0, 0.0, s)


def _general_numeric_key(s: str) -> tuple[int, float, str]:
    """Extract a general numeric sort key (handles scientific notation).

    This is used by ``-g``. It parses floats including scientific
    notation like ``1.5e3``.
    """
    stripped = s.lstrip()
    match = re.match(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?", stripped)
    if match:
        return (1, float(match.group()), s)
    return (0, 0.0, s)


# ---------------------------------------------------------------------------
# Helper: human-readable numeric sort (e.g., 2K, 1G).
# ---------------------------------------------------------------------------

_HUMAN_SUFFIXES = {
    "K": 1e3, "M": 1e6, "G": 1e9, "T": 1e12, "P": 1e15, "E": 1e18,
    "k": 1e3, "m": 1e6, "g": 1e9, "t": 1e12, "p": 1e15, "e": 1e18,
}


def _human_numeric_key(s: str) -> tuple[int, float, str]:
    """Parse human-readable numbers like 2K, 100M, 1.5G."""
    stripped = s.lstrip()
    match = re.match(r"([+-]?\d+\.?\d*)\s*([KMGTPEkmgtpe])?", stripped)
    if match:
        value = float(match.group(1))
        suffix = match.group(2)
        if suffix:
            value *= _HUMAN_SUFFIXES.get(suffix, 1)
        return (1, value, s)
    return (0, 0.0, s)


# ---------------------------------------------------------------------------
# Helper: month sort.
# ---------------------------------------------------------------------------

_MONTHS = {
    "JAN": 1, "FEB": 2, "MAR": 3, "APR": 4, "MAY": 5, "JUN": 6,
    "JUL": 7, "AUG": 8, "SEP": 9, "OCT": 10, "NOV": 11, "DEC": 12,
}


def _month_key(s: str) -> tuple[int, str]:
    """Parse a month abbreviation for month sort (-M)."""
    stripped = s.lstrip().upper()[:3]
    month_num = _MONTHS.get(stripped, 0)
    return (month_num, s)


# ---------------------------------------------------------------------------
# Helper: version sort.
# ---------------------------------------------------------------------------

def _version_key(s: str) -> list[int | str]:
    """Split a string into chunks for natural/version sort.

    "file10.txt" -> ["file", 10, ".txt"]

    Numeric chunks are compared as integers so that "file2" < "file10".
    """
    parts: list[int | str] = []
    for chunk in re.split(r"(\d+)", s):
        if chunk.isdigit():
            parts.append(int(chunk))
        else:
            parts.append(chunk)
    return parts


# ---------------------------------------------------------------------------
# Core: extract a key from a line based on field definitions.
# ---------------------------------------------------------------------------

def _split_fields(line: str, separator: str | None) -> list[str]:
    """Split a line into fields.

    If ``separator`` is None, fields are separated by runs of blanks
    (the default). Otherwise, the separator character is used as a
    delimiter, and empty fields are preserved (like ``awk -F``).
    """
    if separator is None:
        return line.split()
    return line.split(separator)


def _extract_key(line: str, key_def: str, separator: str | None) -> str:
    """Extract a sort key from a line using a KEYDEF specification.

    A key_def has the form ``start[,stop]`` where start and stop are
    1-based field numbers. Modifier letters (n, r, f, b, d) on the
    positions are NOT handled here — this function just extracts the
    substring.

    Examples:
        "2"    -> field 2 to end of line
        "2,2"  -> field 2 only
        "1,3"  -> fields 1 through 3
    """
    # Strip any modifier letters for field extraction.
    clean = re.sub(r"[nrfbdMghVi]", "", key_def)
    parts = clean.split(",")
    start = int(parts[0]) if parts[0] else 1
    stop = int(parts[1]) if len(parts) > 1 and parts[1] else None

    fields = _split_fields(line, separator)

    # Convert to 0-based indexing.
    start_idx = max(0, start - 1)
    if stop is not None:
        stop_idx = min(len(fields), stop)
        selected = fields[start_idx:stop_idx]
    else:
        selected = fields[start_idx:]

    joiner = separator if separator is not None else " "
    return joiner.join(selected)


# ---------------------------------------------------------------------------
# Business logic: sort_lines
# ---------------------------------------------------------------------------


def sort_lines(
    lines: list[str],
    *,
    reverse: bool = False,
    numeric: bool = False,
    general_numeric: bool = False,
    human_numeric: bool = False,
    month: bool = False,
    version: bool = False,
    ignore_case: bool = False,
    dictionary_order: bool = False,
    unique: bool = False,
    key_defs: list[str] | None = None,
    field_sep: str | None = None,
    ignore_leading_blanks: bool = False,
    stable: bool = False,
) -> list[str]:
    """Sort a list of lines according to the specified options.

    This function wraps Python's ``sorted()`` with a custom key function
    built from the various sort options.

    Args:
        lines: The input lines (with trailing newlines stripped).
        reverse: If True, reverse the sort order.
        numeric: If True, sort by numeric value (-n).
        general_numeric: If True, sort by general numeric value (-g).
        human_numeric: If True, sort by human-readable numbers (-h).
        month: If True, sort by month abbreviation (-M).
        version: If True, natural sort of version numbers (-V).
        ignore_case: If True, fold case for comparison (-f).
        dictionary_order: If True, consider only blanks and alphanumeric (-d).
        unique: If True, output only the first of equal runs (-u).
        key_defs: List of KEYDEF strings for -k.
        field_sep: Field separator character for -t.
        ignore_leading_blanks: If True, ignore leading blanks (-b).
        stable: If True, use stable sort (Python sort is stable by default).

    Returns:
        The sorted lines.
    """
    def make_key(line: str) -> tuple:  # noqa: ANN401
        """Build a composite sort key for a single line.

        If key_defs are given, we extract each key field and apply the
        sort mode to it. Otherwise, we use the entire line.
        """
        if key_defs:
            key_parts: list = []
            for kd in key_defs:
                field_text = _extract_key(line, kd, field_sep)
                key_parts.append(_apply_sort_mode(field_text, kd))
            return tuple(key_parts)
        return (_apply_sort_mode(line, ""),)

    def _apply_sort_mode(text: str, key_def: str) -> tuple:  # noqa: ANN401
        """Apply the sort mode to a text based on global flags or key modifiers."""
        # Check for per-key modifiers.
        use_numeric = numeric or "n" in key_def
        use_general = general_numeric or "g" in key_def
        use_human = human_numeric or "h" in key_def
        use_month = month or "M" in key_def
        use_version = version or "V" in key_def
        use_fold = ignore_case or "f" in key_def
        use_dict = dictionary_order or "d" in key_def
        use_blanks = ignore_leading_blanks or "b" in key_def

        if use_blanks:
            text = text.lstrip()
        if use_dict:
            text = re.sub(r"[^a-zA-Z0-9 \t]", "", text)
        if use_fold:
            text = text.lower()

        if use_numeric:
            return _numeric_key(text)
        if use_general:
            return _general_numeric_key(text)
        if use_human:
            return _human_numeric_key(text)
        if use_month:
            return _month_key(text)
        if use_version:
            return (0, 0.0, _version_key(text))
        return (0, 0.0, text)

    sorted_lines = sorted(lines, key=make_key, reverse=reverse)

    # --- Unique filtering ---
    # With -u, we remove adjacent duplicates based on the sort key.
    if unique:
        seen_keys: list[tuple] = []
        unique_lines: list[str] = []
        for line in sorted_lines:
            k = make_key(line)
            if k not in seen_keys:
                seen_keys.append(k)
                unique_lines.append(line)
        return unique_lines

    return sorted_lines


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then sort lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"sort: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # Extract flags.
    reverse = result.flags.get("reverse", False)
    numeric = result.flags.get("numeric_sort", False)
    general_numeric = result.flags.get("general_numeric_sort", False)
    human_numeric = result.flags.get("human_numeric_sort", False)
    month_sort = result.flags.get("month_sort", False)
    version_sort = result.flags.get("version_sort", False)
    ignore_case = result.flags.get("ignore_case", False)
    dictionary_order = result.flags.get("dictionary_order", False)
    unique = result.flags.get("unique", False)
    stable = result.flags.get("stable", False)
    ignore_blanks = result.flags.get("ignore_leading_blanks", False)
    key_defs = result.flags.get("key")
    field_sep = result.flags.get("field_separator")
    output_file = result.flags.get("output")

    # Read input files.
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]

    all_lines: list[str] = []
    try:
        for fname in files:
            if fname == "-":
                all_lines.extend(line.rstrip("\n") for line in sys.stdin)
            else:
                with open(fname) as f:
                    all_lines.extend(line.rstrip("\n") for line in f)
    except FileNotFoundError:
        print(f"sort: {fname}: No such file or directory", file=sys.stderr)
        raise SystemExit(2) from None
    except KeyboardInterrupt:
        raise SystemExit(130) from None

    # Normalize key_defs to a list or None.
    if isinstance(key_defs, str):
        key_defs = [key_defs]

    sorted_output = sort_lines(
        all_lines,
        reverse=reverse,
        numeric=numeric,
        general_numeric=general_numeric,
        human_numeric=human_numeric,
        month=month_sort,
        version=version_sort,
        ignore_case=ignore_case,
        dictionary_order=dictionary_order,
        unique=unique,
        key_defs=key_defs,
        field_sep=field_sep,
        ignore_leading_blanks=ignore_blanks,
        stable=stable,
    )

    try:
        if output_file:
            with open(output_file, "w") as f:
                for line in sorted_output:
                    f.write(line + "\n")
        else:
            for line in sorted_output:
                print(line)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
