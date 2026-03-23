"""tr — translate or delete characters.

=== What This Program Does ===

This is a reimplementation of the GNU ``tr`` utility. It reads from
standard input, transforms characters according to two "sets", and
writes to standard output. The three main operations are:

1. **Translate**: Replace each character in SET1 with the corresponding
   character in SET2.
2. **Delete** (``-d``): Remove all characters in SET1.
3. **Squeeze** (``-s``): Replace runs of repeated characters from SET1
   (or SET2 when translating) with a single occurrence.

=== Character Sets ===

Sets are specified as strings with special syntax for ranges and classes:

- **Literal characters**: ``abc`` means the set {a, b, c}.
- **Ranges**: ``a-z`` means all characters from a to z (by ASCII value).
- **Character classes**: ``[:upper:]`` means all uppercase letters,
  ``[:lower:]`` means all lowercase, etc.
- **Escape sequences**: ``\\n`` means newline, ``\\t`` means tab, etc.

=== The Complement Flag (-c) ===

With ``-c``, SET1 is replaced by its complement — all characters NOT
in SET1. This is useful for deleting everything except certain characters::

    $ echo "Hello 123 World" | tr -cd '0-9'
    123

=== Translation (SET1 -> SET2) ===

Characters in SET1 are replaced one-for-one with characters in SET2.
If SET2 is shorter, its last character is repeated to match the length
of SET1. For example::

    $ echo "hello" | tr 'a-z' 'A-Z'
    HELLO

=== Squeeze Repeats (-s) ===

With ``-s``, after translation (if any), runs of consecutive identical
characters from the specified set are collapsed to a single character::

    $ echo "mississippi" | tr -s 'sp'
    misisisipi

=== CLI Builder Integration ===

The entire CLI is defined in ``tr.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "tr.json")


def expand_escapes(s: str) -> str:
    """Expand backslash escape sequences in a set string.

    Supports: \\n (newline), \\t (tab), \\r (carriage return),
    \\a (bell), \\b (backspace), \\f (form feed), \\v (vertical tab),
    \\\\ (literal backslash).

    Args:
        s: The string with potential escape sequences.

    Returns:
        The string with escapes expanded.
    """
    escape_map = {
        "n": "\n",
        "t": "\t",
        "r": "\r",
        "a": "\a",
        "b": "\b",
        "f": "\f",
        "v": "\v",
        "\\": "\\",
    }
    result: list[str] = []
    i = 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            next_char = s[i + 1]
            if next_char in escape_map:
                result.append(escape_map[next_char])
                i += 2
                continue
        result.append(s[i])
        i += 1
    return "".join(result)


def expand_set(set_str: str) -> str:
    """Expand a tr set string into a list of characters.

    This handles:
    - Literal characters
    - Ranges like ``a-z``
    - POSIX character classes like ``[:upper:]``
    - Escape sequences

    Args:
        set_str: The set specification string.

    Returns:
        A string containing all characters in the expanded set.
    """
    # First expand escape sequences.
    set_str = expand_escapes(set_str)

    # Handle POSIX character classes.
    class_map = {
        "[:upper:]": "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "[:lower:]": "abcdefghijklmnopqrstuvwxyz",
        "[:digit:]": "0123456789",
        "[:alpha:]": "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        "[:alnum:]": "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        "[:space:]": " \t\n\r\f\v",
        "[:blank:]": " \t",
        "[:punct:]": "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
        "[:cntrl:]": "".join(chr(c) for c in range(32)) + chr(127),
        "[:print:]": "".join(chr(c) for c in range(32, 127)),
        "[:graph:]": "".join(chr(c) for c in range(33, 127)),
        "[:xdigit:]": "0123456789ABCDEFabcdef",
    }

    # Replace character classes with their expansions.
    for class_name, expansion in class_map.items():
        set_str = set_str.replace(class_name, expansion)

    # Expand ranges like a-z.
    result: list[str] = []
    i = 0
    while i < len(set_str):
        # Check for a range pattern: X-Y where both X and Y are
        # single characters and X < Y.
        if (
            i + 2 < len(set_str)
            and set_str[i + 1] == "-"
            and ord(set_str[i]) <= ord(set_str[i + 2])
        ):
            start = ord(set_str[i])
            end = ord(set_str[i + 2])
            result.extend(chr(c) for c in range(start, end + 1))
            i += 3
        else:
            result.append(set_str[i])
            i += 1

    return "".join(result)


def tr_translate(
    text: str,
    set1_chars: str,
    set2_chars: str,
    *,
    squeeze: bool,
) -> str:
    """Translate characters in text from set1 to set2.

    For each character in the input:
    - If it's in set1, replace it with the corresponding character in set2.
    - If set2 is shorter than set1, the last character of set2 is used
      for all remaining set1 characters.

    If squeeze is True, consecutive identical characters that appear in
    set2 are collapsed to a single occurrence.

    Args:
        text: The input text.
        set1_chars: The source character set.
        set2_chars: The destination character set.
        squeeze: Whether to squeeze repeated result characters.

    Returns:
        The translated text.
    """
    # Build a translation mapping. If set2 is shorter, pad it by
    # repeating the last character.
    if set2_chars:
        padded_set2 = set2_chars.ljust(len(set1_chars), set2_chars[-1])
    else:
        padded_set2 = set1_chars  # No translation if set2 is empty.

    trans_map = {}
    for i, ch in enumerate(set1_chars):
        if i < len(padded_set2):
            trans_map[ch] = padded_set2[i]

    # The squeeze set is the characters in set2 (after translation).
    squeeze_set = set(set2_chars) if squeeze else set()

    result: list[str] = []
    prev_char = ""

    for ch in text:
        translated = trans_map.get(ch, ch)

        # Squeeze: skip consecutive duplicates of characters in the
        # squeeze set.
        if squeeze and translated in squeeze_set and translated == prev_char:
            continue

        result.append(translated)
        prev_char = translated

    return "".join(result)


def tr_delete(text: str, set1_chars: str, *, squeeze: bool, squeeze_set: str) -> str:
    """Delete characters in set1 from text.

    Args:
        text: The input text.
        set1_chars: Characters to delete.
        squeeze: Whether to squeeze repeated characters.
        squeeze_set: Characters to squeeze (from SET2 when -ds combined).

    Returns:
        The text with set1 characters removed.
    """
    delete_chars = set(set1_chars)
    squeeze_chars = set(squeeze_set) if squeeze else set()

    result: list[str] = []
    prev_char = ""

    for ch in text:
        if ch in delete_chars:
            continue

        if squeeze and ch in squeeze_chars and ch == prev_char:
            continue

        result.append(ch)
        prev_char = ch

    return "".join(result)


def tr_squeeze_only(text: str, set1_chars: str) -> str:
    """Squeeze repeated characters in set1.

    Without translation or deletion, ``-s`` alone squeezes runs of
    characters from set1.

    Args:
        text: The input text.
        set1_chars: Characters to squeeze.

    Returns:
        The text with squeezed characters.
    """
    squeeze_chars = set(set1_chars)
    result: list[str] = []
    prev_char = ""

    for ch in text:
        if ch in squeeze_chars and ch == prev_char:
            continue
        result.append(ch)
        prev_char = ch

    return "".join(result)


def complement_set(set_chars: str) -> str:
    """Return all characters NOT in the given set.

    The complement covers all bytes 0-255, which is the standard
    behavior for tr.

    Args:
        set_chars: Characters to exclude.

    Returns:
        A string of all byte values not in set_chars.
    """
    char_set = set(set_chars)
    return "".join(chr(c) for c in range(256) if chr(c) not in char_set)


def main() -> None:
    """Entry point: parse args via CLI Builder, then translate stdin."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"tr: {error.message}", file=sys.stderr)
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

    complement = result.flags.get("complement", False)
    delete = result.flags.get("delete", False)
    squeeze = result.flags.get("squeeze_repeats", False)

    set1_raw = result.arguments.get("set1", "")
    set2_raw = result.arguments.get("set2", "")

    # Expand the character sets.
    set1_chars = expand_set(set1_raw)
    set2_chars = expand_set(set2_raw) if set2_raw else ""

    # Apply complement if requested.
    if complement:
        set1_chars = complement_set(set1_chars)

    # Read all of stdin.
    try:
        text = sys.stdin.read()
    except KeyboardInterrupt:
        raise SystemExit(130) from None

    # Apply the appropriate operation.
    if delete and squeeze:
        # -ds: delete chars in set1, then squeeze chars in set2.
        output = tr_delete(text, set1_chars, squeeze=True, squeeze_set=set2_chars)
    elif delete:
        # -d: just delete chars in set1.
        output = tr_delete(text, set1_chars, squeeze=False, squeeze_set="")
    elif squeeze and not set2_raw:
        # -s without SET2: squeeze chars in set1.
        output = tr_squeeze_only(text, set1_chars)
    else:
        # Default: translate set1 -> set2, optionally squeeze.
        if not set2_raw:
            print("tr: missing operand after SET1", file=sys.stderr)
            raise SystemExit(1) from None
        output = tr_translate(text, set1_chars, set2_chars, squeeze=squeeze)

    try:
        sys.stdout.write(output)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
