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
  ``%d``, ``%i``, ``%u``, ``%f``, ``%e``, ``%g``, ``%s``, ``%q`` (SQL
  string-literal escape, no surrounding quotes), ``%Q`` (like ``%q`` but
  wraps in single quotes, or emits the literal ``NULL``), ``%w`` (SQL
  identifier escape — double quotes doubled, no surrounding quotes),
  ``%%``.
"""

from __future__ import annotations

import calendar
import copy
import json as _json
import math
import os
import re
from collections.abc import Callable
from datetime import UTC, datetime, timedelta

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


# ---------------------------------------------------------------------------
# SQLite-compatible numeric-prefix parsers
# ---------------------------------------------------------------------------
#
# SQLite's string-to-number coercion does NOT use Python's ``int()``/
# ``float()`` semantics — instead it takes the **longest valid numeric
# prefix** and ignores any trailing garbage.  So ``CAST('1.5abc' AS REAL)``
# returns 1.5 (the float prefix), and ``CAST('123abc' AS INTEGER)`` returns
# 123 (just the int prefix; SQLite's INTEGER cast specifically rejects the
# decimal point and exponent).
#
# Python's ``float('inf')`` produces an infinity but SQLite rejects the
# literal text "inf"/"nan"/"infinity" — there is no leading digit so the
# numeric prefix is empty, hence 0.0.  Same for "abc".  These regexes
# capture the SQLite rule directly: optional whitespace, optional sign,
# then either digits or digits-with-decimals-and-exponent.

_INT_PREFIX = re.compile(r"^\s*[+-]?\d+")
_REAL_PREFIX = re.compile(
    r"^\s*[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?"
)


# SQLite's INTEGER type is a signed 64-bit value.  Out-of-range numeric
# casts saturate at the int64 endpoints rather than wrapping or raising.
_INT64_MAX = 2**63 - 1
_INT64_MIN = -(2**63)


def _clamp_int64(value: int) -> int:
    """Clamp *value* to the signed 64-bit integer range.

    SQLite's INTEGER affinity is an signed 64-bit value; CAST to
    INTEGER saturates at ``-2**63`` / ``2**63 - 1`` rather than
    wrapping or producing a Python bigint.  Apply this clamp to every
    INTEGER cast result so callers see SQLite-compatible output.
    """
    if value > _INT64_MAX:
        return _INT64_MAX
    if value < _INT64_MIN:
        return _INT64_MIN
    return value


def _sqlite_str_to_int(s: str) -> int:
    """Take the longest leading integer prefix of *s*; 0 if none.

    The result is clamped to the signed 64-bit range, matching SQLite's
    INTEGER affinity: ``CAST('99999999999999999999' AS INTEGER)`` gives
    ``9223372036854775807``, not the unclamped Python bigint.
    """
    m = _INT_PREFIX.match(s)
    if not m:
        return 0
    try:
        return _clamp_int64(int(m.group().strip()))
    except (ValueError, OverflowError):
        return 0


def _sqlite_str_to_real(s: str) -> float:
    """Take the longest leading float prefix of *s*; 0.0 if none.

    Matches SQLite's behaviour of rejecting non-numeric prefixes (so
    ``'inf'`` → 0.0, not Python's ``float('inf')``).
    """
    m = _REAL_PREFIX.match(s)
    if not m:
        return 0.0
    prefix = m.group().strip()
    # A bare sign or empty match isn't a valid float — bail to 0.0.
    if prefix in ("", "+", "-", "."):
        return 0.0
    try:
        return float(prefix)
    except (ValueError, OverflowError):
        return 0.0


@register("cast")
def _cast_fn(x: SqlValue, target_type: SqlValue) -> SqlValue:
    """Cast *x* to the SQL type named by *target_type* (a TEXT string).

    Follows SQLite's type affinity rules:

    - ``"integer"`` / ``"int"`` → Python ``int`` (longest leading int
      prefix; truncate floats toward zero).  Crucially, the *string*
      INTEGER cast in SQLite extracts only the digit prefix — so
      ``CAST('1.5abc' AS INTEGER)`` is ``1`` (just ``1``, not ``1.5``
      truncated), and ``CAST('1e5' AS INTEGER)`` is also ``1``.
    - ``"real"`` / ``"float"`` / ``"double"`` / ``"numeric"`` → Python
      ``float`` (longest leading float prefix, including optional sign,
      decimal point, and ``e``/``E`` exponent).  Non-numeric strings —
      including the literal text ``"inf"`` and ``"nan"`` — coerce to
      ``0.0`` because SQLite has no special-case for those keywords.
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
            # All four numeric→int paths flow through ``_clamp_int64`` so
            # the result fits in a signed 64-bit value, matching SQLite's
            # INTEGER affinity.  Truncation (float→int) happens *before*
            # the clamp so ``CAST(1e20 AS INTEGER)`` yields ``int64_max``
            # rather than overflowing Python's int conversion.
            if isinstance(x, bool):
                return int(x)
            if isinstance(x, float):
                # Float→int: truncate toward zero, then clamp.  ``float``
                # values outside int64 range (e.g. ``1e20``, ``inf``)
                # would raise OverflowError from ``int()`` — handle by
                # saturating to the appropriate endpoint.
                try:
                    return _clamp_int64(int(x))
                except (OverflowError, ValueError):
                    return _INT64_MAX if x > 0 else _INT64_MIN
            if isinstance(x, str):
                return _sqlite_str_to_int(x)
            if isinstance(x, bytes):
                return _sqlite_str_to_int(x.decode("utf-8", errors="replace"))
            return _clamp_int64(int(x))
        if t in ("real", "float", "double", "double precision",
                 "numeric", "decimal"):
            if isinstance(x, bool):
                return float(int(x))
            if isinstance(x, (int, float)):
                return float(x)
            if isinstance(x, str):
                return _sqlite_str_to_real(x)
            return float(len(x))  # blob → length as float (legacy quirk)
        if t in ("text", "varchar", "nvarchar", "character", "char",
                 "varying character", "nchar", "native character",
                 "clob"):
            if isinstance(x, bytes):
                # SQLite's BLOB→TEXT cast UTF-8-decodes the bytes
                # (treating them as the encoded text representation),
                # NOT hex-encodes them.  So ``CAST(x'48656c6c6f' AS
                # TEXT)`` is ``'Hello'`` and ``CAST(CAST(42 AS BLOB)
                # AS TEXT)`` round-trips to ``'42'``.  Invalid UTF-8
                # bytes are replaced with U+FFFD via ``errors="replace"``
                # so a malformed blob can never raise UnicodeDecodeError
                # mid-query (matches SQLite's lenient decoding).
                return x.decode("utf-8", errors="replace")
            # SQLite has no native boolean type — TRUE / FALSE round-trip
            # as integers 1 / 0.  ``CAST(TRUE AS TEXT)`` must therefore
            # yield ``'1'``, not Python's ``'True'``.  Check ``bool``
            # before the generic ``str`` path because ``bool`` is a
            # subclass of ``int`` and would otherwise be caught by it
            # under the existing INTEGER affinity path.
            if isinstance(x, bool):
                return str(int(x))
            return str(x)
        if t in ("blob", "none"):
            if isinstance(x, bytes):
                return x
            if isinstance(x, str):
                return x.encode("utf-8")
            # SQLite's numeric→BLOB cast goes through the TEXT
            # representation first: ``CAST(1 AS BLOB)`` yields
            # ``b'1'`` (one byte) — the UTF-8 encoding of the
            # integer's decimal string — not an 8-byte big-endian
            # packed int.  The same applies to floats (``b'1.5'``)
            # and booleans (``CAST(TRUE AS BLOB)`` → ``b'1'``).
            # Check ``bool`` before ``int`` because Python's
            # ``bool`` is a subclass of ``int``.
            if isinstance(x, bool):
                return str(int(x)).encode("utf-8")
            if isinstance(x, int):
                return str(x).encode("utf-8")
            if isinstance(x, float):
                return str(x).encode("utf-8")
            return bytes(x)  # type: ignore[call-overload]
        if t in ("boolean", "bool"):
            return bool(x)
    except (ValueError, TypeError, OverflowError):
        pass
    return x


# ---------------------------------------------------------------------------
# GLOB — case-sensitive Unix-style glob pattern matching
# ---------------------------------------------------------------------------
#
# SQL form:    string GLOB pattern
# Internal:    glob(pattern, string)
#
# The argument order matches SQLite's C API: ``glob(Y, X)`` where Y is the
# pattern and X is the string being tested.  This reversal from the SQL form
# is conventional (LIKE is also internally ``like(pattern, string)``).
#
# Character classes:
#   *   — matches any sequence of zero or more characters
#   ?   — matches exactly one character
#
# Unlike LIKE, GLOB is case-sensitive.  Python's ``fnmatch.fnmatchcase``
# implements exactly these semantics.


@register("glob")
def _glob_fn(pattern: SqlValue, string: SqlValue) -> SqlValue:
    """Case-sensitive glob pattern match.

    Returns 1 (truthy) if *string* matches *pattern*, 0 otherwise.
    NULL arguments propagate to NULL.

    Truth table::

        glob('J*',   'John')   → 1   (starts with J)
        glob('J*',   'john')   → 0   (case-sensitive)
        glob('Jo?n', 'John')   → 1   (? matches h)
        glob('Jo?n', 'Jon')    → 0   (? needs exactly one char)
        glob('*',    '')        → 1   (* matches empty string)
        glob(NULL,   'x')       → NULL
        glob('*',    NULL)      → NULL
    """
    import fnmatch  # stdlib — import here to avoid top-level cost when unused

    if pattern is None or string is None:
        return None
    if not isinstance(pattern, str) or not isinstance(string, str):
        # Non-string operands: convert to string first (SQLite behaviour)
        pattern = str(pattern) if not isinstance(pattern, str) else pattern
        string = str(string) if not isinstance(string, str) else string
    # Return a Python bool so that NOT GLOB compiles correctly via UnaryOp.NOT,
    # which expects a boolean operand.  The cursor layer's _coerce_value()
    # converts True→1 and False→0 on the way out, so SELECT results still
    # show 1/0 rather than True/False — matching SQLite's output format.
    return fnmatch.fnmatchcase(string, pattern)


# ---------------------------------------------------------------------------
# Numeric functions
# ---------------------------------------------------------------------------


@register("abs")
def _abs(x: SqlValue) -> SqlValue:
    """Return the absolute value of *x*.

    Returns NULL for NULL input.
    For integers and floats, returns the standard absolute value.
    For text or blobs, SQLite attempts to parse a leading numeric portion;
    if the string doesn't start with a number, the result is 0.0.  This
    matches real SQLite: ``ABS('text')`` → ``0.0``, ``ABS('-3.5abc')`` →
    ``3.5``.

    Examples::

        ABS(-5)      → 5
        ABS(-3.14)   → 3.14
        ABS(NULL)    → NULL
        ABS('text')  → 0.0      ← differs from Python; matches SQLite
        ABS('-3abc') → 3.0
    """
    if x is None:
        return None
    if isinstance(x, bool):
        return abs(int(x))
    if isinstance(x, (int, float)):
        return abs(x)  # type: ignore[arg-type]
    # Text / blob: try to extract a leading numeric value exactly as SQLite does.
    # SQLite reads as many characters as form a valid number; the rest is ignored.
    s = x.decode("utf-8", errors="replace") if isinstance(x, (bytes, bytearray)) else str(x)
    s = s.strip()
    # Collect leading numeric portion: optional sign, digits, optional dot + digits.
    m = re.match(r"^[+-]?(\d+\.?\d*|\.\d+)", s)
    if m:
        num = float(m.group(0))
        result = abs(num)
        # Return int if the result is a whole number (SQLite style).
        return int(result) if result == int(result) and isinstance(x, str) else result
    return 0.0


@register("round")
def _round(*args: SqlValue) -> SqlValue:
    """Round *x* to *digits* decimal places (default 0).

    ``ROUND(x)`` → integer number of decimal places (0).
    ``ROUND(x, n)`` → rounded to *n* places.  Negative *n* rounds to the
    left of the decimal point.

    **Tie-breaking — half away from zero, not banker's rounding.**

    Python's built-in ``round()`` uses *banker's rounding* (round half
    to even): ``round(0.5) == 0`` and ``round(2.5) == 2``.  SQLite uses
    *round half away from zero*: ``round(0.5) == 1.0`` and
    ``round(2.5) == 3.0`` — the same convention taught in school.

    For the two-argument form, the rounding is applied to the *exact
    IEEE 754 representation* of *x*, not its shortest decimal repr.
    So ``round(2.355, 2) == 2.35`` because the underlying float is
    ≈ 2.3549999…, but ``round(2.345, 2) == 2.35`` because that float is
    ≈ 2.3450000…  This matches sqlite3's printf-based implementation
    (which converts the double to decimal, then rounds half-up).

    **NULL handling.**  Either argument being NULL returns NULL; this
    matches SQLite, which short-circuits if either ``x`` or ``digits``
    is NULL (mini-sqlite previously coerced a NULL digits argument to
    the default ``0`` — wrong).

    Examples::

        ROUND(3.14159, 2)  → 3.14
        ROUND(0.5)         → 1.0   (not 0.0)
        ROUND(2.5)         → 3.0   (not 2.0)
        ROUND(-2.5)        → -3.0  (not -2.0)
        ROUND(0.25, 1)     → 0.3
        ROUND(1.5, NULL)   → NULL
    """
    _arity("round", list(args), 1, 2)
    x = args[0]
    if x is None:
        return None
    # NULL digits → NULL result (SQLite short-circuits, unlike Python's
    # ``round(x, None)`` which falls back to integer rounding).
    if len(args) == 2 and args[1] is None:
        return None
    if not isinstance(x, (int, float)):
        return x
    n = int(args[1]) if len(args) == 2 else 0
    # SQLite clamps the digits argument to [0, 30]: negative values are
    # treated as 0 (no rounding to the left of the decimal point) and
    # values above 30 are capped (the IEEE 754 double has at most ~17
    # decimal digits of precision anyway).
    if n < 0:
        n = 0
    elif n > 30:
        n = 30
    xf = float(x)
    if n == 0:
        # One-arg form: int64 cast of ``x ± 0.5`` — half away from zero.
        # SQLite uses ``(double)((int64)(r + (r<0 ? -0.5 : +0.5)))`` so
        # the result is always a whole-number double.
        return float(int(xf + 0.5)) if xf >= 0 else float(int(xf - 0.5))
    # Two-arg form: SQLite uses its own printf("%.*f", n, x) on the
    # exact IEEE 754 value and re-parses the resulting string.  We
    # emulate via ``decimal.Decimal(float)`` (which gives the exact
    # binary representation) followed by ``quantize`` with
    # ROUND_HALF_UP — equivalent because both engines work in decimal
    # space on the actual stored double.
    #
    # The default Decimal precision (28 digits) is not enough to hold
    # the exact representation of a double when ``n`` approaches 30 —
    # ``Decimal(0.1)`` already needs ~17 digits, plus ``n`` more for the
    # quantize target — so we bump the local precision to 80 (well above
    # the maximum meaningful digit count for a float64).
    from decimal import ROUND_HALF_UP, Decimal, localcontext
    with localcontext() as ctx:
        ctx.prec = 80
        q = Decimal(10) ** -n
        return float(Decimal(xf).quantize(q, rounding=ROUND_HALF_UP))


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
    """Return *x* modulo *y* — C-style ``fmod`` semantics, float result.

    SQLite's ``MOD(x, y)`` is implemented on top of ``fmod`` from the
    C math library: the result has the sign of the *dividend* (``x``)
    and is always returned as a floating-point value (even for integer
    inputs).  Python's built-in ``%`` operator follows the *divisor*
    sign and preserves the integer type — neither matches.

    Returns NULL for NULL inputs and for division-by-zero (matching
    SQLite's "arithmetic errors return NULL" policy).

    Examples::

        MOD(10, 3)    → 1.0    (Python ``10 % 3``   == 1, same magnitude)
        MOD(-7, 3)    → -1.0   (Python ``-7 % 3``   == 2; wrong sign)
        MOD(7, -3)    → 1.0    (Python ``7 % -3``   == -2; wrong sign)
        MOD(7.5, 2.0) → 1.5
        MOD(10, 0)    → NULL
    """
    if isinstance(x, (int, float)) and isinstance(y, (int, float)):
        if y == 0:
            return None
        # math.fmod implements C-style modulo: result sign matches dividend.
        import math
        return math.fmod(float(x), float(y))
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


@register("ln")
def _ln(x: SqlValue) -> SqlValue:
    """Natural logarithm (base e).  SQLite 3.35+ math function.

    Returns NULL for non-positive *x* or NULL input.
    """
    return _safe_math(math.log, x)


@register("log")
def _log(*args: SqlValue) -> SqlValue:
    """Base-10 logarithm (1 arg) or log base B (2 args: ``LOG(B, x)``).

    ``LOG(x)``    → base-10 log of *x*.  Matches SQLite's ``log()`` function,
                    which is base 10 — NOT natural log.  Use ``LN(x)`` for
                    natural log.
    ``LOG(B, x)`` → log of *x* in base *B*.

    Returns NULL for non-positive inputs.
    """
    _arity("log", list(args), 1, 2)
    if len(args) == 1:
        return _safe_math(math.log10, args[0])
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
# Hyperbolic trigonometric functions (SQLite "extended math")
# ---------------------------------------------------------------------------
#
# Hyperbolic-trig functions are part of the standard SQLite math library
# (``--enable-math-functions`` build option, which is the default for the
# Python ``sqlite3`` module on every modern platform).  Mathematical
# definitions:
#
#   sinh(x) = (e^x − e^−x) / 2          — hyperbolic sine
#   cosh(x) = (e^x + e^−x) / 2          — hyperbolic cosine
#   tanh(x) = sinh(x) / cosh(x)         — hyperbolic tangent
#   asinh(x) = ln(x + √(x² + 1))        — inverse sinh, domain: all reals
#   acosh(x) = ln(x + √(x² − 1))        — inverse cosh, domain: x ≥ 1
#   atanh(x) = ½ · ln((1 + x)/(1 − x))  — inverse tanh, domain: |x| < 1
#
# Out-of-domain inputs return NULL (e.g. ``acosh(0.5)`` and ``atanh(1)``),
# matching SQLite's behaviour where math domain errors silently produce NULL.


@register("sinh")
def _sinh(x: SqlValue) -> SqlValue:
    """Return hyperbolic sine of *x*."""
    return _safe_math(math.sinh, x)


@register("cosh")
def _cosh(x: SqlValue) -> SqlValue:
    """Return hyperbolic cosine of *x*."""
    return _safe_math(math.cosh, x)


@register("tanh")
def _tanh(x: SqlValue) -> SqlValue:
    """Return hyperbolic tangent of *x*."""
    return _safe_math(math.tanh, x)


@register("asinh")
def _asinh(x: SqlValue) -> SqlValue:
    """Return inverse hyperbolic sine of *x* (domain: all reals)."""
    return _safe_math(math.asinh, x)


@register("acosh")
def _acosh(x: SqlValue) -> SqlValue:
    """Return inverse hyperbolic cosine of *x* (domain: x ≥ 1)."""
    return _safe_math(math.acosh, x)


@register("atanh")
def _atanh(x: SqlValue) -> SqlValue:
    """Return inverse hyperbolic tangent of *x* (domain: |x| < 1)."""
    return _safe_math(math.atanh, x)


@register("trunc")
def _trunc(x: SqlValue) -> SqlValue:
    """Truncate *x* toward zero — drop the fractional part.

    Differs from ``floor`` and ``ceiling`` in sign handling:

        trunc( 3.7) =  3.0     floor( 3.7) =  3.0     ceiling( 3.7) =  4.0
        trunc(−3.7) = −3.0     floor(−3.7) = −4.0     ceiling(−3.7) = −3.0

    SQLite returns the truncated value as REAL (not INTEGER) to match the
    input type, which is what ``math.trunc`` already produces via float().
    """
    if x is None or not isinstance(x, (int, float)):
        return None
    try:
        # math.trunc returns int; SQLite returns REAL.  Cast back to float
        # so ``trunc(3.7) == 3.0``, not ``3``.
        return float(math.trunc(float(x)))  # type: ignore[arg-type]
    except (ValueError, OverflowError):
        return None


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


@register("octet_length")
def _octet_length(x: SqlValue) -> SqlValue:
    """Return the number of **bytes** in a string (UTF-8) or BLOB.

    Differs from ``length()`` for non-ASCII text:

        LENGTH('café')        → 4       (4 characters)
        OCTET_LENGTH('café')  → 5       ('é' is 2 bytes in UTF-8)

    For ASCII strings, ``length()`` and ``octet_length()`` agree.

    Numeric inputs are coerced via decimal string representation, so
    ``OCTET_LENGTH(123)`` is ``3`` (the byte-length of ``"123"``).

    NULL input propagates as NULL.
    """
    if x is None:
        return None
    if isinstance(x, str):
        return len(x.encode("utf-8"))
    if isinstance(x, (bytes, bytearray)):
        return len(x)
    # Numeric — convert to string first (matches LENGTH semantics).
    return len(str(x).encode("utf-8"))


@register("concat")
def _concat(*args: SqlValue) -> SqlValue:
    """Concatenate one or more arguments into a single TEXT string.

    SQLite 3.44+ built-in.  NULL arguments are treated as empty strings
    (do NOT propagate to a NULL result — that's ``concat_ws``'s job for
    the separator).  At least one argument is required; calling with
    zero arguments matches SQLite's error.

    Non-string arguments are coerced via ``str()`` (their SQL text
    representation), so ``CONCAT(1, '+', 2) → '1+2'``.

    Examples::

        CONCAT('a', 'b', 'c')   → 'abc'
        CONCAT('a', NULL, 'c')  → 'ac'
        CONCAT('id=', 42)       → 'id=42'
    """
    # Variadic with minimum 1.  ``_arity`` only supports a fixed set of
    # accepted counts; for "≥ 1" we do the check inline.
    if len(args) < 1:
        raise WrongNumberOfArguments(name="concat", expected="≥ 1", got=len(args))
    parts: list[str] = []
    for a in args:
        if a is None:
            continue
        # Preserve already-string inputs; coerce others via str().
        parts.append(a if isinstance(a, str) else str(a))
    return "".join(parts)


@register("concat_ws")
def _concat_ws(*args: SqlValue) -> SqlValue:
    """Concatenate arguments with a separator (``concat_ws`` = "with separator").

    SQLite 3.44+ built-in.  The first argument is the separator; the
    remaining arguments are values to concatenate.

    Distinct NULL semantics:

    - **Separator is NULL** → result is NULL.
    - **A value is NULL**   → skip it (does NOT terminate the result).

    Examples::

        CONCAT_WS('-', 'a', 'b', 'c')   → 'a-b-c'
        CONCAT_WS('-', 'a', NULL, 'c')  → 'a-c'      (NULL skipped)
        CONCAT_WS(NULL, 'a', 'b')       → NULL        (NULL sep)
        CONCAT_WS(',', 1, 2, 3)         → '1,2,3'

    A minimum of two arguments (separator + one value) is required to
    match SQLite's documented signature, though SQLite is permissive and
    accepts just the separator (returning the empty string); we mirror
    that.
    """
    if len(args) < 1:
        raise WrongNumberOfArguments(name="concat_ws", expected="≥ 1", got=len(args))
    sep = args[0]
    if sep is None:
        return None
    sep_str = sep if isinstance(sep, str) else str(sep)
    parts: list[str] = []
    for a in args[1:]:
        if a is None:
            continue
        parts.append(a if isinstance(a, str) else str(a))
    return sep_str.join(parts)


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
    """Extract a substring — full SQLite semantics, edge cases included.

    ``SUBSTR(x, y)``       → from 1-indexed position *y* to end of string.
    ``SUBSTR(x, y, z)``    → *z* characters starting at position *y*.

    The arithmetic is **1-indexed**, with three subtleties that catch
    most implementations off-guard:

    1. **Negative ``y``** counts back from the end: ``y = -1`` is the
       last character, ``y = -2`` is the second-to-last, …, ``y = -N``
       is the first character of an ``N``-length string.  Concretely,
       a negative ``y`` is resolved to ``N + 1 + y``.

    2. **``y = 0``** is *one position to the left* of the first
       character — neither a valid index nor a sentinel for "beginning".
       Combined with ``z = 3`` on ``"hello"`` (length 5), the requested
       span is positions ``0, 1, 2`` but only positions ``1, 2`` are
       inside the string, so the result is ``"he"`` (not ``"hel"``).

    3. **Negative ``z``** means "the ``|z|`` characters *preceding*
       position ``y``".  So ``SUBSTR("hello", 3, -2)`` asks for two
       characters ending at position 2: positions ``1, 2`` → ``"he"``.

    The implementation models the requested character range as a
    closed 1-indexed interval ``[lo, hi]``, clips it to the string's
    valid range ``[1, N]``, and converts back to a Python slice.  This
    handles every overflow/underflow case uniformly — including
    ``y = -100`` on a 5-char string (where ``y`` resolves to ``-94``
    and the entire requested span lies to the left of the string).

    Blob inputs use the same algorithm, operating on bytes.

    Returns NULL if *x* is NULL.

    Examples::

        SUBSTR("hello",  2)       → "ello"
        SUBSTR("hello",  2, 3)    → "ell"
        SUBSTR("hello", -3)       → "llo"     ( y = 6 + (-3) = 3 )
        SUBSTR("hello",  0, 3)    → "he"      ( positions 0,1,2 ∩ 1..5 )
        SUBSTR("hello",  3, -2)   → "he"      ( two chars before pos 3 )
        SUBSTR("hello", -100, 5)  → ""        ( span entirely left of string )
        SUBSTR("hello", -100, 102)→ "hello"   ( span covers whole string )
    """
    _arity("substr", list(args), 2, 3)
    x = args[0]
    if x is None:
        return None
    is_blob = isinstance(x, (bytes, bytearray))
    if not is_blob and not isinstance(x, str):
        return x
    s: bytes | str = bytes(x) if is_blob else x
    empty: bytes | str = b"" if is_blob else ""
    n = len(s)
    y_raw = args[1]
    y = int(y_raw) if y_raw is not None else 1  # type: ignore[arg-type]
    # Resolve 1-indexed start position.  Negative y counts from the end;
    # y == 0 stays as 0 (one position before the first character — a
    # legitimate SQLite-ism that callers occasionally rely on).
    if y < 0:
        y = n + 1 + y
    # Determine the requested closed interval [lo, hi] in 1-indexed
    # positions.  Both bounds may be outside [1, n] before clipping.
    if len(args) == 3 and args[2] is not None:
        z = int(args[2])  # type: ignore[arg-type]
        if z >= 0:
            lo, hi = y, y + z - 1
        else:
            # Negative length: |z| characters *preceding* position y.
            lo, hi = y + z, y - 1
    else:
        lo, hi = y, n
    # Clip to the valid range; if the clipped interval is empty, return
    # the empty string/blob without going through the slice (which would
    # do the right thing anyway, but we want a clean unified exit).
    lo = max(lo, 1)
    hi = min(hi, n)
    if lo > hi:
        return empty
    return s[lo - 1: hi]


@register("replace")
@null_propagating
def _replace(x: SqlValue, old: SqlValue, new: SqlValue) -> SqlValue:
    """Replace all occurrences of *old* in *x* with *new*.

    Returns NULL if any argument is NULL (handled by ``null_propagating``).

    **Empty needle ⇒ no-op.**  Python's ``str.replace("", X)`` inserts ``X``
    between every character (and at both ends), so ``"hello".replace("", "X")``
    becomes ``"XhXeXlXlXoX"``.  SQLite explicitly treats an empty search
    string as "match nothing" and returns the input unchanged.  We honour
    SQLite's behaviour because callers building a SQL pipeline would not
    expect a no-op edit to suddenly multiply their string length.

    Examples::

        REPLACE("hello world", "world", "SQL")  → "hello SQL"
        REPLACE("aaa", "a", "bb")               → "bbbbbb"
        REPLACE("hello", "", "X")               → "hello"   (no-op)
    """
    if isinstance(x, str) and isinstance(old, str) and isinstance(new, str):
        if old == "":
            # Match SQLite: empty needle is a no-op rather than Python's
            # "insert between every character" behaviour.
            return x
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
    """Convert *x* to an upper-cased hexadecimal string, matching SQLite semantics.

    SQLite's HEX() function works on the *string representation* of its
    argument, not the binary representation.  For numeric values this means
    SQLite first formats the number using its default conversion rules
    (e.g. ``123`` → ``"123"``) and then hex-encodes the resulting UTF-8 bytes
    (``"123"`` → ``"313233"``).

    For BLOB values, encodes each byte as two hex digits.
    For TEXT values, encodes the UTF-8 bytes.
    Returns the empty string for NULL input (matching SQLite's NULL handling).

    Examples::

        HEX(X'DEADBEEF')   → "DEADBEEF"
        HEX("AB")          → "4142"
        HEX(123)           → "313233"      (the string "123" hex-encoded)
        HEX(3.14)          → "332E3134"    (the string "3.14" hex-encoded)
        HEX(NULL)          → ""
    """
    if x is None:
        return ""
    if isinstance(x, (bytes, bytearray)):
        return bytes(x).hex().upper()
    if isinstance(x, str):
        return x.encode("utf-8").hex().upper()
    if isinstance(x, bool):
        # SQLite stores booleans as 0/1 integers; format as a decimal string.
        return str(int(x)).encode("utf-8").hex().upper()
    if isinstance(x, int):
        return str(x).encode("utf-8").hex().upper()
    if isinstance(x, float):
        # Float formatting: SQLite uses the shortest round-trippable repr.
        # Python's str(x) is a good approximation (e.g. 3.14 → "3.14", not "3.1400000000000001").
        return str(x).encode("utf-8").hex().upper()
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
    r"%(?P<flags>[-+ #0]*)(?P<width>\d*)(?:\.(?P<prec>\d+))?(?P<conv>[diouxXeEfgGsqQw%])"
)


def _printf_format(template: str, args: list[SqlValue]) -> str:  # noqa: C901
    """Implement SQLite's subset of C-style ``printf``.

    Supported conversions:

    - ``%d``, ``%i``, ``%o``, ``%u``, ``%x``, ``%X`` — integer formatting
    - ``%f``, ``%e``, ``%E``, ``%g``, ``%G`` — float formatting
    - ``%s`` — string (None → "")
    - ``%q`` — SQL string-literal escape: single quotes doubled, **no
      surrounding quotes**.  NULL → ``"(NULL)"``.  Designed for
      interpolation *inside* a single-quoted SQL string literal — the
      caller supplies the wrapping quotes.
    - ``%Q`` — single-quoted SQL literal: like ``%q`` but **with**
      surrounding single quotes, and NULL → the literal ``"NULL"`` (no
      quotes).  Use ``%Q`` to build a complete inline literal.
    - ``%w`` — SQL identifier escape: double quotes doubled, **no
      surrounding quotes**.  NULL → ``"(NULL)"``.  Designed for
      interpolation inside a ``"…"`` quoted identifier (table/column
      name).
    - ``%%`` — literal ``%``

    The ``%q``/``%Q``/``%w`` rules match SQLite's reference
    implementation (``sqlite3_mprintf`` in ``src/printf.c``): see the
    SQLite docs at https://sqlite.org/printf.html.
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
            # Python's ``%#o`` formats with the modern ``0o`` prefix; SQLite
            # (following C printf) uses the classic single ``0`` prefix and,
            # critically, **omits the prefix entirely when the value is 0**
            # (because the digit itself is already a zero, so adding ``0``
            # would just produce ``00``).  Also: when both ``#`` and a
            # width/zero-pad are present, SQLite places the ``0`` prefix
            # *after* the leading spaces (e.g. ``%#5o`` of 8 → ``"  010"``),
            # so we strip ``#`` from the Python format, let Python compute
            # the padding, then prepend ``0`` into the right column.
            if conv == "o" and "#" in flags:
                py_flags = flags.replace("#", "")
                spec = f"%{py_flags}{width_s}"
                if prec_s:
                    spec += f".{prec_s}"
                spec += conv
                try:
                    s = spec % val
                except TypeError:
                    s = str(val)
                if val != 0:
                    # Replace one space with '0' if we right-aligned with
                    # spaces; otherwise simply prepend.  Zero-padded width
                    # produces a string already starting with '0', so the
                    # natural "prepend" branch is correct there too.
                    stripped = s.lstrip(" ")
                    spaces = len(s) - len(stripped)
                    s = (
                        " " * (spaces - 1) + "0" + stripped
                        if spaces > 0
                        else "0" + s
                    )
                result.append(s)
                continue
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
            # SQL string-literal escape — single quotes doubled, *no*
            # surrounding quotes.  NULL becomes the literal "(NULL)" so
            # downstream code that wraps the result in '...' doesn't
            # silently turn a NULL into an empty string.
            if arg is None:
                result.append("(NULL)")
            else:
                result.append(str(arg).replace("'", "''"))
        elif conv == "Q":
            # SQL string literal — like %q but wrapped in single quotes,
            # except NULL emits the literal "NULL" (no quotes) so the
            # output is a syntactically valid SQL expression.
            if arg is None:
                result.append("NULL")
            else:
                s = str(arg).replace("'", "''")
                result.append(f"'{s}'")
        elif conv == "w":
            # SQL identifier escape — double quotes doubled, *no*
            # surrounding quotes.  Mirrors %q for the identifier
            # namespace: the caller supplies the wrapping "...".
            if arg is None:
                result.append("(NULL)")
            else:
                result.append(str(arg).replace('"', '""'))
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
        PRINTF("%q", "it's")                 → "it''s"  (no outer quotes)
        PRINTF("%Q", "it's")                 → "'it''s'"
        PRINTF("%w", 'col"name')             → 'col""name'
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


# ---------------------------------------------------------------------------
# Connection-state pseudo-functions
# ---------------------------------------------------------------------------
#
# SQLite exposes a handful of "scalar" functions whose result depends on
# connection state rather than their arguments:
#
#   - changes()           — rows affected by the most recent INSERT/UPDATE/DELETE
#   - total_changes()     — cumulative rows affected since the connection opened
#   - last_insert_rowid() — rowid of the most recent successful INSERT
#
# Real SQLite stores these on the C ``sqlite3*`` handle.  Mini-sqlite is
# Python-side, single-threaded, and currently does not have a clean
# Connection→VM channel for per-call state.  We fake it with three
# module-level integers that the engine updates after every statement.
# Single-threaded use is the only supported mode of mini-sqlite, so a single
# global is correct as long as the engine maintains the invariant
# "update the globals before calling the next user query".
#
# This approach is intentionally simple — it makes the common case
# (`SELECT changes()` immediately after an UPDATE) byte-compatible with
# sqlite3 without invasive plumbing changes.  Multi-connection programs
# will see cross-talk, which is documented but acceptable for the educational
# scope of mini-sqlite.

_LAST_INSERT_ROWID: int = 0
_CHANGES: int = 0
_TOTAL_CHANGES: int = 0


def set_connection_state(
    *,
    last_insert_rowid: int | None = None,
    changes: int | None = None,
    total_changes: int | None = None,
) -> None:
    """Update the connection-state globals consulted by the scalar functions.

    Called by the mini-sqlite engine after every statement.  Each kwarg
    that is *not* None overwrites the corresponding global; leaving a
    kwarg as None preserves the previous value.
    """
    global _LAST_INSERT_ROWID, _CHANGES, _TOTAL_CHANGES
    if last_insert_rowid is not None:
        _LAST_INSERT_ROWID = last_insert_rowid
    if changes is not None:
        _CHANGES = changes
    if total_changes is not None:
        _TOTAL_CHANGES = total_changes


@register("last_insert_rowid")
def _last_insert_rowid() -> SqlValue:
    """Return the rowid of the most recent successful INSERT on this connection.

    Returns 0 if no INSERT has been performed since the connection was opened.
    Matches SQLite's ``last_insert_rowid()`` C-API and SQL function.
    """
    return _LAST_INSERT_ROWID


@register("changes")
def _changes() -> SqlValue:
    """Return the number of rows affected by the most recent INSERT, UPDATE,
    or DELETE on this connection.

    Excludes rows changed by triggers (matching SQLite).  Returns 0 if no DML
    has been performed.
    """
    return _CHANGES


@register("total_changes")
def _total_changes() -> SqlValue:
    """Return the total number of rows affected by INSERT, UPDATE, or DELETE
    statements on this connection since it was opened.

    Includes rows changed by triggers (matching SQLite).
    """
    return _TOTAL_CHANGES


# ---------------------------------------------------------------------------
# Version / build identification
# ---------------------------------------------------------------------------
#
# SQLite reports its version through two functions:
#
#   sqlite_version()     → e.g. '3.50.4'
#   sqlite_source_id()   → e.g. '2025-07-30 19:33:53 4d8adfb30e03f9cf...'
#
# Real applications use these to gate behaviour based on the available SQLite
# version (e.g. "this query needs JSON1, available since 3.9").  Mini-sqlite
# reports a fixed version that represents the SQLite feature set we
# approximately track.  This is updated when we add a new compatibility tier.

_MINI_SQLITE_REPORTED_SQLITE_VERSION = "3.45.0"
_MINI_SQLITE_REPORTED_SQLITE_SOURCE_ID = (
    "mini-sqlite emulation of SQLite 3.45.0 — not a real SQLite build"
)


@register("sqlite_version")
def _sqlite_version() -> SqlValue:
    """Return a SQLite version string.

    Mini-sqlite is not real SQLite; the version we report is the SQLite
    release whose feature set we most closely approximate.  Applications
    that gate behaviour on `sqlite_version()` get a string they can parse
    with ordinary tuple comparison: ``tuple(map(int, v.split('.')))``.
    """
    return _MINI_SQLITE_REPORTED_SQLITE_VERSION


@register("sqlite_source_id")
def _sqlite_source_id() -> SqlValue:
    """Return a SQLite source-ID string.

    Real SQLite returns a build identifier with date, time, and SHA-3 hash.
    Mini-sqlite is not a SQLite build; we return a marker string so that any
    application code checking this value can see it's running against the
    emulation rather than a real SQLite binary.
    """
    return _MINI_SQLITE_REPORTED_SQLITE_SOURCE_ID


# ---------------------------------------------------------------------------
# Optimizer hints — likely(X), unlikely(X), likelihood(X, Y)
# ---------------------------------------------------------------------------
#
# SQLite exposes three "optimizer hint" functions that always return their
# first argument unchanged — they exist purely to inform the SQLite query
# planner's branch-probability estimates.  Mini-sqlite has no cost-based
# optimizer that uses these hints, but applications written for SQLite
# routinely sprinkle them in `WHERE` clauses (especially `unlikely`).
# Implementing them as identity functions makes such queries portable.
#
# Real-SQLite semantics::
#
#     likely(X)       == X                  (hint: branch is almost always taken)
#     unlikely(X)     == X                  (hint: branch is almost never taken)
#     likelihood(X,Y) == X                  (Y is a 0.0–1.0 probability literal,
#                                            ignored at runtime)


@register("likely")
def _likely(x: SqlValue) -> SqlValue:
    """Return *x* unchanged.

    SQLite's planner uses this as a hint that the expression is usually true
    in a WHERE clause; mini-sqlite has no cost-based optimizer so we treat
    it as the identity function.
    """
    return x


@register("unlikely")
def _unlikely(x: SqlValue) -> SqlValue:
    """Return *x* unchanged.

    SQLite's planner uses this as a hint that the expression is rarely true;
    mini-sqlite ignores the hint and returns *x* verbatim.
    """
    return x


@register("likelihood")
def _likelihood(x: SqlValue, y: SqlValue) -> SqlValue:  # noqa: ARG001 — y is a hint
    """Return *x* unchanged; *y* is a probability hint (ignored).

    ``likelihood(X, Y)`` tells SQLite's planner that ``X`` is true with
    probability ``Y`` (a floating-point literal between 0.0 and 1.0).
    Mini-sqlite has no statistics-based query planner, so we ignore *y*
    and pass *x* through.
    """
    return x


# ---------------------------------------------------------------------------
# sqlite_compileoption_used / sqlite_compileoption_get
# ---------------------------------------------------------------------------
#
# Real SQLite is compiled with feature flags like ``SQLITE_ENABLE_RTREE`` or
# ``SQLITE_THREADSAFE=1``.  These two functions let applications query which
# flags were set at compile time:
#
#   sqlite_compileoption_used(name)  → 1 if *name* was defined, else 0
#   sqlite_compileoption_get(N)      → the Nth defined option's name, or NULL
#
# Mini-sqlite is not a compiled SQLite binary — there are no compile-time
# options.  We return ``0`` from ``compileoption_used`` (no options are set)
# and ``NULL`` from ``compileoption_get`` (no Nth option exists), which is
# safe behaviour for application code that does feature-detection probes.


@register("sqlite_compileoption_used")
def _sqlite_compileoption_used(name: SqlValue) -> SqlValue:  # noqa: ARG001
    """Return ``0`` — mini-sqlite has no SQLite compile-time options."""
    return 0


@register("sqlite_compileoption_get")
def _sqlite_compileoption_get(n: SqlValue) -> SqlValue:  # noqa: ARG001
    """Return ``NULL`` — mini-sqlite has no Nth SQLite compile-time option."""
    return None


# ---------------------------------------------------------------------------
# Scalar MAX(a, b) and MIN(a, b) — two-argument forms
# ---------------------------------------------------------------------------
#
# SQLite overloads MAX and MIN: with one argument they are aggregates (handled
# by InitAgg/FinalizeAgg opcodes); with two or more arguments they are scalar
# functions that return the greatest/least value.  NULL is treated as less than
# any non-null value — so MAX(x, NULL) → x, MIN(x, NULL) → x.


def _sql_compare(a: SqlValue, b: SqlValue) -> int:
    """Return -1, 0, or 1 for a < b, a == b, a > b under SQLite ordering.

    NULL is less than every non-null value.  Comparison between different
    non-null types follows SQLite's type ordering: integer/real < text < blob.
    """
    if a is None and b is None:
        return 0
    if a is None:
        return -1
    if b is None:
        return 1
    # Same broad type: compare directly.
    a_num = isinstance(a, (int, float)) and not isinstance(a, bool)
    b_num = isinstance(b, (int, float)) and not isinstance(b, bool)
    if a_num and b_num:
        return 0 if a == b else (-1 if a < b else 1)  # type: ignore[operator]
    a_text = isinstance(a, str)
    b_text = isinstance(b, str)
    if a_text and b_text:
        return 0 if a == b else (-1 if a < b else 1)  # type: ignore[operator]
    a_blob = isinstance(a, (bytes, bytearray))
    b_blob = isinstance(b, (bytes, bytearray))
    if a_blob and b_blob:
        return 0 if a == b else (-1 if a < b else 1)  # type: ignore[operator]
    # Cross-type: numeric < text < blob (SQLite affinity ordering).
    _rank = {int: 0, float: 0, str: 1, bytes: 2, bytearray: 2}
    ar = _rank.get(type(a), 1)
    br = _rank.get(type(b), 1)
    return 0 if ar == br else (-1 if ar < br else 1)


@register("max")
def _max_scalar(*args: SqlValue) -> SqlValue:
    """Scalar MAX — return the greatest of two or more arguments.

    When called with exactly one argument inside a GROUP BY context the
    planner routes to the aggregate opcode instead; this registry entry
    handles the two-or-more-argument scalar form.

    NULL propagation: if ANY argument is NULL the result is NULL.  This
    differs from the aggregate MAX(), which *ignores* NULLs.  SQLite
    documents this explicitly: "The multi-argument max() works like the
    SQLite extension" and "if any argument is NULL, the result is NULL."

    Examples::

        MAX(3, 5)           → 5
        MAX('apple', 'fig') → 'fig'
        MAX(1, NULL)        → NULL
        MAX(NULL, NULL)     → NULL
    """
    if not args:
        return None
    # Propagate NULL: any NULL argument → NULL result (scalar semantics).
    if any(a is None for a in args):
        return None
    result = args[0]
    for a in args[1:]:
        if _sql_compare(a, result) > 0:
            result = a
    return result


@register("min")
def _min_scalar(*args: SqlValue) -> SqlValue:
    """Scalar MIN — return the least of two or more arguments.

    NULL propagation: if ANY argument is NULL the result is NULL.  This
    matches SQLite's scalar MIN() semantics (distinct from aggregate MIN()
    which ignores NULLs).

    Examples::

        MIN(3, 5)       → 3
        MIN(1, NULL)    → NULL
        MIN(NULL, NULL) → NULL
    """
    if not args:
        return None
    # Propagate NULL: any NULL argument → NULL result (scalar semantics).
    if any(a is None for a in args):
        return None
    result = args[0]
    for a in args[1:]:
        if _sql_compare(a, result) < 0:
            result = a
    return result


# ---------------------------------------------------------------------------
# Date/time functions
# ---------------------------------------------------------------------------
#
# SQLite supports six date/time scalar functions, all sharing a common
# "time value" input format and an optional list of modifier strings.
#
# Time value forms accepted:
#   'now'              → current UTC datetime
#   'YYYY-MM-DD'       → date only (time = 00:00:00 UTC)
#   'YYYY-MM-DD HH:MM' → date + hours/minutes
#   'YYYY-MM-DD HH:MM:SS[.SSS]' → full datetime
#   <float>            → Julian Day Number
#   <int>              → Unix epoch seconds
#
# Modifiers (applied in order, left to right):
#   '+N days' / '-N days'      (also hours, minutes, seconds)
#   '+N months' / '-N months'
#   '+N years' / '-N years'
#   'start of day'
#   'start of month'
#   'start of year'
#   'localtime'   → convert UTC to local time
#   'utc'         → convert local time to UTC


def _parse_timevalue(tv: SqlValue) -> datetime | None:
    """Convert a SQLite time value to an aware UTC datetime, or None on error."""
    if tv is None:
        return None

    if isinstance(tv, str):
        s = tv.strip()
        if s.lower() == "now":
            return datetime.now(tz=UTC).replace(microsecond=0)
        # Fractional-seconds form first: YYYY-MM-DD HH:MM:SS.sss[sss]
        # We try this *before* the fixed-format strptime loop because the
        # loop uses ``s[:len(fmt) + 2]`` to permit trailing garbage, which
        # would silently truncate the fraction and discard the microsecond
        # information.  Preserving microseconds is required for
        # ``strftime('%f', …)`` to round-trip the input.
        m = re.match(
            r"^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})\.(\d+)$", s
        )
        if m:
            try:
                dt = datetime.strptime(f"{m.group(1)} {m.group(2)}", "%Y-%m-%d %H:%M:%S")
                # Pad/truncate the fraction to 6 digits (microseconds).
                frac = (m.group(3) + "000000")[:6]
                return dt.replace(microsecond=int(frac), tzinfo=UTC)
            except ValueError:
                pass
        # ISO-8601 date only: YYYY-MM-DD (and variations without fractions)
        for fmt in (
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M",
            "%Y-%m-%d",
        ):
            try:
                dt = datetime.strptime(s[:len(fmt) + 2], fmt)
                return dt.replace(tzinfo=UTC)
            except ValueError:
                pass
        # SQLite also accepts time-only strings: 'HH:MM', 'HH:MM:SS', 'HH:MM:SS.sss'.
        # These represent the time of day on year 2000-01-01 in SQLite's internal
        # epoch, but for our purposes the date part is irrelevant — we anchor it
        # to 2000-01-01 so the time() function and strftime('%H:%M:%S', ...) work.
        # See https://www.sqlite.org/lang_datefunc.html#tmval — "time" rule.
        # The optional fractional-seconds portion is captured and converted
        # to microseconds (same reasoning as above).
        m = re.match(r"^(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?$", s)
        if m:
            hour = int(m.group(1))
            minute = int(m.group(2))
            second = int(m.group(3)) if m.group(3) else 0
            microsecond = (
                int((m.group(4) + "000000")[:6]) if m.group(4) else 0
            )
            try:
                return datetime(
                    2000, 1, 1, hour, minute, second, microsecond, tzinfo=UTC
                )
            except ValueError:
                pass
        return None

    if isinstance(tv, float):
        # Julian Day Number → datetime.
        # JD 2440587.5 = 1970-01-01 00:00:00 UTC
        unix_seconds = (tv - 2440587.5) * 86400.0
        return _datetime_from_unix_seconds(unix_seconds)

    if isinstance(tv, int) and not isinstance(tv, bool):
        # Unix epoch seconds.
        return _datetime_from_unix_seconds(tv)

    return None


def _datetime_from_unix_seconds(seconds: float) -> datetime | None:
    """Convert Unix-epoch seconds to a UTC ``datetime``, pre-1970 included.

    ``datetime.fromtimestamp()`` goes through the platform C library's time
    conversion, and the Windows CRT's ``_gmtime64``/``_localtime64`` reject
    negative ``time_t`` values (dates before 1970-01-01) with ``OSError`` --
    POSIX's ``gmtime`` handles them fine, so the same SQL query silently
    returned NULL on Windows only. Computing the date by pure calendar
    arithmetic from the fixed 1970-01-01 epoch avoids the platform C library
    entirely, so this works identically on every host `datetime`/`timedelta`
    run on, and still raises OverflowError for a genuinely unrepresentable
    result (outside year 1-9999) rather than for an artificial platform limit.
    """
    try:
        return (
            datetime(1970, 1, 1, tzinfo=UTC) + timedelta(seconds=seconds)
        ).replace(microsecond=0)
    except (ValueError, OverflowError):
        return None


def _apply_modifier(dt: datetime, modifier: str) -> datetime | None:
    """Apply one SQLite datetime modifier to *dt*.

    Returns None for unrecognised modifiers, matching SQLite's NULL propagation.
    """
    m_lower = modifier.strip().lower()

    if m_lower == "now":
        return datetime.now(tz=UTC).replace(microsecond=0)
    if m_lower == "start of day":
        return dt.replace(hour=0, minute=0, second=0, microsecond=0)
    if m_lower == "start of month":
        return dt.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
    if m_lower == "start of year":
        return dt.replace(month=1, day=1, hour=0, minute=0, second=0, microsecond=0)
    if m_lower == "localtime":
        return dt.astimezone().replace(tzinfo=UTC)
    if m_lower == "utc":
        return dt  # already UTC in our model

    # weekday N — advance to the next day-of-week N (0=Sun, 1=Mon, …, 6=Sat).
    # If today already matches, dt is unchanged (per SQLite spec).
    weekday_match = re.match(r"^weekday\s+(\d)$", m_lower)
    if weekday_match:
        target = int(weekday_match.group(1))
        if not 0 <= target <= 6:
            return None
        # Python: Monday=0..Sunday=6; SQLite: Sunday=0..Saturday=6
        # Convert SQLite target to Python: 0→6 (Sun), 1→0 (Mon), …, 6→5 (Sat)
        py_target = (target - 1) % 7
        days_ahead = (py_target - dt.weekday()) % 7
        return dt + timedelta(days=days_ahead)

    # unixepoch — interpret the time value as a Unix-epoch integer (SQLite 3.38+).
    # When applied as a modifier it forces the *current* dt's interpretation to
    # have come from a Unix epoch integer.  Since we already store as datetime
    # this is a no-op; we accept it so applications that pass it don't error.
    if m_lower == "unixepoch":
        return dt

    # auto (SQLite 3.46+) — auto-detect whether a numeric time argument was a
    # Unix epoch or a Julian day, based on its magnitude.  In mini-sqlite the
    # parser at :func:`_parse_timevalue` already discriminates by Python type
    # (``int`` → Unix epoch, ``float`` → Julian day), so this modifier is a
    # semantic no-op for us: by the time we reach ``_apply_modifier`` the
    # value is already a ``datetime``.  Accepting ``auto`` prevents NULL
    # propagation for application SQL written against SQLite 3.46+ that uses
    # it defensively (e.g. for forward-compatibility with future numeric
    # encodings).  Real SQLite also accepts ``auto`` on string inputs and
    # treats it as a pass-through, matching our behaviour here.
    if m_lower == "auto":
        return dt

    # Note: we do not accept ``julianday`` as a modifier.  In real SQLite
    # that modifier *requires* the time value to be a float and returns NULL
    # otherwise; we can't reliably reproduce that without restructuring
    # ``_resolve_datetime`` to keep the original value type alongside the
    # parsed datetime.  For ``float`` inputs mini-sqlite already interprets
    # them as Julian days via ``_parse_timevalue``, so omitting the modifier
    # is harmless in the common case.

    # Timezone offset: ±HH:MM, ±HH:MM:SS, ±HH:MM:SS.SSS
    #
    # SQLite treats the offset as a *shift* of the underlying datetime: a
    # positive value moves the wall-clock forward (UTC + offset), a negative
    # value moves it backward.  Application code typically uses this to
    # convert from UTC to a fixed timezone for display::
    #
    #     datetime('2024-03-15 14:30:00', '+02:00') → '2024-03-15 16:30:00'
    #     datetime('2024-03-15 14:30:00', '-05:30') → '2024-03-15 09:00:00'
    #
    # The seconds/fractional-seconds portions are optional and rarely used
    # but accepted for completeness with SQLite.
    tz_match = re.match(
        r"^([+-])(\d{2}):(\d{2})(?::(\d{2})(?:\.\d+)?)?$",
        m_lower,
    )
    if tz_match:
        sign = 1 if tz_match.group(1) == "+" else -1
        h = int(tz_match.group(2))
        m = int(tz_match.group(3))
        s = int(tz_match.group(4)) if tz_match.group(4) else 0
        # Reject out-of-range components (e.g. '+99:99') — match SQLite.
        if h > 23 or m > 59 or s > 59:
            return None
        total_seconds = sign * (h * 3600 + m * 60 + s)
        return dt + timedelta(seconds=total_seconds)

    # Numeric offset: [+-]N unit
    pat = re.match(
        r"^([+-]?\d+(?:\.\d+)?)\s+"
        r"(year|month|day|hour|minute|second)s?$",
        m_lower,
    )
    if pat:
        amount_s, unit = pat.group(1), pat.group(2)
        try:
            amount = float(amount_s)
        except ValueError:
            return None

        if unit in ("day", "days"):
            return dt + timedelta(days=amount)
        if unit in ("hour", "hours"):
            return dt + timedelta(hours=amount)
        if unit in ("minute", "minutes"):
            return dt + timedelta(minutes=amount)
        if unit in ("second", "seconds"):
            return dt + timedelta(seconds=amount)

        # Month and year adjustments — no timedelta for these; adjust calendar.
        n = int(amount)
        if unit in ("month", "months"):
            month = dt.month - 1 + n
            year = dt.year + month // 12
            month = month % 12 + 1
            # SQLite does NOT clamp when the day overflows the target month —
            # it lets the date roll into the next month.  E.g.:
            #   DATE('2024-01-31', '+1 month') → 2024-02-31 → 2024-03-02
            # We try the original day first; on ValueError we compute the
            # overflow and add it as extra days from the month's last day.
            try:
                return dt.replace(year=year, month=month, day=dt.day)
            except ValueError:
                last_day = calendar.monthrange(year, month)[1]
                overflow = dt.day - last_day
                return dt.replace(year=year, month=month, day=last_day) + timedelta(days=overflow)
        if unit in ("year", "years"):
            year = dt.year + n
            # SQLite does NOT clamp Feb 29 → Feb 28 — it lets the day roll
            # into March.  ``date('2024-02-29', '+1 year')`` becomes
            # ``'2025-03-01'`` (Feb 29 → Feb 28 + 1 overflow day), not
            # ``'2025-02-28'``.  Mirror the month-rollover algorithm above:
            # try the literal date first, then on ValueError add the
            # overflow as extra days from the last valid day of the target
            # month.
            try:
                return dt.replace(year=year, day=dt.day)
            except ValueError:
                last_day = calendar.monthrange(year, dt.month)[1]
                overflow = dt.day - last_day
                return dt.replace(year=year, day=last_day) + timedelta(days=overflow)

    return None  # unrecognised modifier → NULL propagation


def _resolve_datetime(args: list[SqlValue], skip_first: bool = False) -> datetime | None:
    """Parse time value from args[0] (or args[1] if skip_first), apply modifiers.

    The ``unixepoch`` modifier is special: it changes how the *time value
    itself* is interpreted (not a post-processing step on an already-parsed
    datetime).  Specifically, when ``unixepoch`` appears anywhere in the
    modifier list:

    - A numeric time value (int or float) is read as Unix-epoch seconds.
    - A string time value is parsed via SQLite's longest-numeric-prefix
      rule and read as Unix-epoch seconds.  Strings that lack a numeric
      prefix — including ISO-8601 date strings like ``'2024-01-01'`` —
      return NULL.

    Without the ``unixepoch`` modifier, mini-sqlite still accepts bare
    integer time values as Unix epoch (more lenient than real SQLite,
    which returns NULL); changing that is a behavioural break held back
    for a future PR.
    """
    offset = 1 if skip_first else 0
    if len(args) <= offset:
        return None
    raw_tv = args[offset]
    modifiers = list(args[offset + 1:])

    force_unixepoch = any(
        isinstance(m, str) and m.strip().lower() == "unixepoch"
        for m in modifiers
    )
    if force_unixepoch:
        if raw_tv is None:
            return None
        # SQLite requires the *entire* string (modulo surrounding
        # whitespace) to be a valid number — ``'2024-01-15'`` does not
        # count as ``2024`` followed by garbage, it counts as not-a-number
        # and produces NULL.  We enforce the whole-string match via
        # ``fullmatch`` rather than the longest-prefix rule used by CAST.
        if isinstance(raw_tv, str):
            num_m = re.fullmatch(r"\s*[+-]?\d+(?:\.\d+)?\s*", raw_tv)
            if not num_m:
                return None
            text = raw_tv.strip()
            try:
                raw_tv = float(text) if "." in text else int(text)
            except (ValueError, OverflowError):
                return None
        elif isinstance(raw_tv, bool) or not isinstance(raw_tv, (int, float)):
            return None
        dt = _datetime_from_unix_seconds(float(raw_tv))
        if dt is None:
            return None
        # Strip ``unixepoch`` from the remaining modifier chain so the
        # downstream handler doesn't see it twice (it's currently a no-op
        # there, but we want the modifier semantics centralised here).
        modifiers = [
            m for m in modifiers
            if not (isinstance(m, str) and m.strip().lower() == "unixepoch")
        ]
    else:
        dt = _parse_timevalue(raw_tv)
        if dt is None:
            return None

    for mod in modifiers:
        if mod is None:
            return None
        if not isinstance(mod, str):
            return None
        dt = _apply_modifier(dt, mod)
        if dt is None:
            return None
    return dt


@register("date")
def _date(*args: SqlValue) -> SqlValue:
    """Return an ISO-8601 date string for the given time value and modifiers.

    ``DATE(timevalue [, modifier...])`` → ``'YYYY-MM-DD'``

    Time value forms: ``'now'``, ISO-8601 string, Julian Day float, Unix int.

    Examples::

        DATE('now')                   → '2024-03-15'  (today's date)
        DATE('2024-01-31', '+1 month')→ '2024-02-29'  (leap-year clamp)
        DATE('2024-03-15', 'start of month') → '2024-03-01'
        DATE(NULL)                    → NULL
    """
    if not args:
        return None
    dt = _resolve_datetime(list(args))
    if dt is None:
        return None
    return dt.strftime("%Y-%m-%d")


@register("time")
def _time(*args: SqlValue) -> SqlValue:
    """Return a time string for the given time value and modifiers.

    ``TIME(timevalue [, modifier...])`` → ``'HH:MM:SS'``

    Examples::

        TIME('now')                  → '14:30:00'
        TIME('now', '+1 hour')       → '15:30:00'
        TIME(NULL)                   → NULL
    """
    if not args:
        return None
    dt = _resolve_datetime(list(args))
    if dt is None:
        return None
    return dt.strftime("%H:%M:%S")


@register("datetime")
def _datetime(*args: SqlValue) -> SqlValue:
    """Return a datetime string for the given time value and modifiers.

    ``DATETIME(timevalue [, modifier...])`` → ``'YYYY-MM-DD HH:MM:SS'``

    Examples::

        DATETIME('now')                      → '2024-03-15 14:30:00'
        DATETIME('now', 'start of year')     → '2024-01-01 00:00:00'
        DATETIME('now', '-1 day')            → yesterday at same time
        DATETIME(NULL)                       → NULL
    """
    if not args:
        return None
    dt = _resolve_datetime(list(args))
    if dt is None:
        return None
    return dt.strftime("%Y-%m-%d %H:%M:%S")


@register("julianday")
def _julianday(*args: SqlValue) -> SqlValue:
    """Return the Julian Day Number for the given time value and modifiers.

    ``JULIANDAY(timevalue [, modifier...])`` → float

    JDN 2451544.5 corresponds to 2000-01-01 00:00:00 UTC.

    Examples::

        JULIANDAY('2000-01-01')  → 2451544.5
        JULIANDAY('now')         → ~2460384.6  (varies)
        JULIANDAY(NULL)          → NULL
    """
    if not args:
        return None
    dt = _resolve_datetime(list(args))
    if dt is None:
        return None
    # Convert UTC datetime to Unix epoch seconds, then to Julian Day.
    unix_ts = dt.timestamp()
    return 2440587.5 + unix_ts / 86400.0


@register("unixepoch")
def _unixepoch(*args: SqlValue) -> SqlValue:
    """Return the Unix epoch (seconds since 1970-01-01 00:00:00 UTC) as integer.

    ``UNIXEPOCH(timevalue [, modifier...])`` → int

    Examples::

        UNIXEPOCH('1970-01-01')   → 0
        UNIXEPOCH('2000-01-01')   → 946684800
        UNIXEPOCH('now')          → current Unix timestamp
        UNIXEPOCH(NULL)           → NULL
    """
    if not args:
        return None
    dt = _resolve_datetime(list(args))
    if dt is None:
        return None
    return int(dt.timestamp())


@register("timediff")
def _timediff(a: SqlValue, b: SqlValue) -> SqlValue:
    """Return the calendar-aware difference *A* − *B* as text.

    ``TIMEDIFF(A, B)`` returns a string of the form
    ``±YYYY-MM-DD HH:MM:SS.sss`` representing how much later *A* is than
    *B*.  When *A* is earlier than *B* the sign is ``-`` and the
    magnitude is the time from *A* to *B*.  Returns NULL if either
    argument is NULL or fails to parse as a time value.

    Available in SQLite 3.43+ (see https://sqlite.org/lang_datefunc.html).

    **Calendar borrowing.**  The output components are *not* simply
    seconds-converted: the year and month fields use calendar
    arithmetic with monthly borrowing.  Concretely, the algorithm
    walks the seven fields from microseconds upward and borrows from
    the next higher field when the current one goes negative.  The
    month-borrow step uses the day count of the month immediately
    preceding ``A``'s month — that's what makes
    ``timediff('2024-03-15', '2024-01-20')`` come out to
    ``'+0000-01-24 00:00:00.000'`` (24 days because February 2024 has
    29 days, hence ``15 + 29 − 20 = 24``).

    Examples::

        TIMEDIFF('2024-01-02 10:30:00', '2024-01-01 09:00:00')
            → '+0000-00-01 01:30:00.000'
        TIMEDIFF('2024-01-01 09:00:00', '2024-01-02 10:30:00')
            → '-0000-00-01 01:30:00.000'
        TIMEDIFF('2025-01-01', '2024-01-01') → '+0001-00-00 00:00:00.000'
        TIMEDIFF('2024-02-29', '2024-01-31') → '+0000-00-29 00:00:00.000'
        TIMEDIFF('not-a-date', '2024-01-01') → NULL
    """
    if a is None or b is None:
        return None
    dt_a = _parse_timevalue(a)
    dt_b = _parse_timevalue(b)
    if dt_a is None or dt_b is None:
        return None
    sign = "+"
    if dt_a < dt_b:
        sign = "-"
        dt_a, dt_b = dt_b, dt_a
    # Field-by-field calendar borrow.  Subtract from microseconds up;
    # borrow from the next higher field whenever a component goes
    # negative.  The day-borrow uses the day count of the month preceding
    # ``dt_a``'s month, which is what gives ``timediff`` its
    # calendar-aware (rather than seconds-only) flavour.
    micro = dt_a.microsecond - dt_b.microsecond
    sec = dt_a.second - dt_b.second
    if micro < 0:
        micro += 1_000_000
        sec -= 1
    minute = dt_a.minute - dt_b.minute
    if sec < 0:
        sec += 60
        minute -= 1
    hour = dt_a.hour - dt_b.hour
    if minute < 0:
        minute += 60
        hour -= 1
    day = dt_a.day - dt_b.day
    if hour < 0:
        hour += 24
        day -= 1
    month = dt_a.month - dt_b.month
    if day < 0:
        # Borrow one month — add the day count of the *previous* month
        # (in dt_a's frame of reference) to the day field.
        if dt_a.month > 1:
            prev_month_year, prev_month = dt_a.year, dt_a.month - 1
        else:
            prev_month_year, prev_month = dt_a.year - 1, 12
        day += calendar.monthrange(prev_month_year, prev_month)[1]
        month -= 1
    year = dt_a.year - dt_b.year
    if month < 0:
        month += 12
        year -= 1
    # Truncate microseconds to milliseconds (3 decimal places) — matches
    # SQLite's output format.  No rounding; SQLite truncates here too.
    millis = micro // 1000
    return (
        f"{sign}{year:04d}-{month:02d}-{day:02d} "
        f"{hour:02d}:{minute:02d}:{sec:02d}.{millis:03d}"
    )


# Pre-compiled substitution map for STRFTIME's SQLite-specific specifiers.
# Python's strftime does not support %f (SQLite = SS.SSS), %s (epoch),
# %J (Julian day), or %P (lowercase am/pm on macOS libc).  We intercept
# those and pre-substitute concrete strings, then let Python format
# everything else.  ``%W`` (week-of-year, Monday-based, 00–53) used to be
# intercepted here because of a misreading of the spec — Python's
# strftime(%W) already produces SQLite-compatible output, so it's now
# routed through the default path.
_STRFTIME_PREPROCESS = re.compile(r"%[fJjsP]")


def _sqlite_strftime(fmt: str, dt: datetime) -> str:
    """Format *dt* using SQLite's strftime specifiers, delegating to Python.

    Cross-platform note on ``%P``
    -----------------------------
    SQLite's ``%P`` is lowercase ``am``/``pm`` (the GNU extension).  Python's
    own ``strftime`` honours this on Linux libc but not on macOS libc — on
    macOS ``dt.strftime('%P')`` returns the literal ``'P'``.  We pre-process
    ``%P`` ourselves so output is identical on every platform.
    """
    def _replace(m: re.Match) -> str:  # type: ignore[type-arg]
        spec = m.group(0)
        if spec == "%f":
            # SQLite %f = SS.SSS (3 decimal places)
            return f"{dt.second:02d}.{dt.microsecond // 1000:03d}"
        if spec == "%s":
            return str(int(dt.timestamp()))
        if spec == "%J":
            unix_ts = dt.timestamp()
            return str(2440587.5 + unix_ts / 86400.0)
        if spec == "%j":
            return f"{dt.timetuple().tm_yday:03d}"
        if spec == "%P":
            # Lowercase am/pm — Python's macOS libc doesn't support %P so we
            # synthesise it from %p (uppercase) and lowercase the result.
            return "am" if dt.hour < 12 else "pm"
        return spec

    processed = _STRFTIME_PREPROCESS.sub(_replace, fmt)
    return dt.strftime(processed)


@register("strftime")
def _strftime(*args: SqlValue) -> SqlValue:
    """Format a time value using C-style strftime format specifiers.

    ``STRFTIME(format, timevalue [, modifier...])`` → text

    SQLite-specific extensions beyond standard strftime:

    - ``%f`` → ``SS.SSS`` (seconds with 3-decimal-place fraction)
    - ``%s`` → Unix epoch as decimal integer string
    - ``%J`` → Julian Day number as float string

    Examples::

        STRFTIME('%Y-%m', 'now')              → '2024-03'
        STRFTIME('%s', '2000-01-01')          → '946684800'
        STRFTIME('%Y-%m-%d', 'now', '-7 days')→ last week's date
        STRFTIME(NULL, 'now')                 → NULL
        STRFTIME('%Y', NULL)                  → NULL
    """
    if len(args) < 2:
        return None
    fmt = args[0]
    if fmt is None:
        return None
    if not isinstance(fmt, str):
        return None
    dt = _resolve_datetime(list(args), skip_first=True)
    if dt is None:
        return None
    try:
        return _sqlite_strftime(fmt, dt)
    except (ValueError, OSError):
        return None


# ---------------------------------------------------------------------------
# JSON functions (SQLite-compatible subset)
# ---------------------------------------------------------------------------
#
# SQLite ships a built-in JSON1 extension that exposes a family of functions
# for creating, extracting, and mutating JSON documents.  Our implementation
# uses Python's :mod:`json` stdlib and mirrors SQLite semantics as closely as
# possible.
#
# Type mapping
# ~~~~~~~~~~~~
# SQL → JSON (for inputs):
#   None        → null
#   bool        → true / false
#   int         → integer
#   float       → real
#   str         → string
#   bytes/blob  → NULL (blobs are not representable in JSON; SQLite also
#                 returns NULL when a blob appears in a JSON context)
#
# JSON → SQL (for outputs extracted by json_extract):
#   null        → None (SQL NULL)
#   true        → 1   (SQLite uses integers, not booleans)
#   false       → 0
#   integer     → int
#   real        → float
#   string      → str
#   array       → str  (the JSON text of the array)
#   object      → str  (the JSON text of the object)
#
# Path format
# ~~~~~~~~~~~
# SQLite JSON paths start with ``$`` (the root) followed by zero or more
# selectors:
#   ``$.field``   — object key access
#   ``$[N]``      — array index (0-based, negative indices allowed)
#   ``$[#-1]``    — last element (``#`` means array length)
# Selectors chain: ``$.a.b[0].c``
#
# Reference: https://www.sqlite.org/json1.html


# --- internal helpers -------------------------------------------------------

_JSON_PATH_COMPONENT = re.compile(
    r'\.([^\.\[\]]+)'  # .field  (everything up to the next dot or bracket)
    r'|\[(-?\d+|\#(?:[+-]\d+)?)\]'  # [N] or [#] or [#-N] array indexing
)

# Safety cap: reject JSON strings larger than this to prevent CPU/memory DoS
# via pathologically large or deeply-nested documents.  1 MB is generous for
# any realistic SQL JSON value while still blocking obvious abuse.
_JSON_MAX_BYTES: int = 1_000_000  # 1 MB


def _safe_json_loads(s: str) -> tuple[object, bool]:
    """Parse *s* as JSON and return ``(parsed, ok)``.

    Returns ``(None, False)`` if *s* exceeds :data:`_JSON_MAX_BYTES`, is not
    valid JSON, or causes a ``RecursionError`` (deeply-nested documents).
    This is the single choke-point for all user-supplied JSON in the VM; it
    prevents CPU/memory denial-of-service via crafted payloads.
    """
    if len(s) > _JSON_MAX_BYTES:
        return None, False
    try:
        return _json.loads(s), True
    except (ValueError, TypeError, RecursionError):
        return None, False


def _sql_to_json_val(v: SqlValue) -> object:
    """Convert a SQL scalar value to a Python value suitable for json.dumps.

    Blobs (bytes) have no JSON representation and are converted to None,
    which serialises as JSON ``null``.  This matches SQLite's behaviour.
    """
    if isinstance(v, (type(None), bool, int, float, str)):
        return v
    if isinstance(v, (bytes, bytearray)):
        return None   # blobs → null
    return str(v)


def _json_to_sql_val(v: object) -> SqlValue:
    """Convert a Python JSON value back to a SQL scalar.

    JSON booleans are integers (true→1, false→0) in SQLite.
    Nested arrays and objects are returned as their JSON text representation.
    """
    if v is None:
        return None
    if isinstance(v, bool):
        return 1 if v else 0
    if isinstance(v, int):
        return v
    if isinstance(v, float):
        return v
    if isinstance(v, str):
        return v
    if isinstance(v, (list, dict)):
        return _json.dumps(v, separators=(",", ":"))
    return str(v)


def _json_type_name(v: object) -> str:
    """Return the SQLite json_type() name for a Python JSON value.

    Possible return values: ``"null"``, ``"true"``, ``"false"``,
    ``"integer"``, ``"real"``, ``"text"``, ``"array"``, ``"object"``.
    """
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return "integer"
    if isinstance(v, float):
        return "real"
    if isinstance(v, str):
        return "text"
    if isinstance(v, list):
        return "array"
    if isinstance(v, dict):
        return "object"
    return "text"


def _json_navigate(root: object, path: str) -> tuple[object, bool]:
    """Follow *path* from *root* and return ``(value, found)``.

    Returns ``(None, False)`` when any segment of the path does not exist.
    """
    if not isinstance(path, str) or not path.startswith("$"):
        return None, False
    remainder = path[1:]
    current = root
    while remainder:
        m = _JSON_PATH_COMPONENT.match(remainder)
        if m is None:
            return None, False
        remainder = remainder[m.end():]
        field = m.group(1)
        idx_s = m.group(2)
        if field is not None:
            # Object key access: $.field
            if not isinstance(current, dict):
                return None, False
            if field not in current:
                return None, False
            current = current[field]
        else:
            # Array index access: $[N] or $[#] or $[#-N]
            if not isinstance(current, list):
                return None, False
            length = len(current)
            if idx_s == "#":
                idx = length        # past-the-end (used internally)
            elif idx_s.startswith("#"):
                delta = int(idx_s[1:])  # e.g. "#-1" → -1
                idx = length + delta
            else:
                idx = int(idx_s)
            if idx < 0:
                idx += length
            if idx < 0 or idx >= length:
                return None, False
            current = current[idx]
    return current, True


def _json_set_path(
    root: object,
    path: str,
    value: object,
    *,
    insert: bool,
    replace: bool,
) -> object:
    """Return a deep copy of *root* with *path* set to *value*.

    - ``insert=True, replace=False`` → only creates, never overwrites (json_insert)
    - ``insert=False, replace=True`` → only overwrites, never creates (json_replace)
    - ``insert=True, replace=True``  → creates or overwrites (json_set)
    """
    if not isinstance(path, str) or not path.startswith("$"):
        return root
    if path == "$":
        return value    # replace root — unusual but valid

    # Collect path segments so we can navigate to the parent.
    segments: list[tuple[str, object]] = []  # ("field", name) or ("index", int)
    remainder = path[1:]
    while remainder:
        m = _JSON_PATH_COMPONENT.match(remainder)
        if m is None:
            return root   # malformed path → no-op
        remainder = remainder[m.end():]
        field = m.group(1)
        idx_s = m.group(2)
        if field is not None:
            segments.append(("field", field))
        else:
            segments.append(("index", idx_s))

    # Deep copy so we don't mutate the original.
    result = copy.deepcopy(root)

    # Navigate to the parent node.
    parent: object = result
    for seg_type, seg_key in segments[:-1]:
        if seg_type == "field":
            if not isinstance(parent, dict) or seg_key not in parent:
                return result   # path does not exist → no-op
            parent = parent[seg_key]  # type: ignore[index]
        else:
            if not isinstance(parent, list):
                return result
            length = len(parent)
            if isinstance(seg_key, str):
                idx = length if seg_key == "#" else (
                    length + int(seg_key[1:]) if seg_key.startswith("#")
                    else int(seg_key)
                )
            else:
                idx = int(seg_key)
            if idx < 0:
                idx += length
            if idx < 0 or idx >= length:
                return result
            parent = parent[idx]   # type: ignore[index]

    # Apply the final segment.
    last_type, last_key = segments[-1]
    if last_type == "field":
        if not isinstance(parent, dict):
            return result
        exists = last_key in parent
        if (exists and replace) or (not exists and insert):
            parent[last_key] = value  # type: ignore[index]
    else:
        if not isinstance(parent, list):
            return result
        length = len(parent)
        if isinstance(last_key, str):
            idx = (
                length if last_key == "#" else
                (length + int(last_key[1:]) if last_key.startswith("#") else int(last_key))
            )
        else:
            idx = int(last_key)
        if idx < 0:
            idx += length
        if idx == length and insert:
            parent.append(value)   # type: ignore[union-attr]
        elif 0 <= idx < length and replace:
            parent[idx] = value    # type: ignore[index]

    return result


def _json_remove_path(root: object, path: str) -> object:
    """Return a deep copy of *root* with the node at *path* removed."""
    if not isinstance(path, str) or not path.startswith("$"):
        return root
    if path == "$":
        return root  # cannot remove root

    segments: list[tuple[str, object]] = []
    remainder = path[1:]
    while remainder:
        m = _JSON_PATH_COMPONENT.match(remainder)
        if m is None:
            return root
        remainder = remainder[m.end():]
        field = m.group(1)
        idx_s = m.group(2)
        if field is not None:
            segments.append(("field", field))
        else:
            segments.append(("index", idx_s))

    result = copy.deepcopy(root)
    parent: object = result
    for seg_type, seg_key in segments[:-1]:
        if seg_type == "field":
            if not isinstance(parent, dict) or seg_key not in parent:
                return result
            parent = parent[seg_key]  # type: ignore[index]
        else:
            if not isinstance(parent, list):
                return result
            length = len(parent)
            sk = str(seg_key)
            if sk == "#":
                idx = length
            elif sk.startswith("#"):
                idx = length + int(sk[1:])
            else:
                idx = int(sk)
            if idx < 0:
                idx += length
            if idx < 0 or idx >= length:
                return result
            parent = parent[idx]   # type: ignore[index]

    last_type, last_key = segments[-1]
    if last_type == "field":
        if isinstance(parent, dict) and last_key in parent:
            del parent[last_key]  # type: ignore[attr-defined]
    else:
        if isinstance(parent, list):
            length = len(parent)
            lk = str(last_key)
            if lk == "#":
                idx = length
            elif lk.startswith("#"):
                idx = length + int(lk[1:])
            else:
                idx = int(lk)
            if idx < 0:
                idx += length
            if 0 <= idx < length:
                parent.pop(idx)   # type: ignore[union-attr]

    return result


# --- public JSON functions ---------------------------------------------------


@register("json")
def _json_fn(x: SqlValue) -> SqlValue:
    """Return the canonical minified form of a JSON string.

    Parses *x* and re-serialises it with no extra whitespace.  Returns NULL
    if *x* is NULL or not valid JSON.

    Examples::

        JSON('{ "a" : 1, "b" : 2 }')   → '{"a":1,"b":2}'
        JSON('[1, 2,  3]')              → '[1,2,3]'
        JSON(NULL)                      → NULL
        JSON('invalid')                 → NULL
    """
    if x is None:
        return None
    if not isinstance(x, str):
        return None
    parsed, ok = _safe_json_loads(x)
    if not ok:
        return None
    return _json.dumps(parsed, separators=(",", ":"))


@register("json_valid")
def _json_valid(x: SqlValue) -> SqlValue:
    """Return 1 if *x* is valid JSON, 0 otherwise.  NULL input → NULL.

    Matches SQLite 3.45+ behaviour where ``JSON_VALID(NULL)`` returns NULL
    rather than 0.  For non-null, non-string inputs the result is 0.

    Examples::

        JSON_VALID('{"a":1}')    → 1
        JSON_VALID('[1,2,3]')    → 1
        JSON_VALID('invalid')    → 0
        JSON_VALID(NULL)         → NULL
    """
    if x is None:
        return None
    if not isinstance(x, str):
        return 0
    _, ok = _safe_json_loads(x)
    return 1 if ok else 0


@register("json_quote")
def _json_quote_fn(x: SqlValue) -> SqlValue:
    """Convert a SQL value to its JSON representation as a TEXT string.

    This is the inverse of ``json_extract`` for scalar values.

    - NULL → ``"null"``
    - integers → decimal string: ``1``
    - floats   → decimal string: ``3.14``
    - text     → double-quoted JSON string: ``"hello"``
    - blob     → ``"null"``  (blobs not representable)

    Examples::

        JSON_QUOTE(NULL)         → 'null'
        JSON_QUOTE(42)           → '42'
        JSON_QUOTE(3.14)         → '3.14'
        JSON_QUOTE('hello')      → '"hello"'
        JSON_QUOTE('it"s')       → '"it\\"s"'
    """
    return _json.dumps(_sql_to_json_val(x), separators=(",", ":"))


@register("json_array")
def _json_array_fn(*args: SqlValue) -> SqlValue:
    """Build a JSON array from zero or more SQL values.

    Each argument becomes one element.  NULL arguments become JSON null.

    Examples::

        JSON_ARRAY(1, 2, 3)        → '[1,2,3]'
        JSON_ARRAY('a', NULL, 'b') → '["a",null,"b"]'
        JSON_ARRAY()               → '[]'
    """
    items = [_sql_to_json_val(a) for a in args]
    return _json.dumps(items, separators=(",", ":"))


@register("json_object")
def _json_object_fn(*args: SqlValue) -> SqlValue:
    """Build a JSON object from alternating key/value pairs.

    ``JSON_OBJECT(key1, val1, key2, val2, ...)``

    Keys must be TEXT.  Values follow the SQL-to-JSON mapping (NULL → null,
    etc.).  An odd number of arguments raises an error.  Duplicate keys
    produce an object where the last value wins (matching SQLite behaviour).

    Examples::

        JSON_OBJECT('a', 1, 'b', 2)     → '{"a":1,"b":2}'
        JSON_OBJECT('x', NULL)          → '{"x":null}'
        JSON_OBJECT('n', 3.14)          → '{"n":3.14}'
    """
    if len(args) % 2 != 0:
        raise WrongNumberOfArguments(
            name="json_object",
            expected="an even number",
            got=len(args),
        )
    obj: dict[str, object] = {}
    for i in range(0, len(args), 2):
        key = args[i]
        val = args[i + 1]
        if key is None:
            # SQLite returns an error for a NULL key; we follow suit by
            # converting it to the string "null" which is the least-bad
            # behaviour without raising a hard exception.
            key = "null"
        obj[str(key)] = _sql_to_json_val(val)
    return _json.dumps(obj, separators=(",", ":"))


@register("json_extract")
def _json_extract(*args: SqlValue) -> SqlValue:
    """Extract one or more values from a JSON document at the given paths.

    ``JSON_EXTRACT(json, path1 [, path2 ...])``

    - One path: returns the SQL value at that path (NULL if not found).
    - Multiple paths: returns a JSON array of the extracted values.

    Extracted scalars are returned as SQL values (string, integer, real, NULL).
    Extracted arrays/objects are returned as their JSON text.

    Examples::

        JSON_EXTRACT('{"a":1,"b":2}', '$.a')       → 1
        JSON_EXTRACT('[10,20,30]', '$[1]')          → 20
        JSON_EXTRACT('{"a":{"b":3}}', '$.a.b')     → 3
        JSON_EXTRACT('{"a":1}', '$.missing')       → NULL
        JSON_EXTRACT('{"a":1}', '$.a', '$.b')      → '[1,null]'
        JSON_EXTRACT(NULL, '$.a')                  → NULL
    """
    if len(args) < 2:
        raise WrongNumberOfArguments(name="json_extract", expected="at least 2", got=len(args))
    json_str = args[0]
    paths = args[1:]

    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    results: list[object] = []
    for path in paths:
        if not isinstance(path, str):
            results.append(None)
            continue
        val, found = _json_navigate(doc, path)
        results.append(val if found else None)

    if len(results) == 1:
        return _json_to_sql_val(results[0])
    # Multiple paths → return as JSON array.
    return _json.dumps(results, separators=(",", ":"))


def _path_arg_to_jsonpath(arg: SqlValue) -> str | None:
    """Convert a ``->`` / ``->>`` right-hand side to a SQLite JSON path.

    SQLite accepts three forms for the RHS of the JSON path-shortcut operators:

    * **Integer** ``N``  →  ``$[N]``  (array index access)
    * **String** ``"a"``  →  ``$.a``  (object key access)
    * **String already starting with ``$``** → used verbatim, allowing the
      caller to write e.g. ``j -> '$.a.b'`` or ``j -> '$[0].name'``.

    Returns ``None`` if *arg* is NULL or an unsupported type; the caller
    propagates NULL on receipt of None.
    """
    if arg is None:
        return None
    if isinstance(arg, bool):
        return None  # SQLite rejects booleans on the right of -> / ->>
    if isinstance(arg, int):
        return f"$[{arg}]"
    if isinstance(arg, str):
        if arg.startswith("$"):
            return arg
        return f"$.{arg}"
    return None


@register("__json_arrow")
def _json_arrow(json_val: SqlValue, path_arg: SqlValue) -> SqlValue:
    """Implement ``json -> path`` — JSON-typed path extraction.

    The result is always re-encoded as JSON text so that downstream
    chains (``j -> 'a' -> 'b'``) keep operating on JSON-shaped strings.
    Matches SQLite 3.38+ semantics:

    * Scalar results are JSON-quoted: ``'[1,2,3]' -> 0`` → ``'1'``
      (note: the *string* "1", not the integer 1).
    * Object / array results are returned as canonical JSON text.
    * NULL inputs propagate to NULL.
    """
    if json_val is None or path_arg is None:
        return None
    path = _path_arg_to_jsonpath(path_arg)
    if path is None:
        return None
    if not isinstance(json_val, str):
        return None
    doc, ok = _safe_json_loads(json_val)
    if not ok:
        return None
    val, found = _json_navigate(doc, path)
    if not found:
        return None
    # Always re-serialise as JSON.  This makes ``j -> 'a' -> 'b'`` work
    # because the intermediate string is still parseable JSON.
    return _json.dumps(val, separators=(",", ":"))


@register("__json_arrow_text")
def _json_arrow_text(json_val: SqlValue, path_arg: SqlValue) -> SqlValue:
    """Implement ``json ->> path`` — SQL-typed path extraction.

    Returns the SQL scalar at the path:

    * Strings as TEXT, numbers as INTEGER/REAL, null as NULL.
    * Arrays and objects are still returned as JSON text (matching
      SQLite — ``->>`` does NOT unwrap composite values).
    """
    if json_val is None or path_arg is None:
        return None
    path = _path_arg_to_jsonpath(path_arg)
    if path is None:
        return None
    if not isinstance(json_val, str):
        return None
    doc, ok = _safe_json_loads(json_val)
    if not ok:
        return None
    val, found = _json_navigate(doc, path)
    if not found:
        return None
    return _json_to_sql_val(val)


@register("json_type")
def _json_type(*args: SqlValue) -> SqlValue:
    """Return the type of a JSON value or a value within a JSON document.

    ``JSON_TYPE(json)``          → type of the root element
    ``JSON_TYPE(json, path)``    → type of the element at *path*

    Return values: ``"null"``, ``"true"``, ``"false"``, ``"integer"``,
    ``"real"``, ``"text"``, ``"array"``, ``"object"``.

    Returns NULL if the JSON is invalid or the path does not exist.

    Examples::

        JSON_TYPE('{"a":1}')              → 'object'
        JSON_TYPE('[1,2,3]')              → 'array'
        JSON_TYPE('"hello"')              → 'text'
        JSON_TYPE('{"a":1}', '$.a')       → 'integer'
        JSON_TYPE('{"a":null}', '$.a')    → 'null'
        JSON_TYPE('{"a":1}', '$.missing') → NULL
    """
    _arity("json_type", list(args), 1, 2)
    json_str = args[0]
    path = args[1] if len(args) == 2 else "$"

    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    if path == "$":
        return _json_type_name(doc)

    val, found = _json_navigate(doc, path)  # type: ignore[arg-type]
    if not found:
        return None
    return _json_type_name(val)


@register("json_array_length")
def _json_array_length(*args: SqlValue) -> SqlValue:
    """Return the number of elements in a JSON array.

    ``JSON_ARRAY_LENGTH(json)``          → length of root array
    ``JSON_ARRAY_LENGTH(json, path)``    → length of array at *path*

    Returns NULL if the JSON is invalid, the path does not exist, or the
    target element is not an array.

    Examples::

        JSON_ARRAY_LENGTH('[1,2,3]')              → 3
        JSON_ARRAY_LENGTH('{"a":[1,2]}', '$.a')   → 2
        JSON_ARRAY_LENGTH('{"a":1}')              → NULL   (not an array)
        JSON_ARRAY_LENGTH(NULL)                   → NULL
    """
    _arity("json_array_length", list(args), 1, 2)
    json_str = args[0]
    path = args[1] if len(args) == 2 else "$"

    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    if path == "$":
        target = doc
    else:
        target, found = _json_navigate(doc, path)  # type: ignore[arg-type]
        if not found:
            return None

    if not isinstance(target, list):
        # Valid JSON but not an array — SQLite returns 0, not NULL.
        # "The json_array_length(X) function returns … 0 if X is some kind of
        # JSON value other than an array" — SQLite documentation.
        return 0
    return len(target)


@register("json_keys")
def _json_keys(*args: SqlValue) -> SqlValue:
    """Return a JSON array of the keys in a JSON object.

    ``JSON_KEYS(json)``         → keys of the root object
    ``JSON_KEYS(json, path)``   → keys of the object at *path*

    Returns NULL if the target element is not an object or the path does
    not exist.

    Examples::

        JSON_KEYS('{"a":1,"b":2}')         → '["a","b"]'
        JSON_KEYS('{"x":{"y":3}}', '$.x')  → '["y"]'
        JSON_KEYS('[1,2]')                  → NULL   (not an object)
        JSON_KEYS(NULL)                     → NULL
    """
    _arity("json_keys", list(args), 1, 2)
    json_str = args[0]
    path = args[1] if len(args) == 2 else "$"

    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    if path == "$":
        target = doc
    else:
        target, found = _json_navigate(doc, path)  # type: ignore[arg-type]
        if not found:
            return None

    if not isinstance(target, dict):
        return None
    return _json.dumps(list(target.keys()), separators=(",", ":"))


@register("json_patch")
def _json_patch(target: SqlValue, patch: SqlValue) -> SqlValue:
    """Apply an RFC 7396 JSON Merge Patch to a JSON document.

    ``JSON_PATCH(target, patch)``

    The merge patch algorithm:
    - If *patch* is an object, recursively merge into *target*.
    - A key in *patch* whose value is null removes that key from *target*.
    - Other values in *patch* overwrite or insert into *target*.
    - If *patch* is not an object, replace *target* entirely with *patch*.

    Returns NULL if either argument is NULL or not valid JSON.

    Examples::

        JSON_PATCH('{"a":1,"b":2}', '{"b":null,"c":3}')
            → '{"a":1,"c":3}'
        JSON_PATCH('[1,2]', '[3,4,5]')
            → '[3,4,5]'
    """
    if target is None or patch is None:
        return None
    if not isinstance(target, str) or not isinstance(patch, str):
        return None
    t, ok1 = _safe_json_loads(target)
    p, ok2 = _safe_json_loads(patch)
    if not ok1 or not ok2:
        return None

    def _merge(a: object, b: object) -> object:
        if not isinstance(b, dict):
            return b
        result = copy.deepcopy(a) if isinstance(a, dict) else {}
        for k, v in b.items():
            if v is None:
                result.pop(k, None)   # type: ignore[union-attr]
            else:
                result[k] = _merge(result.get(k), v)  # type: ignore[union-attr]
        return result

    return _json.dumps(_merge(t, p), separators=(",", ":"))


@register("json_remove")
def _json_remove(*args: SqlValue) -> SqlValue:
    """Remove one or more paths from a JSON document.

    ``JSON_REMOVE(json, path1 [, path2 ...])``

    Paths that do not exist are silently ignored.  Returns NULL if *json*
    is NULL or not valid JSON.

    Examples::

        JSON_REMOVE('{"a":1,"b":2}', '$.a')       → '{"b":2}'
        JSON_REMOVE('[1,2,3]', '$[1]')             → '[1,3]'
        JSON_REMOVE('{"a":1}', '$.missing')       → '{"a":1}'
        JSON_REMOVE(NULL, '$.a')                   → NULL
    """
    if len(args) < 2:
        raise WrongNumberOfArguments(name="json_remove", expected="at least 2", got=len(args))
    json_str = args[0]
    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    for path in args[1:]:
        if isinstance(path, str):
            doc = _json_remove_path(doc, path)

    return _json.dumps(doc, separators=(",", ":"))


@register("json_set")
def _json_set(*args: SqlValue) -> SqlValue:
    """Insert or replace values in a JSON document.

    ``JSON_SET(json, path1, val1 [, path2, val2 ...])``

    Creates the path if it does not exist; overwrites it if it does.
    Arguments after the first must come in path/value pairs.

    Returns NULL if *json* is NULL or not valid JSON.

    Examples::

        JSON_SET('{"a":1}', '$.a', 99)         → '{"a":99}'
        JSON_SET('{"a":1}', '$.b', 2)          → '{"a":1,"b":2}'
        JSON_SET('[1,2,3]', '$[1]', 99)        → '[1,99,3]'
    """
    if len(args) < 3 or (len(args) - 1) % 2 != 0:
        raise WrongNumberOfArguments(
            name="json_set", expected="1 + even number of path/value pairs", got=len(args)
        )
    json_str = args[0]
    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    for i in range(1, len(args), 2):
        path = args[i]
        value = _sql_to_json_val(args[i + 1])
        if isinstance(path, str):
            doc = _json_set_path(doc, path, value, insert=True, replace=True)

    return _json.dumps(doc, separators=(",", ":"))


@register("json_insert")
def _json_insert(*args: SqlValue) -> SqlValue:
    """Insert values into a JSON document without overwriting existing paths.

    ``JSON_INSERT(json, path1, val1 [, path2, val2 ...])``

    Only inserts where the path does not yet exist.  Existing values are
    left unchanged (use ``JSON_SET`` to overwrite).

    Returns NULL if *json* is NULL or not valid JSON.

    Examples::

        JSON_INSERT('{"a":1}', '$.a', 99)   → '{"a":1}'     (no-op)
        JSON_INSERT('{"a":1}', '$.b', 2)    → '{"a":1,"b":2}'
    """
    if len(args) < 3 or (len(args) - 1) % 2 != 0:
        raise WrongNumberOfArguments(
            name="json_insert", expected="1 + even number of path/value pairs", got=len(args)
        )
    json_str = args[0]
    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    for i in range(1, len(args), 2):
        path = args[i]
        value = _sql_to_json_val(args[i + 1])
        if isinstance(path, str):
            doc = _json_set_path(doc, path, value, insert=True, replace=False)

    return _json.dumps(doc, separators=(",", ":"))


@register("json_replace")
def _json_replace(*args: SqlValue) -> SqlValue:
    """Replace existing values in a JSON document without creating new paths.

    ``JSON_REPLACE(json, path1, val1 [, path2, val2 ...])``

    Only replaces values where the path already exists.  Missing paths are
    silently ignored (use ``JSON_SET`` to create new keys).

    Returns NULL if *json* is NULL or not valid JSON.

    Examples::

        JSON_REPLACE('{"a":1}', '$.a', 99)   → '{"a":99}'
        JSON_REPLACE('{"a":1}', '$.b', 2)    → '{"a":1}'   (no-op)
    """
    if len(args) < 3 or (len(args) - 1) % 2 != 0:
        raise WrongNumberOfArguments(
            name="json_replace", expected="1 + even number of path/value pairs", got=len(args)
        )
    json_str = args[0]
    if json_str is None:
        return None
    if not isinstance(json_str, str):
        return None
    doc, ok = _safe_json_loads(json_str)
    if not ok:
        return None

    for i in range(1, len(args), 2):
        path = args[i]
        value = _sql_to_json_val(args[i + 1])
        if isinstance(path, str):
            doc = _json_set_path(doc, path, value, insert=False, replace=True)

    return _json.dumps(doc, separators=(",", ":"))


@register("json_group_array")
def _json_group_array_scalar(*args: SqlValue) -> SqlValue:
    """Scalar variant of json_group_array called with explicit value list.

    This is exposed for completeness but the real aggregate form
    (``SELECT json_group_array(col) FROM t GROUP BY ...``) is handled
    elsewhere.  Called as a scalar, it builds a JSON array from its
    arguments exactly like ``json_array``.

    Examples::

        JSON_GROUP_ARRAY(1, 2, 3)  → '[1,2,3]'
    """
    return _json_array_fn(*args)
