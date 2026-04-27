"""
Parameter binding — substitute ``?`` placeholders into SQL source text.

PEP 249 ``paramstyle = "qmark"``: the user writes SQL with ``?`` markers and
passes a parameter tuple. We must bind those parameters before the SQL text
is handed to the lexer — because the vendored SQL lexer has no QMARK token
and extending the grammar here would be a larger change than this facade
warrants.

The substitution runs a single left-to-right scan over the source. Inside
string literals (``'...'``, including the ``''`` escape for embedded single
quotes) and comments (``--...\\n`` and ``/* ... */``) the scanner does *not*
count ``?`` characters — those aren't parameter markers. Everywhere else,
each ``?`` is replaced with the SQL literal form of the corresponding
parameter.

Type mapping — the output must parse as a valid SQL literal:

    None                  → NULL
    True / False          → 1 / 0   (sqlite3 convention)
    int / float           → repr-style numeric literal
    str                   → single-quoted, with embedded ``'`` doubled
    bytes                 → not supported in v1 (PEP 249 allows a driver
                            to raise NotSupportedError for BLOB)

We validate arity up front: placeholders found in SQL must equal
``len(parameters)``. Over- or under-supply raises ProgrammingError —
matching sqlite3's behavior and PEP 249's expectations.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any

from .errors import NotSupportedError, ProgrammingError


def substitute(sql: str, parameters: Sequence[Any]) -> str:
    """Return ``sql`` with each ``?`` replaced by a SQL literal.

    Raises :class:`ProgrammingError` if the count of ``?`` markers found in
    the SQL doesn't match ``len(parameters)``.
    """
    out: list[str] = []
    i = 0
    n = len(sql)
    param_idx = 0

    while i < n:
        ch = sql[i]

        # String literal: ``'...'`` with backslash escapes (``\'``, ``\\``).
        # The underlying SQL lexer uses backslash-style escaping — we
        # scan past such escapes without ending the string.
        if ch == "'":
            start = i
            i += 1
            while i < n:
                if sql[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                if sql[i] == "'":
                    i += 1
                    break
                i += 1
            out.append(sql[start:i])
            continue

        # Line comment: from ``--`` to end-of-line.
        if ch == "-" and i + 1 < n and sql[i + 1] == "-":
            start = i
            while i < n and sql[i] != "\n":
                i += 1
            out.append(sql[start:i])
            continue

        # Block comment: ``/* ... */``.
        if ch == "/" and i + 1 < n and sql[i + 1] == "*":
            start = i
            i += 2
            while i + 1 < n and not (sql[i] == "*" and sql[i + 1] == "/"):
                i += 1
            i = min(i + 2, n)
            out.append(sql[start:i])
            continue

        # Placeholder.
        if ch == "?":
            if param_idx >= len(parameters):
                raise ProgrammingError(
                    f"not enough parameters supplied: SQL has at least "
                    f"{param_idx + 1} placeholders, got {len(parameters)}"
                )
            out.append(_to_sql_literal(parameters[param_idx]))
            param_idx += 1
            i += 1
            continue

        out.append(ch)
        i += 1

    if param_idx != len(parameters):
        raise ProgrammingError(
            f"too many parameters supplied: SQL has {param_idx} placeholders, "
            f"got {len(parameters)}"
        )
    return "".join(out)


def _to_sql_literal(value: Any) -> str:
    """Render a Python value as a SQL literal string.

    The output is fed back into the lexer, so it must tokenize cleanly.
    """
    # Subclasses of ``int``/``float``/``str`` can override ``__repr__`` or
    # ``replace`` to emit arbitrary text. Coerce to the base type before
    # formatting so a hostile subclass can't smuggle SQL past us.
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, int):
        return str(int(value))
    if isinstance(value, float):
        f = float(value)
        if not math.isfinite(f):
            raise ProgrammingError(f"cannot bind non-finite float: {value!r}")
        return repr(f)
    if isinstance(value, str):
        # The vendored SQL lexer recognises backslash escapes, not ANSI
        # doubled-quote escapes. Match its rules: escape backslashes and
        # single quotes with a preceding backslash. ``str.__str__`` forces
        # base-str semantics so a ``str`` subclass can't override escape.
        s = str.__str__(value)
        escaped = s.replace("\\", "\\\\").replace("'", "\\'")
        return f"'{escaped}'"
    if isinstance(value, bytes | bytearray | memoryview):
        raise NotSupportedError("BLOB parameters are not supported in v1")
    raise ProgrammingError(f"cannot bind value of type {type(value).__name__}")
