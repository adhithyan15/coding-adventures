"""
Parameter binding — substitute placeholders into SQL source text.

PEP 249 ``paramstyle``: this driver supports both ``"qmark"`` (``?``) and
``"named"`` (``:name``) styles, matching the stdlib ``sqlite3`` module.

* ``qmark``: caller passes a ``Sequence``.  Each ``?`` consumes the next
  positional parameter.  Arity must match exactly.
* ``named``: caller passes a ``Mapping``.  Each ``:identifier`` is replaced
  by ``parameters[identifier]``.  A missing key raises ``ProgrammingError``.

Mixing both styles in one statement is rejected — pick one paramstyle per
statement.  Inside string literals (``'...'``, with backslash escapes) and
comments (``--...\\n`` and ``/* ... */``) the scanner does *not* count
placeholders — those aren't parameter markers.

Type mapping — the output must parse as a valid SQL literal:

    None                  → NULL
    True / False          → 1 / 0   (sqlite3 convention)
    int / float           → repr-style numeric literal
    str                   → single-quoted, with embedded ``'`` doubled
    bytes / bytearray     → not supported in v1 (PEP 249 allows a driver
                            to raise NotSupportedError for BLOB)
"""

from __future__ import annotations

import math
import re
from collections.abc import Mapping, Sequence
from typing import Any

from .errors import NotSupportedError, ProgrammingError

# A named parameter is ``:identifier`` where identifier follows Python-ish
# identifier rules (letter or underscore, then letters/digits/underscores).
# This matches sqlite3's accepted shape; SQLite's own grammar additionally
# allows ``$identifier`` and ``@identifier`` but those are out of scope.
_IDENT_START = re.compile(r"[A-Za-z_]")
_IDENT_CONT = re.compile(r"[A-Za-z0-9_]")


def substitute(sql: str, parameters: Sequence[Any] | Mapping[str, Any]) -> str:
    """Return ``sql`` with each ``?`` or ``:name`` replaced by a SQL literal.

    The paramstyle is decided from the type of *parameters*:

    * ``Sequence`` (tuple, list, …) → qmark style.  Every ``?`` in the SQL
      consumes one positional parameter; the count must match exactly.
    * ``Mapping`` (dict, …) → named style.  Every ``:identifier`` is looked
      up by key in *parameters*.  Missing keys raise ``ProgrammingError``.

    Mixing ``?`` and ``:name`` markers in the same statement is rejected.

    Raises :class:`ProgrammingError` on any of:
      * arity mismatch (qmark)
      * unknown key (named)
      * mixed paramstyle in one statement
      * unsupported parameter type
    """
    is_mapping = isinstance(parameters, Mapping) and not isinstance(parameters, str | bytes)
    out: list[str] = []
    i = 0
    n = len(sql)
    pos_idx = 0
    seen_qmark = False
    seen_named = False

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

        # Qmark placeholder.
        if ch == "?":
            if seen_named:
                raise ProgrammingError(
                    "cannot mix '?' and ':name' parameter styles in one statement"
                )
            seen_qmark = True
            if is_mapping:
                raise ProgrammingError(
                    "SQL has '?' placeholders but parameters is a mapping; "
                    "pass a sequence for qmark style"
                )
            if pos_idx >= len(parameters):
                raise ProgrammingError(
                    f"not enough parameters supplied: SQL has at least "
                    f"{pos_idx + 1} placeholders, got {len(parameters)}"
                )
            out.append(_to_sql_literal(parameters[pos_idx]))
            pos_idx += 1
            i += 1
            continue

        # Named placeholder ``:identifier``.  Only treat as a placeholder if
        # the next character looks like the start of an identifier.  This
        # avoids false positives in expressions like ``a::INT`` (Postgres-
        # style cast) or stray colons.
        if (
            ch == ":"
            and i + 1 < n
            and _IDENT_START.match(sql[i + 1])
        ):
            j = i + 1
            while j < n and _IDENT_CONT.match(sql[j]):
                j += 1
            name = sql[i + 1 : j]
            seen_named = True
            if seen_qmark:
                raise ProgrammingError(
                    "cannot mix '?' and ':name' parameter styles in one statement"
                )
            if not is_mapping:
                raise ProgrammingError(
                    f"SQL has named parameter ':{name}' but parameters is not "
                    f"a mapping; pass a dict for named style"
                )
            if name not in parameters:
                raise ProgrammingError(
                    f"no value supplied for named parameter ':{name}'"
                )
            out.append(_to_sql_literal(parameters[name]))
            i = j
            continue

        out.append(ch)
        i += 1

    # Final arity check for qmark — too many positional params is also wrong.
    if seen_qmark and pos_idx != len(parameters):
        raise ProgrammingError(
            f"too many parameters supplied: SQL has {pos_idx} placeholders, "
            f"got {len(parameters)}"
        )
    # For an empty SQL with a non-empty sequence, also flag.
    if not seen_qmark and not seen_named and not is_mapping and len(parameters) > 0:
        raise ProgrammingError(
            f"too many parameters supplied: SQL has 0 placeholders, "
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
