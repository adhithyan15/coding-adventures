"""split -- split a file into pieces.

=== What This Program Does ===

This is a reimplementation of the GNU ``split`` utility. It breaks a
single file into multiple smaller files, each containing a fixed number
of lines or bytes.

=== Why Split Files? ===

Splitting is useful when:

1. A file is too large for a text editor or email attachment.
2. You need to process a large file in parallel chunks.
3. You want to distribute data across multiple storage devices.
4. You need to upload a large file to a service with size limits.

The companion tool ``cat`` can reassemble the pieces::

    split -l 100 bigfile.txt chunk_     # Split into 100-line chunks
    cat chunk_* > reassembled.txt       # Reassemble

=== Splitting Modes ===

+------------+-----------------------------------------------+
| Flag       | Mode                                          |
+============+===============================================+
| ``-l N``   | Split by lines (default: 1000 lines per file) |
| ``-b SIZE``| Split by bytes (e.g., ``-b 1M``)             |
| ``-n N``   | Split into exactly N files of equal size      |
+------------+-----------------------------------------------+

=== Output Filenames ===

Output files are named by combining a prefix (default ``x``) with a
suffix. The suffix type and length can be controlled:

+-----------+--------------------------------------------+
| Flag      | Suffix style                               |
+===========+============================================+
| (default) | Alphabetic: ``aa``, ``ab``, ..., ``zz``    |
| ``-d``    | Numeric: ``00``, ``01``, ..., ``99``       |
| ``-x``    | Hexadecimal: ``00``, ``01``, ..., ``ff``   |
+-----------+--------------------------------------------+

The suffix length defaults to 2 but can be changed with ``-a N``.

=== Suffix Generation ===

Alphabetic suffixes use a base-26 numbering system:

::

    Index 0  → "aa"
    Index 1  → "ab"
    ...
    Index 25 → "az"
    Index 26 → "ba"
    ...
    Index 675 → "zz"  (26*26 - 1)

With suffix length 3, you get ``aaa`` through ``zzz`` (17,576 files).

=== Size Suffixes ===

The ``-b`` flag accepts human-readable size specifications:

+--------+------------------+
| Suffix | Multiplier       |
+========+==================+
| (none) | 1 byte           |
| K / KB | 1024             |
| M / MB | 1024 * 1024      |
| G / GB | 1024^3           |
+--------+------------------+

=== CLI Builder Integration ===

The entire CLI is defined in ``split.json``.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import TextIO

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "split.json")


# ---------------------------------------------------------------------------
# Business logic — suffix generation
# ---------------------------------------------------------------------------


def generate_suffix(
    index: int,
    length: int = 2,
    *,
    numeric: bool = False,
    hexadecimal: bool = False,
) -> str:
    """Generate a filename suffix for the given chunk index.

    Alphabetic suffixes use base-26 (a-z). Numeric suffixes use
    base-10 (0-9). Hex suffixes use base-16 (0-f).

    Args:
        index: Zero-based chunk index.
        length: Number of suffix characters.
        numeric: If True, use numeric (0-9) suffixes.
        hexadecimal: If True, use hexadecimal (0-f) suffixes.

    Returns:
        The suffix string.

    Raises:
        ValueError: If the index exceeds the suffix space.

    Examples::

        >>> generate_suffix(0, 2)
        'aa'
        >>> generate_suffix(1, 2)
        'ab'
        >>> generate_suffix(26, 2)
        'ba'
        >>> generate_suffix(0, 2, numeric=True)
        '00'
        >>> generate_suffix(15, 2, hexadecimal=True)
        '0f'
    """
    if numeric:
        result = str(index).zfill(length)
        if len(result) > length:
            msg = f"output file suffixes exhausted (index {index} needs more than {length} digits)"
            raise ValueError(msg)
        return result

    if hexadecimal:
        result = format(index, "x").zfill(length)
        if len(result) > length:
            msg = f"output file suffixes exhausted (index {index} needs more than {length} hex digits)"
            raise ValueError(msg)
        return result

    # Alphabetic: base-26 using 'a'-'z'.
    chars: list[str] = []
    remaining = index
    for _ in range(length):
        chars.append(chr(ord("a") + remaining % 26))
        remaining //= 26

    if remaining > 0:
        msg = f"output file suffixes exhausted (index {index} needs more than {length} letters)"
        raise ValueError(msg)

    return "".join(reversed(chars))


def make_filename(
    prefix: str,
    index: int,
    suffix_length: int = 2,
    *,
    numeric: bool = False,
    hexadecimal: bool = False,
    additional_suffix: str = "",
) -> str:
    """Build a complete output filename.

    Args:
        prefix: The filename prefix (default "x").
        index: Zero-based chunk index.
        suffix_length: Length of the generated suffix.
        numeric: Use numeric suffixes.
        hexadecimal: Use hex suffixes.
        additional_suffix: Extra suffix to append (e.g., ".txt").

    Returns:
        The complete filename.

    Example::

        >>> make_filename("chunk_", 3, 2, numeric=True, additional_suffix=".txt")
        'chunk_03.txt'
    """
    suffix = generate_suffix(
        index,
        suffix_length,
        numeric=numeric,
        hexadecimal=hexadecimal,
    )
    return prefix + suffix + additional_suffix


# ---------------------------------------------------------------------------
# Business logic — size parsing
# ---------------------------------------------------------------------------


def parse_size(size_str: str) -> int:
    """Parse a human-readable size string into bytes.

    Supports suffixes: K/KB, M/MB, G/GB, T/TB (powers of 1024).
    Also supports bare numbers.

    Args:
        size_str: The size string (e.g., "10M", "1024", "512KB").

    Returns:
        Size in bytes.

    Raises:
        ValueError: If the string cannot be parsed.

    Examples::

        >>> parse_size("1024")
        1024
        >>> parse_size("1K")
        1024
        >>> parse_size("2M")
        2097152
    """
    multipliers = {
        "K": 1024,
        "KB": 1024,
        "M": 1024 ** 2,
        "MB": 1024 ** 2,
        "G": 1024 ** 3,
        "GB": 1024 ** 3,
        "T": 1024 ** 4,
        "TB": 1024 ** 4,
    }

    size_str = size_str.strip()

    # Try to find a suffix.
    for suffix in sorted(multipliers, key=len, reverse=True):
        if size_str.upper().endswith(suffix):
            num_part = size_str[: -len(suffix)].strip()
            if not num_part:
                num_part = "1"
            return int(float(num_part) * multipliers[suffix])

    # No suffix — treat as raw bytes.
    return int(size_str)


# ---------------------------------------------------------------------------
# Business logic — splitting
# ---------------------------------------------------------------------------


def split_by_lines(
    data: str,
    lines_per_chunk: int,
    prefix: str,
    *,
    suffix_length: int = 2,
    numeric: bool = False,
    hexadecimal: bool = False,
    additional_suffix: str = "",
    verbose: bool = False,
) -> list[tuple[str, str]]:
    """Split text data into chunks of N lines each.

    This is the default split mode. Each output chunk contains exactly
    ``lines_per_chunk`` lines, except possibly the last chunk which
    may contain fewer.

    Args:
        data: The input text (may include newlines).
        lines_per_chunk: Number of lines per output file.
        prefix: Output filename prefix.
        suffix_length: Length of filename suffix.
        numeric: Use numeric suffixes.
        hexadecimal: Use hex suffixes.
        additional_suffix: Extra filename suffix.
        verbose: If True, return diagnostic info.

    Returns:
        A list of (filename, content) tuples. Each tuple represents
        one output file.

    Example::

        >>> result = split_by_lines("a\\nb\\nc\\nd\\ne\\n", 2, "x")
        >>> [(name, content) for name, content in result]
        [('xaa', 'a\\nb\\n'), ('xab', 'c\\nd\\n'), ('xac', 'e\\n')]
    """
    lines = data.splitlines(keepends=True)
    chunks: list[tuple[str, str]] = []

    for i in range(0, len(lines), lines_per_chunk):
        chunk_lines = lines[i : i + lines_per_chunk]
        filename = make_filename(
            prefix,
            len(chunks),
            suffix_length,
            numeric=numeric,
            hexadecimal=hexadecimal,
            additional_suffix=additional_suffix,
        )
        chunks.append((filename, "".join(chunk_lines)))

    return chunks


def split_by_bytes(
    data: bytes,
    bytes_per_chunk: int,
    prefix: str,
    *,
    suffix_length: int = 2,
    numeric: bool = False,
    hexadecimal: bool = False,
    additional_suffix: str = "",
    verbose: bool = False,
) -> list[tuple[str, bytes]]:
    """Split binary data into chunks of N bytes each.

    Args:
        data: The input data as bytes.
        bytes_per_chunk: Number of bytes per output file.
        prefix: Output filename prefix.
        suffix_length: Length of filename suffix.
        numeric: Use numeric suffixes.
        hexadecimal: Use hex suffixes.
        additional_suffix: Extra filename suffix.
        verbose: If True, return diagnostic info.

    Returns:
        A list of (filename, content) tuples where content is bytes.

    Example::

        >>> result = split_by_bytes(b"abcdefgh", 3, "x")
        >>> [(name, content) for name, content in result]
        [('xaa', b'abc'), ('xab', b'def'), ('xac', b'gh')]
    """
    chunks: list[tuple[str, bytes]] = []

    for i in range(0, len(data), bytes_per_chunk):
        chunk = data[i : i + bytes_per_chunk]
        filename = make_filename(
            prefix,
            len(chunks),
            suffix_length,
            numeric=numeric,
            hexadecimal=hexadecimal,
            additional_suffix=additional_suffix,
        )
        chunks.append((filename, chunk))

    return chunks


def split_by_number(
    data: bytes,
    num_chunks: int,
    prefix: str,
    *,
    suffix_length: int = 2,
    numeric: bool = False,
    hexadecimal: bool = False,
    additional_suffix: str = "",
) -> list[tuple[str, bytes]]:
    """Split data into exactly N chunks of roughly equal size.

    The first ``remainder`` chunks get one extra byte each, ensuring
    all data is accounted for.

    Args:
        data: The input data as bytes.
        num_chunks: Number of output files to create.
        prefix: Output filename prefix.
        suffix_length: Length of filename suffix.
        numeric: Use numeric suffixes.
        hexadecimal: Use hex suffixes.
        additional_suffix: Extra filename suffix.

    Returns:
        A list of (filename, content) tuples.
    """
    total = len(data)
    base_size = total // num_chunks
    remainder = total % num_chunks

    chunks: list[tuple[str, bytes]] = []
    offset = 0

    for i in range(num_chunks):
        chunk_size = base_size + (1 if i < remainder else 0)
        chunk = data[offset : offset + chunk_size]
        offset += chunk_size

        filename = make_filename(
            prefix,
            i,
            suffix_length,
            numeric=numeric,
            hexadecimal=hexadecimal,
            additional_suffix=additional_suffix,
        )
        chunks.append((filename, chunk))

    return chunks


# ---------------------------------------------------------------------------
# Business logic — reading input
# ---------------------------------------------------------------------------


def read_input(source: str) -> tuple[str, bytes]:
    """Read input from a file path or stdin.

    Args:
        source: File path, or ``-`` for stdin.

    Returns:
        A tuple of (text_content, binary_content). For stdin, the
        binary content is the UTF-8 encoding of the text.
    """
    if source == "-":
        text = sys.stdin.read()
        return text, text.encode("utf-8")

    with open(source, "rb") as f:
        binary = f.read()

    try:
        text = binary.decode("utf-8")
    except UnicodeDecodeError:
        text = binary.decode("latin-1")

    return text, binary


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then split files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"split: {error.message}", file=sys.stderr)
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

    bytes_flag = result.flags.get("bytes", None)
    number_flag = result.flags.get("number", None)
    lines_per_chunk = result.flags.get("lines", 1000)
    suffix_length = result.flags.get("suffix_length", 2)
    numeric_suffixes = result.flags.get("numeric_suffixes", False)
    hex_suffixes = result.flags.get("hex_suffixes", False)
    additional_suffix = result.flags.get("additional_suffix", "") or ""
    verbose = result.flags.get("verbose", False)

    # --- Step 4: Read input ------------------------------------------------
    input_file = result.arguments.get("file", "-")
    prefix = result.arguments.get("prefix", "x")

    text_data, binary_data = read_input(input_file)

    # --- Step 5: Split and write -------------------------------------------
    if bytes_flag:
        chunk_size = parse_size(bytes_flag)
        chunks = split_by_bytes(
            binary_data,
            chunk_size,
            prefix,
            suffix_length=suffix_length,
            numeric=numeric_suffixes,
            hexadecimal=hex_suffixes,
            additional_suffix=additional_suffix,
        )
        for filename, content in chunks:
            if verbose:
                print(f"creating file '{filename}'", file=sys.stderr)
            with open(filename, "wb") as f:
                f.write(content)

    elif number_flag:
        num_chunks = int(number_flag)
        chunks = split_by_number(
            binary_data,
            num_chunks,
            prefix,
            suffix_length=suffix_length,
            numeric=numeric_suffixes,
            hexadecimal=hex_suffixes,
            additional_suffix=additional_suffix,
        )
        for filename, content in chunks:
            if verbose:
                print(f"creating file '{filename}'", file=sys.stderr)
            with open(filename, "wb") as f:
                f.write(content)

    else:
        text_chunks = split_by_lines(
            text_data,
            lines_per_chunk,
            prefix,
            suffix_length=suffix_length,
            numeric=numeric_suffixes,
            hexadecimal=hex_suffixes,
            additional_suffix=additional_suffix,
        )
        for filename, content in text_chunks:
            if verbose:
                print(f"creating file '{filename}'", file=sys.stderr)
            with open(filename, "w") as f:
                f.write(content)


if __name__ == "__main__":
    main()
