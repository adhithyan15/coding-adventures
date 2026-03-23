"""JSON Serializer — converts JsonValue trees (or native Python types) to text.

This is the final stage of the JSON pipeline:

    JSON text  -->  tokens  -->  AST  -->  JsonValue  -->  JSON text
    (input)     lexer       parser     json-value       **json-serializer**

The serializer walks a ``JsonValue`` tree recursively, producing either:

- **Compact JSON**: no unnecessary whitespace, smallest output
- **Pretty JSON**: human-readable with configurable indentation

The algorithm is straightforward recursive descent — the same pattern used
by the parser, but in reverse. Where the parser *consumes* tokens to build
a tree, the serializer *produces* text by walking a tree.

String Escaping (RFC 8259)
--------------------------

JSON requires certain characters to be escaped inside strings. The full
escape table:

    +-----------------+--------+------------------------------------------+
    | Character       | Escape | Why                                      |
    +-----------------+--------+------------------------------------------+
    | ``"`` (quote)   | ``\\"``| Delimiter — must be escaped              |
    | ``\\`` (backsl) | ``\\\\``| Escape char itself                      |
    | Backspace       | ``\\b``| Control char U+0008                      |
    | Form feed       | ``\\f``| Control char U+000C                      |
    | Newline         | ``\\n``| Control char U+000A                      |
    | Carriage return | ``\\r``| Control char U+000D                      |
    | Tab             | ``\\t``| Control char U+0009                      |
    | U+0000..U+001F  | ``\\uXXXX``| All other control characters         |
    +-----------------+--------+------------------------------------------+

Note: forward slash (``/``) is NOT escaped. RFC 8259 allows it but does
not require it, and escaping it adds noise without benefit.
"""

from __future__ import annotations

import math
from typing import TYPE_CHECKING

from json_serializer.config import SerializerConfig

if TYPE_CHECKING:
    pass

# We import JsonValue types from the json_value package.
# These are simple dataclasses representing the six JSON types.
from json_value import (
    JsonArray,
    JsonBool,
    JsonNull,
    JsonNumber,
    JsonObject,
    JsonString,
    JsonValue,
    from_native,
)


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class JsonSerializerError(Exception):
    """Raised when a value cannot be serialized to valid JSON.

    The most common cause is attempting to serialize ``float('inf')`` or
    ``float('nan')``, which have no JSON representation. RFC 8259 only
    allows finite numbers.
    """


# ---------------------------------------------------------------------------
# String escaping — the trickiest part of serialization
# ---------------------------------------------------------------------------
#
# Why is string escaping tricky? Because there are THREE levels of escaping
# happening at the same time:
#
# 1. Python source code escaping: ``"\\n"`` in Python source is a single
#    newline character (U+000A) in memory.
#
# 2. JSON escaping: that newline character must become the TWO characters
#    ``\`` and ``n`` in the JSON output.
#
# 3. RFC 8259 requires ALL control characters (U+0000 through U+001F) to
#    be escaped, not just the "famous" ones like newline and tab.
#
# The escape table below maps Python characters (in memory) to their JSON
# escape sequences (in the output string).
# ---------------------------------------------------------------------------

# Mapping from Python character -> JSON escape sequence.
# These are the "named" escapes that RFC 8259 defines shorthand for.
_ESCAPE_TABLE: dict[str, str] = {
    '"': '\\"',       # Quotation mark — the string delimiter
    "\\": "\\\\",     # Reverse solidus (backslash) — the escape character
    "\b": "\\b",      # Backspace (U+0008)
    "\f": "\\f",      # Form feed (U+000C)
    "\n": "\\n",      # Line feed / newline (U+000A)
    "\r": "\\r",      # Carriage return (U+000D)
    "\t": "\\t",      # Horizontal tab (U+0009)
}


def _escape_json_string(s: str) -> str:
    """Escape a Python string for inclusion in JSON output.

    This function handles all three categories of characters that need
    escaping:

    1. **Named escapes**: ``"``, ``\\``, ``\\b``, ``\\f``, ``\\n``,
       ``\\r``, ``\\t`` — these have compact two-character representations.

    2. **Other control characters**: U+0000 through U+001F that aren't in
       category 1 — these use the ``\\uXXXX`` hex notation.

    3. **Everything else**: passed through unchanged. Unicode characters
       above U+001F (including emoji, CJK, etc.) are valid in JSON strings
       and do not need escaping.

    Examples
    --------
    >>> _escape_json_string('hello')
    'hello'
    >>> _escape_json_string('say "hi"')
    'say \\\\"hi\\\\"'
    >>> _escape_json_string('\\x00')
    '\\\\u0000'
    """
    # We build the result character by character. This is O(n) in the length
    # of the string and handles all cases uniformly.
    parts: list[str] = []

    for char in s:
        # Check the named escape table first (most common escapes).
        if char in _ESCAPE_TABLE:
            parts.append(_ESCAPE_TABLE[char])

        # Check for other control characters (U+0000 through U+001F).
        # The named escapes above cover some of this range, but the table
        # lookup already handled those. ord() < 0x20 catches the rest.
        elif ord(char) < 0x20:
            # Format as \uXXXX with exactly 4 hex digits, zero-padded.
            parts.append(f"\\u{ord(char):04x}")

        # All other characters pass through unchanged.
        else:
            parts.append(char)

    return "".join(parts)


# ---------------------------------------------------------------------------
# Number formatting
# ---------------------------------------------------------------------------
#
# JSON numbers are simpler than Python numbers:
# - No Infinity (JSON has no way to represent it)
# - No NaN (ditto)
# - No complex numbers (JSON is for data interchange, not math)
# - Integers have no decimal point: 42, not 42.0
# - Floats must have a decimal point or exponent: 3.14, 1e10
#
# Python's repr() for floats is *almost* correct for JSON, but we need
# to handle a few edge cases.
# ---------------------------------------------------------------------------


def _format_number(n: int | float) -> str:
    """Format a number for JSON output.

    Integers are formatted without a decimal point. Floats use Python's
    repr(), which produces the shortest string that round-trips correctly.

    Raises ``JsonSerializerError`` for Infinity and NaN, which have no
    JSON representation.

    Examples
    --------
    >>> _format_number(42)
    '42'
    >>> _format_number(3.14)
    '3.14'
    >>> _format_number(0)
    '0'
    """
    # Integers are straightforward — just convert to string.
    if isinstance(n, int):
        return str(n)

    # Floats need validation: JSON cannot represent Infinity or NaN.
    if math.isinf(n):
        raise JsonSerializerError(
            f"Cannot serialize {n} to JSON: Infinity has no JSON representation"
        )
    if math.isnan(n):
        raise JsonSerializerError(
            "Cannot serialize NaN to JSON: NaN has no JSON representation"
        )

    # Python's repr() for floats produces the shortest round-trippable string.
    # For example: repr(3.14) -> '3.14', repr(1e10) -> '10000000000.0'
    # We use repr() rather than str() because repr() guarantees round-trip
    # fidelity (float(repr(x)) == x for all finite floats).
    return repr(n)


# ---------------------------------------------------------------------------
# Core serialization — compact mode
# ---------------------------------------------------------------------------
#
# The serialize() function is a recursive dispatch on JsonValue type.
# Each JSON type has exactly one representation:
#
#   JsonNull   ->  "null"
#   JsonBool   ->  "true" or "false"
#   JsonNumber ->  formatted number
#   JsonString ->  quoted and escaped string
#   JsonArray  ->  "[" + comma-separated elements + "]"
#   JsonObject ->  "{" + comma-separated key:value pairs + "}"
#
# Empty containers are special-cased to "{}" and "[]" (no whitespace).
# ---------------------------------------------------------------------------


def serialize(value: JsonValue) -> str:
    """Serialize a JsonValue to compact JSON text.

    Produces the smallest valid JSON representation — no unnecessary
    whitespace. Suitable for wire transmission, storage, or any context
    where human readability is not a priority.

    Parameters
    ----------
    value : JsonValue
        The value to serialize. Must be one of: ``JsonNull``, ``JsonBool``,
        ``JsonNumber``, ``JsonString``, ``JsonArray``, ``JsonObject``.

    Returns
    -------
    str
        Compact JSON text.

    Raises
    ------
    JsonSerializerError
        If the value contains ``float('inf')`` or ``float('nan')``.

    Examples
    --------
    >>> serialize(JsonNull())
    'null'
    >>> serialize(JsonObject({"name": JsonString("Alice")}))
    '{"name":"Alice"}'
    """
    # --- Null ---
    # JSON null is the simplest value — always the 4-character string "null".
    if isinstance(value, JsonNull):
        return "null"

    # --- Boolean ---
    # JSON booleans are lowercase: "true" and "false" (not "True"/"False").
    if isinstance(value, JsonBool):
        return "true" if value.value else "false"

    # --- Number ---
    # Delegate to _format_number() which handles int/float distinction
    # and rejects Infinity/NaN.
    if isinstance(value, JsonNumber):
        return _format_number(value.value)

    # --- String ---
    # Wrap in double quotes and escape special characters.
    if isinstance(value, JsonString):
        return '"' + _escape_json_string(value.value) + '"'

    # --- Array ---
    # Empty arrays are "[]". Non-empty arrays are comma-separated elements
    # with no spaces: "[1,2,3]".
    if isinstance(value, JsonArray):
        if not value.elements:
            return "[]"
        parts = [serialize(elem) for elem in value.elements]
        return "[" + ",".join(parts) + "]"

    # --- Object ---
    # Empty objects are "{}". Non-empty objects are comma-separated
    # key:value pairs with no spaces: '{"a":1,"b":2}'.
    if isinstance(value, JsonObject):
        if not value.pairs:
            return "{}"
        parts = [
            '"' + _escape_json_string(key) + '":' + serialize(val)
            for key, val in value.pairs.items()
        ]
        return "{" + ",".join(parts) + "}"

    # If we reach here, the value is not a recognized JsonValue type.
    raise JsonSerializerError(  # noqa: TRY003
        f"Cannot serialize unknown type: {type(value).__name__}"
    )


# ---------------------------------------------------------------------------
# Pretty-printing — human-readable mode
# ---------------------------------------------------------------------------
#
# Pretty-printing adds two things that compact mode omits:
#
# 1. **Newlines** between array elements and object pairs
# 2. **Indentation** that increases with nesting depth
#
# Primitives (null, bool, number, string) look the same in both modes —
# they have no internal structure to indent.
#
# Empty containers ({} and []) also look the same — there's nothing
# inside to indent, so we don't add newlines.
#
# The indentation algorithm:
#   - Each nesting level adds (indent_size * indent_char) characters
#   - Opening brackets are followed by a newline
#   - Each element/pair is preceded by (depth+1) indentation units
#   - Elements/pairs are separated by comma + newline
#   - Closing brackets are preceded by (depth) indentation units
#
# Example with indent_size=2, indent_char=' ':
#   {          <-- depth 0
#     "a": [   <-- depth 1
#       1,     <-- depth 2
#       2      <-- depth 2
#     ]        <-- depth 1
#   }          <-- depth 0
# ---------------------------------------------------------------------------


def _serialize_pretty_recursive(
    value: JsonValue,
    config: SerializerConfig,
    depth: int,
) -> str:
    """Internal recursive helper for pretty-printing.

    This is separated from the public ``serialize_pretty()`` so we can
    track the ``depth`` parameter internally without exposing it in the
    public API.

    Parameters
    ----------
    value : JsonValue
        The value to serialize.
    config : SerializerConfig
        Formatting configuration.
    depth : int
        Current indentation depth (0 for top-level).
    """
    # --- Primitives ---
    # Null, bool, number, string look identical in compact and pretty modes.
    if isinstance(value, (JsonNull, JsonBool, JsonNumber, JsonString)):
        return serialize(value)

    # Compute indentation strings:
    # - indent_unit: one "level" of indentation (e.g., "  " for 2-space)
    # - current_indent: indentation for closing bracket (depth levels)
    # - next_indent: indentation for content (depth+1 levels)
    indent_unit = config.indent_char * config.indent_size
    current_indent = indent_unit * depth
    next_indent = indent_unit * (depth + 1)

    # --- Array ---
    if isinstance(value, JsonArray):
        # Empty arrays are always "[]" — no whitespace inside.
        if not value.elements:
            return "[]"

        # Each element goes on its own line, indented one level deeper.
        lines = [
            next_indent + _serialize_pretty_recursive(elem, config, depth + 1)
            for elem in value.elements
        ]
        return "[\n" + ",\n".join(lines) + "\n" + current_indent + "]"

    # --- Object ---
    if isinstance(value, JsonObject):
        # Empty objects are always "{}" — no whitespace inside.
        if not value.pairs:
            return "{}"

        # Optionally sort keys alphabetically for deterministic output.
        keys = sorted(value.pairs.keys()) if config.sort_keys else value.pairs.keys()

        # Each key-value pair goes on its own line.
        # Note the ": " (colon + space) between key and value — this is
        # the pretty-print convention that makes JSON more readable.
        lines = []
        for key in keys:
            val_str = _serialize_pretty_recursive(
                value.pairs[key], config, depth + 1
            )
            lines.append(
                next_indent + '"' + _escape_json_string(key) + '": ' + val_str
            )
        return "{\n" + ",\n".join(lines) + "\n" + current_indent + "}"

    raise JsonSerializerError(  # noqa: TRY003
        f"Cannot serialize unknown type: {type(value).__name__}"
    )


def serialize_pretty(
    value: JsonValue,
    config: SerializerConfig | None = None,
) -> str:
    """Serialize a JsonValue to pretty-printed JSON text.

    Uses configurable indentation, optional key sorting, and optional
    trailing newline. If no config is provided, uses 2-space indentation
    with no key sorting and no trailing newline.

    Parameters
    ----------
    value : JsonValue
        The value to serialize.
    config : SerializerConfig or None
        Formatting options. Defaults to ``SerializerConfig()`` if None.

    Returns
    -------
    str
        Pretty-printed JSON text.

    Examples
    --------
    >>> serialize_pretty(JsonObject({"a": JsonNumber(1)}))
    '{\\n  "a": 1\\n}'
    """
    if config is None:
        config = SerializerConfig()

    result = _serialize_pretty_recursive(value, config, depth=0)

    # Optionally append a trailing newline — useful when writing JSON to
    # files, since POSIX convention expects files to end with newline.
    if config.trailing_newline:
        result += "\n"

    return result


# ---------------------------------------------------------------------------
# Convenience API — native Python types to JSON text
# ---------------------------------------------------------------------------
#
# These functions combine two steps:
#   1. Convert native Python types to JsonValue (via json_value.from_native)
#   2. Serialize the JsonValue to text (via serialize/serialize_pretty)
#
# This is the "happy path" for most users who just want:
#   stringify({"name": "Alice"})  -->  '{"name":"Alice"}'
# ---------------------------------------------------------------------------


def stringify(
    value: dict | list | str | int | float | bool | None,
) -> str:
    """Convert native Python types to compact JSON text.

    This is a convenience function equivalent to::

        serialize(from_native(value))

    Parameters
    ----------
    value : dict, list, str, int, float, bool, or None
        Any JSON-compatible Python value. Dicts must have string keys.

    Returns
    -------
    str
        Compact JSON text.

    Examples
    --------
    >>> stringify({"name": "Alice", "age": 30})
    '{"name":"Alice","age":30}'
    >>> stringify([1, 2, 3])
    '[1,2,3]'
    >>> stringify(None)
    'null'
    """
    return serialize(from_native(value))


def stringify_pretty(
    value: dict | list | str | int | float | bool | None,
    config: SerializerConfig | None = None,
) -> str:
    """Convert native Python types to pretty-printed JSON text.

    This is a convenience function equivalent to::

        serialize_pretty(from_native(value), config)

    Parameters
    ----------
    value : dict, list, str, int, float, bool, or None
        Any JSON-compatible Python value. Dicts must have string keys.
    config : SerializerConfig or None
        Formatting options. Defaults to ``SerializerConfig()`` if None.

    Returns
    -------
    str
        Pretty-printed JSON text.

    Examples
    --------
    >>> stringify_pretty({"a": 1})
    '{\\n  "a": 1\\n}'
    """
    return serialize_pretty(from_native(value), config)
