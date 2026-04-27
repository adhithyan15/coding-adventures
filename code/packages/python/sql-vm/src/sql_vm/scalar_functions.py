"""
Built-in scalar functions
=========================

This module provides the registry of every scalar SQL function that
``CallScalar`` dispatches to.  A *scalar* function takes a fixed number of
concrete :data:`SqlValue` arguments and returns one :data:`SqlValue`.
Aggregate functions (``COUNT``, ``SUM``, …) are handled by separate VM
instructions and live in :mod:`sql_vm.vm`.

Design
------

Each function is registered with :func:`register` (or its alias
:func:`scalar`) under one or more lower-cased SQL names.  The dispatch
table is a plain ``dict[str, Callable[..., SqlValue]]``.  Look-ups are
O(1); function bodies are thin wrappers that match SQLite semantics as
closely as possible.

NULL propagation
~~~~~~~~~~~~~~~~

SQL has a *propagating NULL* rule: any function that receives a ``NULL``
argument should return ``NULL`` unless it is specifically designed to handle
``NULL`` inputs (like ``COALESCE``, ``IFNULL``, ``TYPEOF``).  We implement
this with the :func:`null_propagating` decorator that short-circuits to
``None`` when any argument is ``None``.

SQLite compat notes
~~~~~~~~~~~~~~~~~~~

- ``SUBSTR`` is 1-indexed (first character is position 1) — matches SQLite.
- ``ROUND(x)`` with no precision rounds to 0 decimal places.
- ``TYPEOF`` returns the type string SQLite uses: ``"null"``, ``"integer"``,
  ``"real"``, ``"text"``, ``"blob"``.
- ``CAST`` delegates to Python's type coercions, matching SQLite's affinity
  rules as closely as possible.
- Math functions (``SQRT``, ``LOG``, etc.) return ``NULL`` for out-of-domain
  inputs rather than raising — matching SQLite 3.35+ ``math.*`` functions.
- ``RANDOM()`` returns a random 64-bit signed integer (same range as SQLite).
- ``RANDOMBLOB(n)`` returns *n* random bytes.
- ``ZEROBLOB(n)`` returns *n* zero bytes.
- ``HEX(x)`` converts a blob or text to its hex representation.
- ``SOUNDEX`` is the standard Russell soundex algorithm.
- ``PRINTF`` / ``FORMAT`` implement the SQLite subset of C-style printf:
  ``%d``, ``%i``, ``%u``, ``%f``, ``%e``, ``%g``, ``%s``, ``%q``
  (SQL-escaped), ``%Q`` (SQL-escaped or NULL), ``%%``.
"""

from __future__ import annotations

import math
import os
import re
import struct
from collections.abc import Callable

from sql_backend.values import SqlValue

from .errors import UnsupportedFunction, WrongNumberOfArguments

# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------

_REGISTRY: dict[str, Callable[..., SqlValue]] = {}


def register(*names: str) -> Callable:  # type: ignore[type-arg]
    """Decorator: register a function under one or more SQL names."""
    def _dec(fn: Callable) -> Callable:  # type: ignore[type-arg]
        for name in names:
            _REGISTRY[name.lower()] = fn
        return fn
    return _dec


def call(name: str, args: list[SqlValue]) -> SqlValue:
    """Dispatch *name* with *args*.

    Raises :class:`~sql_vm.errors.UnsupportedFunction` for unknown names.
    """
    fn = _REGISTRY.get(name)
    if fn is None:
        raise UnsupportedFunction(name=name)
    return fn(*args)


def _arity(name: str, args: list[SqlValue], *counts: int) -> None:
    """Raise :class:`~sql_vm.errors.WrongNumberOfArguments` if len(args)
    not in *counts*.
    """
    if len(args) not in counts:
        expected = " or ".join(str(c) for c in counts)
        raise WrongNumberOfArguments(name=name, expected=expected, got=len(args))


def null_propagating(fn: Callable) -> Callable:  # type: ignore[type-arg]
    """Decorator: return NULL immediately if any argument is NULL."""
    def _wrapper(*args: SqlValue) -> SqlValue:
        if any(a is None for a in args):
            return None
        return fn(*args)
    # Preserve the function name for error messages.
    _wrapper.__name__ = fn.__name__
    return _wrapper


# ---------------------------------------------------------------------------
# NULL-handling functions (intentionally receive NULL)
# ---------------------------------------------------------------------------


@register("coalesce")
def _coalesce(*args: SqlValue) -> SqlValue:
    """Return the first non-NULL argument, or NULL if all are NULL.

    ``COALESCE(a, b, c)`` is equivalent to ``CASE WHEN a IS NOT NULL THEN a
    WHEN b IS NOT NULL THEN b ... END``.  Accepts 1 or more arguments.

    Examples::

        COALESCE(NULL, 2, 3)   → 2
        COALESCE(NULL, NULL)   → NULL
        COALESCE(1)            → 1
    """
    for a in args:
        if a is not None:
            return a
    return None


@register("ifnull")
def _ifnull(x: SqlValue, y: SqlValue) -> SqlValue:
    """Return *x* if it is not NULL, else *y*.  Synonym for ``COALESCE(x, y)``."""
    return x if x is not None else y


@register("nullif")
def _nullif(x: SqlValue, y: SqlValue) -> SqlValue:
    """Return NULL if *x* equals *y*, else *x*.

    Useful for turning sentinel values into proper NULLs::

        NULLIF(score, 0)   → NULL when score = 0, else score
    """
    if x is None and y is None:
        return None
    return None if x == y else x


@register("iif")
def _iif(condition: SqlValue, true_val: SqlValue, false_val: SqlValue) -> SqlValue:
    """Inline IF: return *true_val* when *condition* is truthy, else *false_val*.

    ``IIF(a, b, c)`` is equivalent to ``CASE WHEN a THEN b ELSE c END``.
    A NULL condition is treated as false.
    """
    return true_val if condition else false_val


# ---------------------------------------------------------------------------
# Type inspection
# ---------------------------------------------------------------------------


@register("typeof")
def _typeof(x: SqlValue) -> SqlValue:
    """Return the SQLite type name of *x* as a lower-cased text string.

    Possible return values: ``"null"``, ``"integer"``, ``"real"``,
    ``"text"``, ``"blob"``.  Note that booleans are stored as integers
    in SQLite, so ``TYPEOF(TRUE)`` → ``"integer"``.

    Examples::

        TYPEOF(NULL)   → "null"
        TYPEOF(42)     → "integer"
        TYPEOF(3.14)   → "real"
        TYPEOF("hi")   → "text"
        TYPEOF(X'FF')  → "blob"
    """
    if x is None:
        return "null"
    if isinstance(x, bool):
        return "integer"
    if isinstance(x, int):
        return "integer"
    if isinstance(x, float):
        return "real"
    if isinstance(x, str):
        return "text"
    if isinstance(x, (bytes, bytearray)):
        return "blob"
    return "text"


# ---------------------------------------------------------------------------
# CAST
# ---------------------------------------------------------------------------


@register("cast")
def _cast_fn(x: SqlValue, target_type: SqlValue) -> SqlValue:
    """Cast *x* to the SQL type named by *target_type* (a TEXT string).

    Follows SQLite's type affinity rules:

    - ``"integer"`` / ``"int"`` → Python ``int`` (truncate if float)
    - ``"real"`` / ``"float"`` / ``"double"`` / ``"numeric"`` → Python ``float``
    - ``"text"`` / ``"varchar"`` / ``"char"`` → Python ``str``
    - ``"blob"`` / ``"none"`` → Python ``bytes``
    - ``"boolean"`` → Python ``bool`` (True if truthy)

    NULL input → NULL output.  Unknown target type → returns *x* unchanged.
    """
    if x is None:
        return None
    if not isinstance(target_type, str):
        return x
    t = target_type.strip().lower()
    try:
        if t in ("integer", "int", "int2", "int8", "tinyint", "smallint",
                 "mediumint", "bigint", "unsigned big int"):
            if isinstance(x, float):
                return int(x)
            if isinstance(x, str):
                # Try integer first, then float-to-int.
                try:
                    return int(x)
                except ValueError:
                    try:
                        return int(float(x))
                    except ValueError:
                        return 0
            return int(bool(x)) if isinstance(x, bool) else int(x)
        if t in ("real", "float", "double", "double precision",
                 "numeric", "decimal"):
            if isinstance(x, bool):
                return float(int(x))
            if isinstance(x, (int, float)):
                return float(x)
            if isinstance(x, str):
                try:
                    return float(x)
                except ValueError:
                    return 0.0
            return float(len(x))  # blob → length as float
        if t in ("text", "varchar", "nvarchar", "character", "char",
                 "varying character", "nchar", "native character",
                 "clob"):
            if isinstance(x, bytes):
                return x.hex()
            return str(x)
        if t in ("blob", "none"):
            if isinstance(x, bytes):
                return x
            if isinstance(x, str):
                return x.encode("utf-8")
            if isinstance(x, int):
                return struct.pack(">q", x)
            if isinstance(x, float):
                return struct.pack(">d", x)
            return bytes(x)  # type: ignore[call-overload]
        if t in ("boolean", "bool"):
            return bool(x)
    except (ValueError, TypeError, OverflowError):
        pass
    return x


# ---------------------------------------------------------------------------
# Numeric functions
# ---------------------------------------------------------------------------


@register("abs")
@null_propagating
def _abs(x: SqlValue) -> SqlValue:
    """Return the absolute value of *x*.

    Returns NULL for NULL input (handled by ``null_propagating``).
    Returns *x* unchanged for non-numeric types.

    Examples::

        ABS(-5)     → 5
        ABS(-3.14)  → 3.14
        ABS(NULL)   → NULL
    """
    if isinstance(x, (int, float)):
        return abs(x)  # type: ignore[arg-type]
    return x


@register("round")
def _round(*args: SqlValue) -> SqlValue:
    """Round *x* to *digits* decimal places (default 0).

    ``ROUND(x)`` → integer number of decimal places (0).
    ``ROUND(x, n)`` → rounded to *n* places.  Negative *n* rounds to the
    left of the decimal point.

    Returns NULL for NULL *x*.

    Examples::

        ROUND(3.14159, 2)  → 3.14
        ROUND(3.5)         → 4.0
        ROUND(-3.5)        → -4.0
    """
    _arity("round", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, (int, float)):
        return x
    n = int(args[1]) if len(args) == 2 and args[1] is not None else 0
    return round(float(x), n)  # type: ignore[return-value]


@register("ceil", "ceiling")
@null_propagating
def _ceil(x: SqlValue) -> SqlValue:
    """Return the smallest integer ≥ *x* (ceiling).

    Examples::

        CEIL(3.2)   → 4.0
        CEIL(-3.2)  → -3.0
    """
    if isinstance(x, (int, float)):
        return float(math.ceil(x))  # type: ignore[arg-type]
    return x


@register("floor")
@null_propagating
def _floor(x: SqlValue) -> SqlValue:
    """Return the largest integer ≤ *x* (floor).

    Examples::

        FLOOR(3.8)   → 3.0
        FLOOR(-3.2)  → -4.0
    """
    if isinstance(x, (int, float)):
        return float(math.floor(x))  # type: ignore[arg-type]
    return x


@register("sign")
@null_propagating
def _sign(x: SqlValue) -> SqlValue:
    """Return -1, 0, or 1 depending on the sign of *x*.

    Examples::

        SIGN(-5)   → -1
        SIGN(0)    → 0
        SIGN(3.7)  → 1
    """
    if isinstance(x, (int, float)):
        v = x  # type: ignore[assignment]
        return 0 if v == 0 else (1 if v > 0 else -1)
    return x


@register("mod")
@null_propagating
def _mod(x: SqlValue, y: SqlValue) -> SqlValue:
    """Return *x* modulo *y*.

    Returns NULL for NULL inputs.  Returns NULL (not an error) for
    division-by-zero — matching SQLite's ``x % 0 → NULL`` behaviour.

    Examples::

        MOD(10, 3)   → 1
        MOD(10, 0)   → NULL
    """
    if isinstance(x, (int, float)) and isinstance(y, (int, float)):
        if y == 0:
            return None
        return x % y  # type: ignore[operator]
    return None


# ---------------------------------------------------------------------------
# Math functions (SQLite 3.35+ math module equivalents)
# ---------------------------------------------------------------------------

def _safe_math(fn: Callable[[float], float], x: SqlValue) -> SqlValue:
    """Apply *fn* to *x*, returning NULL on domain error or non-numeric input."""
    if x is None or not isinstance(x, (int, float)):
        return None
    try:
        result = fn(float(x))  # type: ignore[arg-type]
        return result if math.isfinite(result) else None
    except (ValueError, ZeroDivisionError):
        return None


@register("sqrt")
def _sqrt(x: SqlValue) -> SqlValue:
    """Return the square root of *x*.  Returns NULL for negative *x* or NULL."""
    return _safe_math(math.sqrt, x)


@register("pow", "power")
def _pow(x: SqlValue, y: SqlValue) -> SqlValue:
    """Return *x* raised to the power *y*.

    Returns NULL for NULL inputs or out-of-domain combinations (e.g. 0**−1).
    """
    if x is None or y is None:
        return None
    if not isinstance(x, (int, float)) or not isinstance(y, (int, float)):
        return None
    try:
        result = float(x) ** float(y)  # type: ignore[operator]
        return result if math.isfinite(result) else None
    except (ValueError, ZeroDivisionError, OverflowError):
        return None


@register("log", "ln")
def _log(*args: SqlValue) -> SqlValue:
    """Natural logarithm (1 arg) or log base B (2 args: ``LOG(B, x)``).

    ``LOG(x)``    → natural log of *x*.
    ``LOG(B, x)`` → log base *B* of *x*.

    Returns NULL for non-positive inputs.
    """
    _arity("log", list(args), 1, 2)
    if len(args) == 1:
        return _safe_math(math.log, args[0])
    base, x = args[0], args[1]
    if base is None or x is None:
        return None
    if not isinstance(base, (int, float)) or not isinstance(x, (int, float)):
        return None
    try:
        result = math.log(float(x), float(base))  # type: ignore[arg-type]
        return result if math.isfinite(result) else None
    except (ValueError, ZeroDivisionError):
        return None


@register("log2")
def _log2(x: SqlValue) -> SqlValue:
    """Return log base 2 of *x*.  Returns NULL for non-positive *x* or NULL."""
    return _safe_math(math.log2, x)


@register("log10")
def _log10(x: SqlValue) -> SqlValue:
    """Return log base 10 of *x*.  Returns NULL for non-positive *x* or NULL."""
    return _safe_math(math.log10, x)


@register("exp")
def _exp(x: SqlValue) -> SqlValue:
    """Return *e* raised to *x*.  Returns NULL for overflow or NULL input."""
    return _safe_math(math.exp, x)


@register("pi")
def _pi() -> SqlValue:
    """Return the mathematical constant π ≈ 3.141592653589793."""
    return math.pi


@register("sin")
def _sin(x: SqlValue) -> SqlValue:
    """Return the sine of *x* (radians)."""
    return _safe_math(math.sin, x)


@register("cos")
def _cos(x: SqlValue) -> SqlValue:
    """Return the cosine of *x* (radians)."""
    return _safe_math(math.cos, x)


@register("tan")
def _tan(x: SqlValue) -> SqlValue:
    """Return the tangent of *x* (radians).  Returns NULL at π/2 + nπ."""
    return _safe_math(math.tan, x)


@register("asin")
def _asin(x: SqlValue) -> SqlValue:
    """Return the arcsine of *x* in radians.  NULL for |x| > 1."""
    return _safe_math(math.asin, x)


@register("acos")
def _acos(x: SqlValue) -> SqlValue:
    """Return the arccosine of *x* in radians.  NULL for |x| > 1."""
    return _safe_math(math.acos, x)


@register("atan")
def _atan(*args: SqlValue) -> SqlValue:
    """Return arctan.

    ``ATAN(x)``    → arctan of *x* in radians.
    ``ATAN(y, x)`` → arctan2(y, x) — the angle from the +X axis to the
    point (*x*, *y*), in radians.
    """
    _arity("atan", list(args), 1, 2)
    if len(args) == 1:
        return _safe_math(math.atan, args[0])
    y, x = args[0], args[1]
    if y is None or x is None:
        return None
    if not isinstance(y, (int, float)) or not isinstance(x, (int, float)):
        return None
    try:
        return math.atan2(float(y), float(x))  # type: ignore[arg-type]
    except (ValueError, ZeroDivisionError):
        return None


@register("atan2")
def _atan2(y: SqlValue, x: SqlValue) -> SqlValue:
    """Return arctan2(y, x) — the angle from the positive X axis to (x, y)."""
    if y is None or x is None:
        return None
    if not isinstance(y, (int, float)) or not isinstance(x, (int, float)):
        return None
    try:
        return math.atan2(float(y), float(x))  # type: ignore[arg-type]
    except (ValueError, ZeroDivisionError):
        return None


@register("degrees")
def _degrees(x: SqlValue) -> SqlValue:
    """Convert *x* from radians to degrees."""
    return _safe_math(math.degrees, x)


@register("radians")
def _radians(x: SqlValue) -> SqlValue:
    """Convert *x* from degrees to radians."""
    return _safe_math(math.radians, x)


# ---------------------------------------------------------------------------
# String functions
# ---------------------------------------------------------------------------


@register("upper")
@null_propagating
def _upper(x: SqlValue) -> SqlValue:
    """Convert *x* to upper case.

    Only ASCII characters are case-folded (matching SQLite's ``UPPER``
    which does not handle Unicode case conversion).

    Examples::

        UPPER("hello")  → "HELLO"
        UPPER(NULL)     → NULL
    """
    if isinstance(x, str):
        return x.upper()
    return x


@register("lower")
@null_propagating
def _lower(x: SqlValue) -> SqlValue:
    """Convert *x* to lower case.

    Examples::

        LOWER("HELLO")  → "hello"
        LOWER(NULL)     → NULL
    """
    if isinstance(x, str):
        return x.lower()
    return x


@register("length", "len")
def _length(x: SqlValue) -> SqlValue:
    """Return the number of characters in a TEXT string or bytes in a BLOB.

    Returns NULL for NULL input, 0 for empty strings.

    For BLOB values, returns the number of bytes.  For TEXT, the number of
    characters (not bytes — matching SQLite's UTF-8 semantics for BMP text).

    Examples::

        LENGTH("hello")   → 5
        LENGTH("")        → 0
        LENGTH(NULL)      → NULL
        LENGTH(X'AABB')   → 2
    """
    if x is None:
        return None
    if isinstance(x, str):
        return len(x)
    if isinstance(x, (bytes, bytearray)):
        return len(x)
    # Numeric — convert to string first (SQLite: LENGTH(42) → 2).
    return len(str(x))


@register("trim")
def _trim(*args: SqlValue) -> SqlValue:
    """Strip leading and trailing characters from *x*.

    ``TRIM(x)``        → strip whitespace.
    ``TRIM(x, chars)`` → strip any character in *chars*.

    Returns NULL for NULL *x*.

    Examples::

        TRIM("  hello  ")        → "hello"
        TRIM("xxhelloxx", "x")   → "hello"
    """
    _arity("trim", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, str):
        return x
    if len(args) == 2:
        chars = args[1]
        return x.strip(str(chars) if chars is not None else None)
    return x.strip()


@register("ltrim")
def _ltrim(*args: SqlValue) -> SqlValue:
    """Strip leading characters from *x*.

    ``LTRIM(x)``        → strip leading whitespace.
    ``LTRIM(x, chars)`` → strip any leading character in *chars*.

    Returns NULL for NULL *x*.
    """
    _arity("ltrim", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, str):
        return x
    if len(args) == 2:
        chars = args[1]
        return x.lstrip(str(chars) if chars is not None else None)
    return x.lstrip()


@register("rtrim")
def _rtrim(*args: SqlValue) -> SqlValue:
    """Strip trailing characters from *x*.

    ``RTRIM(x)``        → strip trailing whitespace.
    ``RTRIM(x, chars)`` → strip any trailing character in *chars*.

    Returns NULL for NULL *x*.
    """
    _arity("rtrim", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, str):
        return x
    if len(args) == 2:
        chars = args[1]
        return x.rstrip(str(chars) if chars is not None else None)
    return x.rstrip()


@register("substr", "substring")
def _substr(*args: SqlValue) -> SqlValue:
    """Extract a substring.

    ``SUBSTR(x, start)``         → from position *start* to end.
    ``SUBSTR(x, start, length)`` → *length* characters starting at *start*.

    *start* is **1-indexed** (first character = position 1), matching SQLite.
    Negative *start* counts from the end (position −1 = last character).
    A *length* of 0 or negative returns an empty string.

    Returns NULL if *x* is NULL.

    Examples::

        SUBSTR("hello", 2)       → "ello"
        SUBSTR("hello", 2, 3)    → "ell"
        SUBSTR("hello", -3)      → "llo"
        SUBSTR("hello", 2, 0)    → ""
    """
    _arity("substr", list(args), 2, 3)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, str):
        if isinstance(x, (bytes, bytearray)):
            # Blob substr — operate on bytes.
            s = bytes(x)
            start = int(args[1]) if args[1] is not None else 1  # type: ignore[arg-type]
            if start > 0:
                start -= 1  # Convert to 0-indexed.
            # negative start: count from end
            idx = start if start < 0 else start
            if len(args) == 3 and args[2] is not None:
                length = int(args[2])  # type: ignore[arg-type]
                if length <= 0:
                    return b""
                return s[idx: idx + length]
            return s[idx:]
        return x
    start = int(args[1]) if args[1] is not None else 1  # type: ignore[arg-type]
    if start > 0:
        start -= 1  # Convert to 0-indexed.
    # Negative start: SQLite counts from end (−1 = last char).
    if len(args) == 3 and args[2] is not None:
        length = int(args[2])  # type: ignore[arg-type]
        if length <= 0:
            return ""
        if start < 0:
            end = start + length
            if end <= 0:
                # Entire span is before the start of the string.
                start_pos = len(x) + start
                if start_pos < 0:
                    start_pos = 0
                return x[start_pos: start_pos + length]
            if end < 0:
                return x[start:end]
            stop = None if start + length > len(x) else start + length
            return x[start:stop]
        return x[start: start + length]
    if start < 0:
        return x[start:]
    return x[start:]


@register("replace")
@null_propagating
def _replace(x: SqlValue, old: SqlValue, new: SqlValue) -> SqlValue:
    """Replace all occurrences of *old* in *x* with *new*.

    Returns NULL if any argument is NULL (handled by ``null_propagating``).

    Examples::

        REPLACE("hello world", "world", "SQL")  → "hello SQL"
        REPLACE("aaa", "a", "bb")               → "bbbbbb"
    """
    if isinstance(x, str) and isinstance(old, str) and isinstance(new, str):
        return x.replace(old, new)
    return x


@register("instr")
def _instr(x: SqlValue, needle: SqlValue) -> SqlValue:
    """Return the 1-based index of the first occurrence of *needle* in *x*.

    Returns 0 if *needle* is not found.  Returns NULL if either argument
    is NULL.

    Examples::

        INSTR("hello", "ll")   → 3
        INSTR("hello", "xyz")  → 0
        INSTR("hello", "")     → 1
        INSTR(NULL, "x")       → NULL
    """
    if x is None or needle is None:
        return None
    if isinstance(x, str) and isinstance(needle, str):
        idx = x.find(needle)
        return idx + 1 if idx >= 0 else 0
    if isinstance(x, (bytes, bytearray)) and isinstance(needle, (bytes, bytearray)):
        idx = bytes(x).find(bytes(needle))
        return idx + 1 if idx >= 0 else 0
    return 0


@register("hex")
def _hex(x: SqlValue) -> SqlValue:
    """Convert *x* to an upper-cased hexadecimal string.

    For BLOB values, encodes each byte as two hex digits (no ``0x`` prefix).
    For TEXT values, encodes the UTF-8 bytes.
    For integers, encodes the big-endian 8-byte representation.
    Returns NULL for NULL input.

    Examples::

        HEX(X'DEADBEEF')   → "DEADBEEF"
        HEX("AB")          → "4142"
        HEX(255)           → "00000000000000FF"
    """
    if x is None:
        return None
    if isinstance(x, (bytes, bytearray)):
        return bytes(x).hex().upper()
    if isinstance(x, str):
        return x.encode("utf-8").hex().upper()
    if isinstance(x, bool):
        return struct.pack(">q", int(x)).hex().upper()
    if isinstance(x, int):
        return struct.pack(">q", x).hex().upper()
    if isinstance(x, float):
        return struct.pack(">d", x).hex().upper()
    return str(x)


@register("unhex")
def _unhex(*args: SqlValue) -> SqlValue:
    """Convert a hexadecimal string to a BLOB.

    ``UNHEX(hex_string)`` → BLOB.
    ``UNHEX(hex_string, ignore_chars)`` → decode, skipping characters in
    *ignore_chars* (e.g. spaces, colons).

    Returns NULL for NULL input or malformed hex strings.

    Examples::

        UNHEX("DEADBEEF")        → b"\\xde\\xad\\xbe\\xef"
        UNHEX("DE AD", " ")      → b"\\xde\\xad"
    """
    _arity("unhex", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    if not isinstance(x, str):
        return None
    s = x
    if len(args) == 2 and args[1] is not None:
        ignore = str(args[1])
        for ch in ignore:
            s = s.replace(ch, "")
    try:
        return bytes.fromhex(s)
    except ValueError:
        return None


@register("quote")
def _quote(x: SqlValue) -> SqlValue:
    """Return a SQL literal that represents *x*, suitable for embedding in SQL.

    - NULL → ``"NULL"``
    - integers / floats → their numeric representation
    - text → single-quoted with internal single-quotes doubled
    - blob → ``X'...'`` hex literal

    This is the same as SQLite's ``QUOTE()`` function.

    Examples::

        QUOTE("hello")       → "'hello'"
        QUOTE("it's")        → "'it''s'"
        QUOTE(NULL)          → "NULL"
        QUOTE(42)            → "42"
        QUOTE(X'DEADBEEF')   → "X'DEADBEEF'"
    """
    if x is None:
        return "NULL"
    if isinstance(x, bool):
        return str(int(x))
    if isinstance(x, (int, float)):
        return str(x)
    if isinstance(x, str):
        escaped = x.replace("'", "''")
        return f"'{escaped}'"
    if isinstance(x, (bytes, bytearray)):
        return f"X'{bytes(x).hex().upper()}'"
    return f"'{x}'"


@register("char")
def _char(*args: SqlValue) -> SqlValue:
    """Return a string composed of characters with the given Unicode code points.

    ``CHAR(65, 66, 67)`` → ``"ABC"``.

    Returns NULL if any argument is NULL.

    Examples::

        CHAR(72, 101, 108, 108, 111)  → "Hello"
    """
    if any(a is None for a in args):
        return None
    try:
        return "".join(chr(int(a)) for a in args)  # type: ignore[arg-type]
    except (ValueError, TypeError, OverflowError):
        return None


@register("unicode")
def _unicode(x: SqlValue) -> SqlValue:
    """Return the Unicode code point of the first character of *x*.

    Returns NULL for NULL or empty input.

    Examples::

        UNICODE("A")      → 65
        UNICODE("hello")  → 104
        UNICODE("")       → NULL
        UNICODE(NULL)     → NULL
    """
    if x is None:
        return None
    if isinstance(x, str):
        if not x:
            return None
        return ord(x[0])
    if isinstance(x, (bytes, bytearray)):
        if not x:
            return None
        return x[0]
    return None


@register("zeroblob")
def _zeroblob(n: SqlValue) -> SqlValue:
    """Return a BLOB consisting of *n* zero bytes.

    Returns NULL for NULL *n*.

    Examples::

        ZEROBLOB(4)  → b"\\x00\\x00\\x00\\x00"
    """
    if n is None:
        return None
    try:
        return bytes(int(n))  # type: ignore[arg-type]
    except (TypeError, ValueError, OverflowError):
        return None


# ---------------------------------------------------------------------------
# SOUNDEX
# ---------------------------------------------------------------------------

_SOUNDEX_TABLE = str.maketrans(
    "BFPVCGJKQSXZDTLMNR",
    "111122222222334556",
)

_SOUNDEX_REMOVE = str.maketrans("", "", "AEIOUYHW")


@register("soundex")
def _soundex(x: SqlValue) -> SqlValue:
    """Return the four-character Russell Soundex code for *x*.

    Follows the standard American Soundex algorithm.  Returns ``"?000"``
    for NULL or empty strings (matching SQLite's documented behaviour).

    Examples::

        SOUNDEX("Robert")    → "R163"
        SOUNDEX("Rupert")    → "R163"
        SOUNDEX("")          → "?000"
        SOUNDEX(NULL)        → "?000"
    """
    if x is None or not isinstance(x, str) or not x:
        return "?000"
    s = x.upper()
    # Keep only ASCII letters.
    s = re.sub(r"[^A-Z]", "", s)
    if not s:
        return "?000"
    first = s[0]
    # Translate letters to codes; remove A E I O U H W Y.
    coded = s.translate(_SOUNDEX_TABLE)
    # Build the code: first letter + 3 digits.
    digits = []
    prev = coded[0]  # First char's code (may be letter if not in table)
    for ch in coded[1:]:
        if ch.isdigit() and ch != prev:
            digits.append(ch)
            if len(digits) == 3:
                break
        elif not ch.isdigit():
            prev = ""
            continue
        prev = ch
    result = first + "".join(digits).ljust(3, "0")
    return result[:4]


# ---------------------------------------------------------------------------
# PRINTF / FORMAT
# ---------------------------------------------------------------------------

_PRINTF_FMT = re.compile(
    r"%(?P<flags>[-+ #0]*)(?P<width>\d*)(?:\.(?P<prec>\d+))?(?P<conv>[diouxXeEfgGsqQ%])"
)


def _printf_format(template: str, args: list[SqlValue]) -> str:  # noqa: C901
    """Implement SQLite's subset of C-style ``printf``.

    Supported conversions:

    - ``%d``, ``%i``, ``%o``, ``%u``, ``%x``, ``%X`` — integer formatting
    - ``%f``, ``%e``, ``%E``, ``%g``, ``%G`` — float formatting
    - ``%s`` — string (None → "")
    - ``%q`` — SQL-escaped string (single-quotes doubled, wrapped in '')
    - ``%Q`` — like ``%q`` but NULL → "NULL"
    - ``%%`` — literal ``%``

    ``%w`` (table/column quoting) is not implemented.
    """
    arg_iter = iter(args)
    result: list[str] = []
    pos = 0
    for m in _PRINTF_FMT.finditer(template):
        result.append(template[pos: m.start()])
        pos = m.end()
        conv = m.group("conv")
        if conv == "%":
            result.append("%")
            continue
        try:
            arg = next(arg_iter)
        except StopIteration:
            arg = None
        flags = m.group("flags") or ""
        width_s = m.group("width")
        prec_s = m.group("prec")
        width = int(width_s) if width_s else 0
        if conv in "diouxX":
            val = 0 if arg is None else (int(arg) if not isinstance(arg, bool) else int(arg))
            spec = f"%{flags}{width_s}"
            if prec_s:
                spec += f".{prec_s}"
            spec += conv
            try:
                result.append(spec % val)
            except TypeError:
                result.append(str(val))
        elif conv in "feEgG":
            val_f = 0.0 if arg is None else float(arg)  # type: ignore[arg-type]
            spec = f"%{flags}{width_s}"
            if prec_s:
                spec += f".{prec_s}"
            spec += conv
            try:
                result.append(spec % val_f)
            except TypeError:
                result.append(str(val_f))
        elif conv == "s":
            s = "" if arg is None else str(arg)
            if prec_s:
                s = s[: int(prec_s)]
            left = "-" in flags
            if width and len(s) < width:
                pad = " " * (width - len(s))
                s = (s + pad) if left else (pad + s)
            result.append(s)
        elif conv == "q":
            if arg is None:
                s = ""
            else:
                s = str(arg).replace("'", "''")
                s = f"'{s}'"
            result.append(s)
        elif conv == "Q":
            if arg is None:
                result.append("NULL")
            else:
                s = str(arg).replace("'", "''")
                result.append(f"'{s}'")
    result.append(template[pos:])
    return "".join(result)


@register("printf", "format")
def _printf(*args: SqlValue) -> SqlValue:
    """Format a string using C-style ``printf`` syntax.

    ``PRINTF(format, arg1, arg2, ...)`` / ``FORMAT(format, arg1, arg2, ...)``

    Returns NULL if *format* is NULL.

    Examples::

        PRINTF("Hello %s!", "world")         → "Hello world!"
        PRINTF("%d + %d = %d", 1, 2, 1+2)   → "1 + 2 = 3"
        PRINTF("%.2f", 3.14159)              → "3.14"
        PRINTF("%q", "it's")                 → "'it''s'"
    """
    if not args:
        raise WrongNumberOfArguments(name="printf", expected="at least 1", got=0)
    fmt = args[0]
    if fmt is None:
        return None
    if not isinstance(fmt, str):
        return str(fmt)
    return _printf_format(fmt, list(args[1:]))


# ---------------------------------------------------------------------------
# Random / utility
# ---------------------------------------------------------------------------


@register("random")
def _random() -> SqlValue:
    """Return a pseudo-random integer in the range [−2^63, 2^63 − 1].

    Matches SQLite's ``RANDOM()`` range.

    Note: uses :func:`os.urandom` (cryptographically strong) unlike SQLite's
    internal PRNG.  This is intentionally stronger than required.

    Examples::

        RANDOM()   → some large signed integer (non-deterministic)
    """
    raw = os.urandom(8)
    n = int.from_bytes(raw, "big")
    # Convert unsigned 64-bit to signed.
    if n >= (1 << 63):
        n -= 1 << 64
    return n


@register("randomblob")
def _randomblob(n: SqlValue) -> SqlValue:
    """Return a BLOB of *n* random bytes.

    Returns NULL for NULL or non-positive *n*.

    Examples::

        RANDOMBLOB(4)   → some 4-byte BLOB (non-deterministic)
    """
    if n is None:
        return None
    try:
        count = int(n)  # type: ignore[arg-type]
    except (TypeError, ValueError):
        return None
    if count <= 0:
        return None
    return os.urandom(count)


@register("last_insert_rowid")
def _last_insert_rowid() -> SqlValue:
    """Return NULL (placeholder — real backends track this per-connection).

    A full implementation would require the VM to receive a rowid from the
    last successful ``INSERT``.  That infrastructure is deferred to a future
    release; for now this returns NULL rather than raising.
    """
    return None
