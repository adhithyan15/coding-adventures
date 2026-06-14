"""
InMemoryBackend — the reference Backend implementation
======================================================

This is the backend the mini-sqlite facade uses for ``connect(":memory:")``,
the one the test suite runs most of its assertions against, and the model
every other backend is measured against via the conformance tests.

Design choices
--------------

**Storage shape.** Each table is represented by a :class:`_Table` holding a
list of :class:`ColumnDef` (the schema) and a list of :data:`Row` (the rows,
in insertion order). Tables live in ``self._tables`` keyed by table name.

We use a list for rows rather than a dict keyed by some rowid for two
reasons. First, SQL semantics care about *insertion order* unless ``ORDER
BY`` says otherwise — a list preserves order for free. Second, the list
index is already a perfectly good rowid for positioned DML, so
:class:`ListCursor` can mutate rows in place without a separate id-to-row
map.

**Constraint enforcement.** Done lazily, per insert/update. We don't keep
per-column uniqueness indexes — a linear scan of the rows list is
correct, simple, and fast enough for the scale this backend targets
(thousands of rows, not millions). Trading speed for clarity here is a
deliberate choice.

**Transactions.** Snapshot-and-restore. ``begin_transaction`` deep-clones
``self._tables`` and stashes the clone in ``self._snapshot``. ``commit``
just drops the snapshot. ``rollback`` replaces ``self._tables`` with the
snapshot. This is O(total data size) per transaction, which would be
disastrous at scale but is fine for a pedagogical in-memory backend.

**Thread safety.** None. This backend is single-threaded. The ``Backend``
interface does not require concurrency; backends that need it (SQLite WAL,
a remote server) handle it internally.

Why this is longer than it looks
--------------------------------

Constraint checking has a lot of cases: NOT NULL on insert, NOT NULL on
update (only if the column is in the assignment), UNIQUE on insert (must
scan all rows), UNIQUE on update (must scan all rows *except* the one being
updated), default application (present vs absent vs NULL-with-default).
Each case has its own error path. The helper methods ``_apply_defaults`` /
``_check_not_null`` / ``_check_unique`` isolate each concern.
"""

from __future__ import annotations

import copy
import re
from collections.abc import Iterator
from typing import Final

from .backend import Backend, TransactionHandle
from .errors import (
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    IndexAlreadyExists,
    IndexNotFound,
    TableAlreadyExists,
    TableNotFound,
    TriggerAlreadyExists,
    TriggerNotFound,
    Unsupported,
)
from .index import IndexDef
from .row import Cursor, ListCursor, ListRowIterator, Row, RowIterator
from .schema import ColumnDef, TriggerDef
from .values import SqlValue

# ---------------------------------------------------------------------------
# SQLite-compatible sort key for in-memory index scans
# ---------------------------------------------------------------------------


def _sql_sort_key(v: SqlValue) -> tuple[int, object]:
    """Map a SQL value to a comparable Python key using SQLite ordering.

    SQLite BINARY collation orders values as::

        NULL (0) < INTEGER / REAL (1) < TEXT (2) < BLOB (3)

    Integers and floats are compared numerically across types (``2.0 == 2``).
    Text is compared by UTF-8 byte values (case-sensitive).  Blobs compare
    by raw byte value.

    Returns a ``(class, value)`` tuple that Python's ``<`` operator sorts
    correctly for each class.  Within class 1 (numeric) the raw Python value
    is used — Python handles mixed int/float comparisons natively.

    Examples::

        _sql_sort_key(None)    # (0, None)   — smallest
        _sql_sort_key(42)      # (1, 42)
        _sql_sort_key(3.14)    # (1, 3.14)
        _sql_sort_key("hi")    # (2, b"hi")  — text sorted as UTF-8 bytes
        _sql_sort_key(b"\\x00") # (3, b"\\x00") — blob last
    """
    if v is None:
        return (0, b"")  # sentinel: None < everything; b"" avoids cross-type compare
    if isinstance(v, bool):
        # bool is a subclass of int in Python; treat as integer.
        return (1, int(v))
    if isinstance(v, (int, float)):
        return (1, v)
    if isinstance(v, str):
        return (2, v.encode("utf-8"))
    if isinstance(v, (bytes, bytearray)):
        return (3, bytes(v))
    # SqlValue is a closed union — if we reach here the caller passed a
    # value that is not part of the type contract.  Raise immediately rather
    # than silently leaking the repr() of an arbitrary object (which could
    # contain secrets or cause non-deterministic sort behaviour).
    raise TypeError(  # noqa: TRY301
        f"_sql_sort_key: unsupported value type {type(v).__name__!r}"
    )


# Hidden row-identity key stored inside every Row dict.  The null-byte
# prefix guarantees that no SQL identifier (which must be printable UTF-8)
# can collide with this name, so user columns named "rowid" are stored
# separately under their real name and take precedence in column resolution.
#
# The rowid convention follows real SQLite:
# - Assigned starting at 1 and strictly monotonically increasing.
# - Never reused within a table's lifetime (even after DELETE).
# - Stable: deleting other rows does not change a surviving row's rowid.
#
# ``ScanAllColumns`` in the VM skips this key when expanding SELECT *.
_ROWID_KEY: Final[str] = "\x00rowid"


def _find_ipk_column(t: _Table) -> ColumnDef | None:
    """Return the table's INTEGER PRIMARY KEY column, or None.

    In SQLite an INTEGER PRIMARY KEY column is an alias for the rowid.
    We recognise it as ``primary_key=True`` plus a type-name in
    ``{INT, INTEGER}`` (case-insensitive).  Composite primary keys
    (which currently can't be expressed in the in-memory backend
    because each column has its own ``primary_key`` flag) do not get
    rowid-alias treatment — the user must supply explicit values.
    """
    for col in t.columns:
        if col.primary_key and col.type_name.upper() in ("INT", "INTEGER"):
            return col
    return None


def _strict_type_label(value: object) -> str:
    """Return the SQLite type-name for *value*, used in STRICT-mode error messages.

    SQLite uses ``INT`` (abbreviated) for integer *values* in the STRICT
    error message — different from ``INTEGER`` which it uses for the
    *column* declared type.  Matching this exactly keeps the message
    byte-identical to real ``sqlite3``::

        cannot store INT value in BLOB column t.x
        cannot store REAL value in INTEGER column t.x
        cannot store TEXT value in INTEGER column t.x
    """
    if isinstance(value, bool):
        return "INT"
    if isinstance(value, int):
        return "INT"
    if isinstance(value, float):
        return "REAL"
    if isinstance(value, str):
        return "TEXT"
    if isinstance(value, (bytes, bytearray)):
        return "BLOB"
    # Fallback for unknown Python types — surfaces the Python class name
    # so the message remains diagnostic instead of opaque.
    return type(value).__name__


def _strict_coerce(value: object, declared: str) -> tuple[object, bool]:
    """Attempt SQLite-STRICT lossless coercion of *value* to column *declared*.

    Returns ``(coerced_value, True)`` if coercion succeeded (the
    coerced value may be identical to *value* when no promotion was
    needed), or ``(value, False)`` if the value cannot be stored in a
    column of type *declared*.

    The promotion rules mirror real SQLite 3.37+ behaviour and were
    pinned by oracle tests against ``sqlite3`` 3.50.
    """
    # bool is a Python subclass of int; SQLite stores it as INTEGER 0/1.
    if declared in ("INT", "INTEGER"):
        if isinstance(value, bool):
            return int(value), True
        if isinstance(value, int):
            return value, True
        if isinstance(value, float):
            # Only whole-number REALs are losslessly representable as INT.
            if value.is_integer():
                return int(value), True
            return value, False
        if isinstance(value, str):
            # SQLite parses leading whitespace and integer literals only —
            # we keep the same shape by going through ``int()``.  Floats
            # like '1.5' are rejected (would lose precision).
            try:
                return int(value, 10), True
            except (TypeError, ValueError):
                return value, False
        return value, False

    if declared == "REAL":
        if isinstance(value, bool):
            return float(value), True
        if isinstance(value, (int, float)):
            return float(value), True
        if isinstance(value, str):
            try:
                return float(value), True
            except (TypeError, ValueError):
                return value, False
        return value, False

    if declared == "TEXT":
        if isinstance(value, str):
            return value, True
        if isinstance(value, bool):
            # SQLite renders booleans through their integer form: 0/1.
            return str(int(value)), True
        if isinstance(value, int):
            return str(value), True
        if isinstance(value, float):
            # SQLite's TEXT-cast of REAL uses ``%.17g``-ish formatting;
            # Python's ``str(float)`` is close enough for our purposes
            # and keeps round-trip parsing intact.
            return str(value), True
        # BLOB → TEXT is forbidden by SQLite STRICT.
        return value, False

    if declared == "BLOB":
        if isinstance(value, (bytes, bytearray)):
            return bytes(value), True
        return value, False

    # Unknown declared type — defensive: refuse the value so the error
    # message points at the column.  In practice this branch is
    # unreachable because ``create_table`` validates the type-name set.
    return value, False


#: Synthetic table names that expose the schema catalog.  Both names refer
#: to the same data — SQLite added ``sqlite_schema`` in 3.33 as the
#: preferred spelling while keeping ``sqlite_master`` for back-compat.
#: Queries against either name go through the same synthesis path; CREATE
#: TABLE, DROP TABLE, INSERT, UPDATE, and DELETE on either name raise an
#: error.
_MASTER_NAMES: Final[frozenset[str]] = frozenset({"sqlite_master", "sqlite_schema"})

#: The SQLite-managed table that tracks per-table AUTOINCREMENT
#: high-water rowids.  Only materialises (in real SQLite) when at least
#: one ``AUTOINCREMENT`` table exists; querying it on a fresh database
#: returns ``no such table``.  Mini-sqlite mirrors that contract: the
#: synthesizer returns rows iff at least one user table has an
#: AUTOINCREMENT column; otherwise ``scan('sqlite_sequence')`` raises
#: ``TableNotFound`` so the caller sees the same error sqlite3 produces.
_SEQUENCE_NAME: Final[str] = "sqlite_sequence"


def _sequence_columns() -> list[ColumnDef]:
    """Return a fresh copy of the ``sqlite_sequence`` column schema.

    Two columns: ``name`` (the AUTOINCREMENT table's name) and ``seq``
    (the high-water rowid — the largest value ever assigned to the
    INTEGER PRIMARY KEY column, even after the row was deleted).
    """
    return [
        ColumnDef(name="name", type_name="TEXT"),
        ColumnDef(name="seq", type_name="INTEGER"),
    ]

#: Fixed column schema for ``sqlite_master`` (matches SQLite 3).
#:
#: =========  =======  =====================================================
#: name       type     meaning
#: =========  =======  =====================================================
#: type       TEXT     'table' | 'index' | 'view' | 'trigger'
#: name       TEXT     name of the schema object
#: tbl_name   TEXT     name of the table the object belongs to (= name for
#:                     tables; the indexed table for indexes; the trigger's
#:                     target table for triggers)
#: rootpage   INTEGER  page number — meaningful only on disk; the in-memory
#:                     backend returns 0
#: sql        TEXT     the CREATE statement that defined the object;
#:                     reconstructed from ColumnDef metadata.  NULL for
#:                     auto-generated indexes (matches SQLite).
#: =========  =======  =====================================================
def _master_columns() -> list[ColumnDef]:
    """Return a fresh copy of the ``sqlite_master`` column schema.

    A fresh list per call keeps callers from accidentally aliasing the
    same list and mutating the canonical schema.
    """
    return [
        ColumnDef(name="type", type_name="TEXT"),
        ColumnDef(name="name", type_name="TEXT"),
        ColumnDef(name="tbl_name", type_name="TEXT"),
        ColumnDef(name="rootpage", type_name="INTEGER"),
        ColumnDef(name="sql", type_name="TEXT"),
    ]


#: Pattern that matches a safe bare SQL identifier, optionally followed by
#: a parenthesized length/precision argument list (e.g. ``VARCHAR(64)`` or
#: ``DECIMAL(10,2)``).  ``_sanitize_type_name`` uses this to decide whether
#: a type-name string is safe to interpolate verbatim into the synthesized
#: ``sqlite_master.sql`` column; anything that doesn't match is replaced
#: with the SQLite NUMERIC affinity name to neutralize second-order
#: injection.
_SAFE_TYPE_NAME = re.compile(
    r"[A-Za-z_][A-Za-z0-9_]*\s*(\(\s*\d+(\s*,\s*\d+)?\s*\))?\s*$"
)

#: Pattern that matches a safe bare collation name — exactly one
#: identifier with no parentheses, separators, or punctuation.
_SAFE_COLLATION_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*$")


def _sanitize_type_name(s: str) -> str:
    """Return *s* verbatim if it's a recognisable type name, else NUMERIC.

    SQLite's CREATE TABLE grammar is famously lenient about the column-type
    slot — it accepts any token sequence and maps via affinity rules.  But
    when we round-trip the type into the synthesized ``sqlite_master.sql``
    column, a string like ``"INTEGER); DROP TABLE users;--"`` would inject
    SQL into a downstream consumer that re-executes our output.

    We accept the standard shape ``IDENT`` or ``IDENT(N)`` or
    ``IDENT(N,M)`` (case-insensitive).  Anything else is replaced with the
    NUMERIC affinity name — preserves the table's semantics (NUMERIC is
    the default affinity for unrecognised types in SQLite) while
    neutralising the injection vector.
    """
    return s if _SAFE_TYPE_NAME.match(s) else "NUMERIC"


def _sanitize_collation_name(s: str) -> str:
    """Return *s* verbatim if it's a bare identifier, else BINARY.

    Collation names in SQLite are bare identifiers (``BINARY``, ``NOCASE``,
    ``RTRIM``, plus any user-registered ones).  Treating an unsafe name
    as BINARY (the default) preserves correctness for ill-formed input
    while preventing SQL injection through the synthesized
    ``sqlite_master.sql`` column.
    """
    return s if _SAFE_COLLATION_NAME.match(s) else "BINARY"


def _quote_identifier(name: str) -> str:
    """Wrap *name* in SQLite-style double quotes, doubling any embedded quotes.

    SQLite's identifier-quoting rule (per the manual): a double-quoted
    identifier may contain any character except an unescaped ``"``;
    embedded quotes are doubled.  We always emit quotes — even for
    well-behaved bare identifiers — so the output is unambiguous when a
    downstream consumer (ORM, migration tool) re-executes the
    ``sqlite_master.sql`` column.  Without quoting, an identifier
    crafted to contain SQL syntax (e.g. a column declared with the name
    ``x); DROP TABLE users;--``) would render injectable SQL.
    """
    escaped = name.replace('"', '""')
    return f'"{escaped}"'


def _format_sql_literal(value: object) -> str:
    """Render *value* as a SQL literal suitable for ``DEFAULT`` clauses.

    Python's ``repr()`` is not a SQL literal serializer — for ``bytes``
    it produces ``b'...'`` (invalid SQL), and for strings with embedded
    quotes its escape rules don't match SQL's (which double the quote
    rather than backslash-escape it).  This helper handles each storage
    class explicitly so the synthesized ``sql`` column is round-trippable
    through real SQLite.
    """
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "1" if value else "0"
    if isinstance(value, float):
        # ``repr()`` emits ``inf`` / ``-inf`` / ``nan`` for non-finite
        # floats, which aren't valid SQL literals.  Real SQLite stores
        # these as NULL when round-tripped through TEXT representation;
        # NULL is the safest no-op here.
        import math
        if not math.isfinite(value):
            return "NULL"
        return repr(value)
    if isinstance(value, int):
        return repr(value)
    if isinstance(value, (bytes, bytearray)):
        # SQLite BLOB literal: X'hexdigits'
        return "X'" + bytes(value).hex().upper() + "'"
    # Default: treat as text — escape embedded single quotes by doubling.
    text = str(value).replace("'", "''")
    return f"'{text}'"


def _reconstruct_create_table(table: str, columns: list[ColumnDef], strict: bool) -> str:
    """Rebuild a ``CREATE TABLE`` statement from ColumnDef metadata.

    The in-memory backend doesn't preserve the user's literal SQL text, so
    the ``sql`` column of ``sqlite_master`` is regenerated from the
    structured schema.  The output is valid SQL but may differ from the
    user's original byte-for-byte (e.g. whitespace, keyword casing).
    Tools that only care about ``type`` / ``name`` / ``tbl_name`` (the
    common case for migration tools) don't notice.

    All identifiers are double-quoted (via :func:`_quote_identifier`) and
    default values go through :func:`_format_sql_literal` so a downstream
    consumer that re-executes the synthesized statement (a common ORM
    pattern) cannot be tricked into running attacker-shaped SQL even if
    a table or column name contains quoting punctuation.
    """
    parts: list[str] = []
    for col in columns:
        toks: list[str] = [
            _quote_identifier(col.name),
            _sanitize_type_name(col.type_name),
        ]
        if col.primary_key:
            toks.append("PRIMARY KEY")
            if col.autoincrement:
                toks.append("AUTOINCREMENT")
        if col.not_null and not col.primary_key:
            toks.append("NOT NULL")
        if col.unique and not col.primary_key:
            toks.append("UNIQUE")
        if col.has_default():
            toks.append(f"DEFAULT {_format_sql_literal(col.default)}")
        if col.collation:
            toks.append(f"COLLATE {_sanitize_collation_name(col.collation)}")
        parts.append(" ".join(toks))
    out = f"CREATE TABLE {_quote_identifier(table)} ({', '.join(parts)})"
    if strict:
        out += " STRICT"
    return out


def _reconstruct_create_index(index: IndexDef) -> str | None:
    """Rebuild a ``CREATE INDEX`` statement from an IndexDef, or None.

    Returns None for auto-generated indexes (those whose name starts with
    ``sqlite_autoindex_``) — SQLite stores NULL in the ``sql`` column of
    ``sqlite_master`` for those, since the user never wrote a literal
    CREATE INDEX statement.  All identifiers are double-quoted via
    :func:`_quote_identifier` so attacker-shaped names cannot inject SQL
    when a downstream consumer re-executes the statement.
    """
    if index.name.startswith("sqlite_autoindex_"):
        return None
    unique = "UNIQUE " if index.unique else ""
    cols = ", ".join(_quote_identifier(c) for c in index.columns)
    return (
        f"CREATE {unique}INDEX {_quote_identifier(index.name)} "
        f"ON {_quote_identifier(index.table)} ({cols})"
    )


#: The set of column types SQLite allows in a STRICT table.  When a table
#: is created with the ``STRICT`` option, every column's declared type must
#: be one of these (case-insensitive) — anything else raises a
#: ``ConstraintViolation`` at CREATE TABLE time.  ``ANY`` is a special
#: SQLite-only type meaning "no type-check", used inside STRICT tables to
#: opt back into lenient typing on a per-column basis.
_STRICT_ALLOWED_TYPES: Final[frozenset[str]] = frozenset({
    "INT",
    "INTEGER",
    "REAL",
    "TEXT",
    "BLOB",
    "ANY",
})


class _Table:
    """Storage for one table — schema plus rows, in insertion order."""

    def __init__(self, columns: list[ColumnDef], strict: bool = False) -> None:
        self.columns: list[ColumnDef] = list(columns)
        self.rows: list[Row] = []
        # Monotonically increasing rowid counter.  Starts at 1 to match
        # real SQLite; never decremented even after DELETE.
        self._next_rowid: int = 1
        # SQLite STRICT-table flag.  When True, ``insert`` / ``update``
        # type-check each value against the column's declared type — see
        # ``_check_strict_types``.  When False (the default), type
        # affinity is lenient, matching legacy SQLite.
        self.strict: bool = strict

    def column_def(self, name: str) -> ColumnDef | None:
        """Return the ColumnDef for ``name``, or None if no such column."""
        for col in self.columns:
            if col.name == name:
                return col
        return None


class InMemoryBackend(Backend):
    """Reference Backend implementation — stores all data in Python dicts and lists.

    Construct empty, then populate via ``create_table`` + ``insert``, or use
    :meth:`from_tables` to preload schema and rows in one call (useful in
    test fixtures).
    """

    def __init__(self) -> None:
        self._tables: dict[str, _Table] = {}
        # Index store: name → IndexDef.  The actual index data is not
        # maintained incrementally (inserts/updates/deletes don't update
        # in-memory index structures); scan_index does a linear scan at
        # call time.  This is fine for a pedagogical reference backend.
        self._indexes: dict[str, IndexDef] = {}
        # Trigger stores: name → TriggerDef (for uniqueness checks and DROP),
        # and table → ordered list of TriggerDef (for firing order).
        self._triggers: dict[str, TriggerDef] = {}
        self._triggers_by_table: dict[str, list[TriggerDef]] = {}
        # Snapshot for the currently-active transaction, if any. ``None``
        # means no transaction is open. We store a full deep copy rather
        # than a diff log because it is dramatically simpler and the data
        # volumes targeted by this backend are small.
        self._snapshot: dict[str, _Table] | None = None
        self._index_snapshot: dict[str, IndexDef] | None = None
        # We hand out handles as monotonically increasing integers. Reusing
        # handles across transactions would make stale-handle bugs silent;
        # this way an old handle will never match a new transaction.
        self._next_handle: int = 1
        self._active_handle: int | None = None
        # Savepoint stack: list of (name, tables_snapshot, indexes_snapshot).
        # Each SAVEPOINT pushes a deep-copy; RELEASE pops; ROLLBACK TO
        # restores from a snapshot but keeps the entry so it can be reused.
        self._savepoint_stack: list[tuple[str, dict[str, _Table], dict[str, IndexDef]]] = []
        # PRAGMA user_version / schema_version backing fields (real SQLite
        # stores these in the file header; for the in-memory backend we
        # just hold ints).  ``_schema_version`` increments on every
        # successful create_table / drop_table / create_index / drop_index.
        self._user_version: int = 0
        self._schema_version: int = 0

    # --- Construction helpers ---------------------------------------------

    @classmethod
    def from_tables(
        cls,
        tables: dict[str, tuple[list[ColumnDef], list[Row]]],
    ) -> InMemoryBackend:
        """Build a backend pre-populated with ``tables``.

        ``tables`` maps table name → (column defs, rows). Rows are inserted
        directly into the backing list *without* constraint checks — this
        is a fixture helper, not a public API. If you want constraint
        checking, call ``create_table`` + ``insert`` explicitly.
        """
        backend = cls()
        for name, (cols, rows) in tables.items():
            t = _Table(cols)
            # Assign stable rowids to pre-loaded rows — same 1-based
            # convention as live inserts.  from_tables is a fixture helper
            # used in tests; assigning rowids here ensures SELECT rowid
            # works correctly on pre-loaded data.
            for row in rows:
                stamped = dict(row)
                stamped[_ROWID_KEY] = t._next_rowid
                t._next_rowid += 1
                t.rows.append(stamped)
            backend._tables[name] = t
        return backend

    # --- Schema -----------------------------------------------------------

    def tables(self) -> list[str]:
        # Mirror SQLite: ``.tables`` and ``SELECT name FROM sqlite_master``
        # both return user tables only.  ``sqlite_master`` / ``sqlite_schema``
        # are visible *by name* via direct query but don't appear in this
        # listing.
        return list(self._tables.keys())

    def columns(self, table: str) -> list[ColumnDef]:
        # sqlite_master / sqlite_schema have a fixed five-column schema —
        # return it directly without touching ``_tables``.
        if table in _MASTER_NAMES:
            return _master_columns()
        if table == _SEQUENCE_NAME:
            return _sequence_columns()
        return list(self._require_table(table).columns)

    def _has_autoincrement_table(self) -> bool:
        """True iff at least one user table has an AUTOINCREMENT column.

        Matches SQLite's "sqlite_sequence materialises lazily" contract:
        the table only exists once an AUTOINCREMENT table has been
        declared.  Used by ``scan('sqlite_sequence')`` and friends to
        decide between synthesising rows or raising ``TableNotFound``.
        """
        for tbl in self._tables.values():
            for col in tbl.columns:
                if col.autoincrement:
                    return True
        return False

    def _synthesize_sequence_rows(self) -> list[Row]:
        """Build the ``sqlite_sequence`` row list.

        One row per AUTOINCREMENT user table.  The ``seq`` value is the
        high-water rowid for that table — ``_next_rowid - 1`` in our
        in-memory backend, since the counter starts at 1 and is never
        decremented (SQLite's AUTOINCREMENT guarantee).  Tables with no
        AUTOINCREMENT column are excluded — they're tracked via the
        normal rowid path which can reuse ids.
        """
        rows: list[Row] = []
        for name, tbl in self._tables.items():
            if not any(col.autoincrement for col in tbl.columns):
                continue
            # ``_next_rowid`` is the *next* value to assign, so the
            # high-water value is one less.  On a freshly-created
            # AUTOINCREMENT table with no inserts yet, ``_next_rowid``
            # is still 1, so ``seq`` is 0 — matches SQLite.
            seq = tbl._next_rowid - 1  # noqa: SLF001 — backend internals
            rows.append({"name": name, "seq": seq})
        return rows

    # --- Header fields (PRAGMA user_version / schema_version) -------------

    def get_user_version(self) -> int:
        """Return the user_version field (a u32 with no engine semantics).

        Defaults to 0 on a fresh backend.  Persistent backends (e.g.
        ``SqliteFileBackend``) read this from byte 60 of the page-1
        header; in-memory just holds it as an attribute.
        """
        return self._user_version

    def set_user_version(self, value: int) -> None:
        """Write *value* into ``user_version``; must fit in u32."""
        if not (0 <= value <= 0xFFFFFFFF):
            raise ValueError(
                f"user_version must fit in u32 (0 ≤ v ≤ {0xFFFFFFFF}), got {value}"
            )
        self._user_version = value

    def get_schema_version(self) -> int:
        """Return the schema cookie — bumped on every DDL operation."""
        return self._schema_version

    # --- Read -------------------------------------------------------------

    def scan(self, table: str) -> RowIterator:
        # sqlite_master / sqlite_schema are synthesized at scan() time from
        # the current schema state — no storage, no maintenance.  Both
        # names route to the same synthesizer.
        if table in _MASTER_NAMES:
            return ListRowIterator(self._synthesize_master_rows())
        # sqlite_sequence materialises lazily — only when at least one
        # user table has an AUTOINCREMENT column.  Otherwise raise
        # TableNotFound to match SQLite's "no such table" error.
        if table == _SEQUENCE_NAME:
            if not self._has_autoincrement_table():
                raise TableNotFound(table=table)
            return ListRowIterator(self._synthesize_sequence_rows())
        t = self._require_table(table)
        # Return a snapshot view — hand out shallow copies of rows so the
        # VM can mutate freely without corrupting our state. ListRowIterator
        # handles that copy on each next() call.
        return ListRowIterator(t.rows)

    def _synthesize_master_rows(self) -> list[Row]:
        """Build the ``sqlite_master`` row list from current schema state.

        Layout matches SQLite::

            type      | name        | tbl_name    | rootpage | sql
            ----------+-------------+-------------+----------+------------------
            'table'   | <user-name> | <user-name> | <n>      | CREATE TABLE ...
            'index'   | <idx-name>  | <tbl-name>  | <n>      | CREATE INDEX ...
            'trigger' | <trg-name>  | <tbl-name>  | 0        | CREATE TRIGGER ...

        Auto-generated indexes (``sqlite_autoindex_*``) get NULL in the
        ``sql`` column, matching real SQLite.  ``rootpage`` is meaningful
        only on disk; the in-memory backend assigns a stable monotonic
        positive integer per table/index (matching SQLite's convention
        that ``rootpage > 0`` means "exists in the b-tree").  Triggers
        and views have ``rootpage = 0`` — they're not b-tree objects.
        Page numbers are not stable across schema mutations: they're
        assigned in iteration order at synthesis time.
        """
        rows: list[Row] = []
        next_page = 1  # tables and indexes start at 1; triggers stay at 0
        # Tables in insertion order.
        for name, tbl in self._tables.items():
            rows.append({
                "type": "table",
                "name": name,
                "tbl_name": name,
                "rootpage": next_page,
                "sql": _reconstruct_create_table(name, tbl.columns, tbl.strict),
            })
            next_page += 1
        # Indexes in insertion order.
        for idx_name, idx in self._indexes.items():
            rows.append({
                "type": "index",
                "name": idx_name,
                "tbl_name": idx.table,
                "rootpage": next_page,
                "sql": _reconstruct_create_index(idx),
            })
            next_page += 1
        # Triggers in creation order (per table).  ``_triggers`` is keyed
        # by name; iterate ``_triggers_by_table`` to preserve per-table
        # creation order (matches SQLite's ordering inside sqlite_master).
        # Triggers do not have a b-tree root; ``rootpage`` is always 0.
        for tbl_name, trigs in self._triggers_by_table.items():
            for trg in trigs:
                rows.append({
                    "type": "trigger",
                    "name": trg.name,
                    "tbl_name": tbl_name,
                    "rootpage": 0,
                    "sql": getattr(trg, "sql", None),
                })
        return rows

    def _open_cursor(self, table: str) -> ListCursor:
        """Internal helper: produce a ListCursor the VM can use for UPDATE/DELETE.

        Not on the public Backend interface, but documented here because
        tests for update/delete need a way to get a cursor. The VM's normal
        flow is: open a scan, iterate with next(), then pass the iterator
        to ``update`` or ``delete`` — so in practice the VM never needs
        this helper. Tests do.

        ``sqlite_master`` / ``sqlite_schema`` are special-cased: the
        synthesized rows are wrapped in a ListCursor on the fly.  Cursors
        opened against the master table cannot be used for UPDATE/DELETE
        because the underlying rows are recomputed on every call —
        positional mutation would have no persistent target.  The
        existing TableNotFound from ``_require_table`` inside ``update``
        / ``delete`` is what fires in that case.
        """
        if table in _MASTER_NAMES:
            return ListCursor(self._synthesize_master_rows())
        if table == _SEQUENCE_NAME:
            if not self._has_autoincrement_table():
                raise TableNotFound(table=table)
            return ListCursor(self._synthesize_sequence_rows())
        t = self._require_table(table)
        return ListCursor(t.rows)

    # --- Write ------------------------------------------------------------

    def insert(self, table: str, row: Row) -> None:
        if table in _MASTER_NAMES or table == _SEQUENCE_NAME:
            raise ConstraintViolation(
                table=table,
                column=None,
                message=f"table {table} may not be modified",
            )
        t = self._require_table(table)
        full_row = self._apply_defaults(t, row)
        self._check_unknown_columns(table, t, full_row)
        # INTEGER PRIMARY KEY auto-assign: SQLite treats an INTEGER PRIMARY
        # KEY column as an alias for the rowid.  When the user omits the
        # column or passes NULL, SQLite assigns the next rowid; when the
        # user supplies an explicit value, ``_next_rowid`` is bumped past
        # it so a subsequent auto-assign doesn't collide.  Done before
        # ``_check_not_null`` because PRIMARY KEY implies NOT NULL.
        self._autoassign_ipk(t, full_row)
        self._check_not_null(table, t, full_row)
        if t.strict:
            self._check_strict_types(table, t, full_row)
        self._check_unique(table, t, full_row, ignore_index=None)
        # Stamp the row with its stable integer rowid AFTER constraint
        # checks pass.  The key ``_ROWID_KEY`` (null-prefixed, not a valid
        # SQL identifier) is invisible to normal column resolution and is
        # skipped by ScanAllColumns.  When an INTEGER PRIMARY KEY column
        # exists, the stamp matches that column's value (rowid is the IPK)
        # so ``SELECT rowid`` and ``SELECT id`` return identical results.
        ipk_col = _find_ipk_column(t)
        if ipk_col is not None and full_row.get(ipk_col.name) is not None:
            full_row[_ROWID_KEY] = full_row[ipk_col.name]
        else:
            full_row[_ROWID_KEY] = t._next_rowid
            t._next_rowid += 1
        t.rows.append(full_row)
        # Reflect any auto-assigned / default values back into the caller's
        # input dict so downstream consumers (e.g. the VM's
        # ``LoadLastInsertedColumn`` path for ``INSERT … RETURNING``)
        # observe the IPK auto-assign rather than the original NULL.
        # The hidden ``_ROWID_KEY`` stamp is excluded — the caller's
        # dict represents user-visible columns only.
        for k, v in full_row.items():
            if k == _ROWID_KEY:
                continue
            if k not in row or row.get(k) is None:
                row[k] = v

    def _autoassign_ipk(self, t: _Table, row: Row) -> None:
        """Fill in the INTEGER PRIMARY KEY column with the next rowid.

        Three cases:

        * Column absent or value is ``None`` → assign ``t._next_rowid``
          and bump the counter.
        * Column value is an explicit integer → bump the counter past
          that value so subsequent auto-assigns don't collide.
        * No INTEGER PRIMARY KEY column on the table → no-op (the
          ``_ROWID_KEY`` stamp still uses ``_next_rowid``).
        """
        ipk_col = _find_ipk_column(t)
        if ipk_col is None:
            return
        val = row.get(ipk_col.name)
        if val is None:
            row[ipk_col.name] = t._next_rowid
            t._next_rowid += 1
        elif isinstance(val, int) and not isinstance(val, bool):
            # User supplied an explicit id; bump the counter so the next
            # auto-assigned rowid doesn't collide.  SQLite's behaviour:
            # ``_next_rowid = max(_next_rowid, supplied_id + 1)``.
            if val >= t._next_rowid:
                t._next_rowid = val + 1

    def update(
        self,
        table: str,
        cursor: Cursor,
        assignments: dict[str, SqlValue],
    ) -> None:
        t = self._require_table(table)
        # We require our own ListCursor for positioned DML because we need
        # to know which index to mutate. Foreign cursors (from other
        # backends) don't make sense here — a backend can only update rows
        # it owns.
        if not isinstance(cursor, ListCursor):
            raise Unsupported(operation="update with non-native cursor")
        idx = cursor.current_index()
        if idx < 0 or idx >= len(t.rows):
            raise Unsupported(operation="update without current row")

        # Validate column names *before* applying any assignment — partial
        # updates would corrupt constraint invariants.
        for col_name in assignments:
            if t.column_def(col_name) is None:
                raise ColumnNotFound(table=table, column=col_name)

        # Build the proposed new row, then re-check constraints against the
        # new values. We must ignore the row at ``idx`` during the UNIQUE
        # check — a row never conflicts with itself.
        proposed = dict(t.rows[idx])
        proposed.update(assignments)
        self._check_not_null(table, t, proposed)
        if t.strict:
            self._check_strict_types(table, t, proposed)
        self._check_unique(table, t, proposed, ignore_index=idx)

        t.rows[idx] = proposed

    def delete(self, table: str, cursor: Cursor) -> None:
        t = self._require_table(table)
        if not isinstance(cursor, ListCursor):
            raise Unsupported(operation="delete with non-native cursor")
        idx = cursor.current_index()
        if idx < 0 or idx >= len(t.rows):
            raise Unsupported(operation="delete without current row")

        del t.rows[idx]
        # After deletion the cursor no longer has a valid current row. We
        # also shift the cursor's index back by one so that the next call
        # to next() returns what *used to be* idx+1 (now at idx after the
        # del). Without this adjustment we'd skip a row.
        cursor._idx -= 1  # noqa: SLF001 — tight coupling with ListCursor is intentional
        cursor._current = None  # noqa: SLF001

    # --- DDL --------------------------------------------------------------

    def create_table(
        self,
        table: str,
        columns: list[ColumnDef],
        if_not_exists: bool,
        *,
        strict: bool = False,
    ) -> None:
        if table in _MASTER_NAMES or table == _SEQUENCE_NAME:
            # SQLite reserves the ``sqlite_*`` prefix; the
            # master/schema/sequence names in particular cannot be
            # redeclared.  Surface a clear ConstraintViolation rather
            # than letting a row get inserted into a fictitious table.
            raise ConstraintViolation(
                table=table,
                column=None,
                message=f"object name reserved for internal use: {table}",
            )
        if table in self._tables:
            if if_not_exists:
                return
            raise TableAlreadyExists(table=table)
        # SQLite STRICT enforcement applies *first* at CREATE TABLE time:
        # every column must declare a type from the small allowed set.
        # Without this check, ``CREATE TABLE t(x BANANA) STRICT`` would
        # succeed; in real SQLite it errors immediately.
        if strict:
            for col in columns:
                if col.type_name.upper() not in _STRICT_ALLOWED_TYPES:
                    raise ConstraintViolation(
                        table=table,
                        column=col.name,
                        message=(
                            f"unknown datatype for {table}.{col.name}: "
                            f"\"{col.type_name}\""
                        ),
                    )
        self._tables[table] = _Table(columns, strict=strict)
        self._schema_version += 1

    def drop_table(self, table: str, if_exists: bool) -> None:
        if table in _MASTER_NAMES or table == _SEQUENCE_NAME:
            # ``DROP TABLE [IF EXISTS] sqlite_master`` (and friends) is
            # always wrong; the IF EXISTS branch would otherwise silently
            # succeed because the name isn't in ``_tables``.  Explicit
            # guard surfaces a clear error matching SQLite's behaviour.
            raise ConstraintViolation(
                table=table,
                column=None,
                message=f"table {table} may not be dropped",
            )
        if table not in self._tables:
            if if_exists:
                return
            raise TableNotFound(table=table)
        del self._tables[table]
        self._schema_version += 1

    def add_column(self, table: str, column: ColumnDef) -> None:
        if table not in self._tables:
            raise TableNotFound(table=table)
        tbl = self._tables[table]
        if any(c.name == column.name for c in tbl.columns):
            raise ColumnAlreadyExists(table=table, column=column.name)
        tbl.columns.append(column)
        # Backfill existing rows: default value if specified, NULL otherwise.
        fill_value: SqlValue = column.default if column.has_default() else None
        for row in tbl.rows:
            row[column.name] = fill_value

    def rename_table(self, old_name: str, new_name: str) -> None:
        if old_name not in self._tables:
            raise TableNotFound(table=old_name)
        if new_name in self._tables:
            raise TableAlreadyExists(table=new_name)
        # Move the _Table object under the new key.  dict order is
        # preserved, so the table moves to the end — SQLite does the
        # same when renaming (rename = drop + recreate in the
        # underlying b-tree page order).
        self._tables[new_name] = self._tables.pop(old_name)
        # Rewrite the ``table`` field on any indexes that referenced
        # the old name.  Indexes are keyed by their own name (not the
        # table's), so we mutate in place.
        for idx in list(self._indexes.values()):
            if idx.table == old_name:
                self._indexes[idx.name] = IndexDef(
                    name=idx.name,
                    table=new_name,
                    columns=idx.columns,
                    unique=idx.unique,
                )
        self._schema_version += 1

    def rename_column(self, table: str, old_name: str, new_name: str) -> None:
        if table not in self._tables:
            raise TableNotFound(table=table)
        tbl = self._tables[table]
        # Find the column.
        col_idx = next(
            (i for i, c in enumerate(tbl.columns) if c.name == old_name),
            None,
        )
        if col_idx is None:
            raise ColumnNotFound(table=table, column=old_name)
        # Reject duplicates BEFORE we mutate state.
        if any(c.name == new_name for c in tbl.columns):
            raise ColumnAlreadyExists(table=table, column=new_name)
        # Build a fresh ColumnDef under the new name; ColumnDef is a
        # dataclass without __init__ overrides so we use dataclasses.replace.
        import dataclasses
        tbl.columns[col_idx] = dataclasses.replace(tbl.columns[col_idx], name=new_name)
        # Rewrite the per-row dict: pop the old key, set the new one.
        for row in tbl.rows:
            row[new_name] = row.pop(old_name, None)
        # Rewrite any index whose ``columns`` list mentions the column.
        for idx in list(self._indexes.values()):
            if idx.table == table and old_name in idx.columns:
                new_cols = tuple(
                    new_name if c == old_name else c for c in idx.columns
                )
                self._indexes[idx.name] = IndexDef(
                    name=idx.name,
                    table=table,
                    columns=new_cols,
                    unique=idx.unique,
                )
        self._schema_version += 1

    def drop_column(self, table: str, column_name: str) -> None:
        if table not in self._tables:
            raise TableNotFound(table=table)
        tbl = self._tables[table]
        col_idx = next(
            (i for i, c in enumerate(tbl.columns) if c.name == column_name),
            None,
        )
        if col_idx is None:
            raise ColumnNotFound(table=table, column=column_name)
        col = tbl.columns[col_idx]
        # SQLite restrictions on DROP COLUMN:
        #   * column cannot be PRIMARY KEY
        #   * column cannot be referenced by an index
        #   * cannot drop the only column in the table
        if col.primary_key:
            raise ConstraintViolation(
                table=table,
                column=column_name,
                message=(
                    f"cannot DROP COLUMN '{column_name}': it is the "
                    f"PRIMARY KEY of '{table}'"
                ),
            )
        if len(tbl.columns) == 1:
            raise ConstraintViolation(
                table=table,
                column=column_name,
                message=(
                    f"cannot DROP COLUMN '{column_name}': it is the only "
                    f"column of '{table}'"
                ),
            )
        for idx in self._indexes.values():
            if idx.table == table and column_name in idx.columns:
                raise ConstraintViolation(
                    table=table,
                    column=column_name,
                    message=(
                        f"cannot DROP COLUMN '{column_name}': it is "
                        f"referenced by index '{idx.name}'"
                    ),
                )
        # Remove from the schema.
        tbl.columns.pop(col_idx)
        # Strip the column value from every existing row.
        for row in tbl.rows:
            row.pop(column_name, None)
        self._schema_version += 1

    # --- Transactions -----------------------------------------------------

    def begin_transaction(self) -> TransactionHandle:
        if self._active_handle is not None:
            raise Unsupported(operation="nested transactions")
        # Deep-copy the whole table map. copy.deepcopy handles the nested
        # list-of-dicts correctly — each row dict is cloned, each value
        # inside is immutable (SqlValue is scalar) so we don't need to
        # recurse deeper.
        self._snapshot = copy.deepcopy(self._tables)
        # Also snapshot the index definitions so that create_index /
        # drop_index inside a rolled-back transaction leave no trace.
        self._index_snapshot = copy.deepcopy(self._indexes)
        handle = self._next_handle
        self._next_handle += 1
        self._active_handle = handle
        return TransactionHandle(handle)

    def commit(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # Changes are already applied to self._tables — we just discard the
        # rollback snapshot.
        self._snapshot = None
        self._index_snapshot = None
        self._active_handle = None

    def rollback(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # _require_active guarantees _snapshot is set whenever _active_handle is.
        assert self._snapshot is not None
        self._tables = self._snapshot
        # Restore index definitions from the snapshot.
        assert self._index_snapshot is not None
        self._indexes = self._index_snapshot
        self._snapshot = None
        self._index_snapshot = None
        self._active_handle = None

    def current_transaction(self) -> TransactionHandle | None:
        """Return the active transaction handle, or ``None`` if no transaction
        is currently open.

        Because the InMemoryBackend stores the handle internally, this method
        can bridge the gap between separate :func:`~sql_vm.vm.execute` calls:
        ``BeginTransaction`` stores the handle; subsequent ``CommitTransaction``
        / ``RollbackTransaction`` calls retrieve it here rather than relying on
        the (by then discarded) VM state object.
        """
        if self._active_handle is None:
            return None
        return TransactionHandle(self._active_handle)

    def create_savepoint(self, name: str) -> None:
        """Push a deep-copy snapshot of the current tables and indexes.

        Each SAVEPOINT call appends to the stack; multiple savepoints with the
        same name stack independently (SQLite allows this).  If a transaction
        is not already active, ``create_savepoint`` implicitly begins one so
        the savepoint has something to anchor to.

        Deep-copying is O(data size) but acceptable for the pedagogical scale
        this backend targets.
        """
        if self._active_handle is None:
            # Implicitly begin a transaction so the savepoint is anchored.
            self.begin_transaction()
        snap_tables = copy.deepcopy(self._tables)
        snap_indexes = copy.deepcopy(self._indexes)
        self._savepoint_stack.append((name, snap_tables, snap_indexes))

    def release_savepoint(self, name: str) -> None:
        """Remove the named savepoint (and all savepoints after it).

        Finds the *last* entry in the stack with the given name, removes it
        and every entry that was pushed after it.  The current table state is
        not changed — this is a "partial commit" up to the release point.

        Raises :class:`~sql_backend.errors.Unsupported` if no savepoint with
        that name exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"RELEASE {name!r}: no such savepoint")
        del self._savepoint_stack[idx:]

    def rollback_to_savepoint(self, name: str) -> None:
        """Restore the database to the state it was in when *name* was created.

        Finds the *last* savepoint with the given name, restores tables and
        indexes from its snapshot, and removes all savepoints pushed after it.
        The named savepoint itself is kept in the stack so the caller may roll
        back to it again or release it later.

        Raises :class:`~sql_backend.errors.Unsupported` if no savepoint with
        that name exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"ROLLBACK TO {name!r}: no such savepoint")
        _name, snap_tables, snap_indexes = self._savepoint_stack[idx]
        # Restore state from the snapshot.
        self._tables = copy.deepcopy(snap_tables)
        self._indexes = copy.deepcopy(snap_indexes)
        # Drop all savepoints created after this one; keep this one alive.
        del self._savepoint_stack[idx + 1:]

    def _find_savepoint(self, name: str) -> int | None:
        """Return the index of the *last* savepoint named *name*, or ``None``."""
        for i in range(len(self._savepoint_stack) - 1, -1, -1):
            if self._savepoint_stack[i][0] == name:
                return i
        return None

    # --- Triggers ----------------------------------------------------------

    def create_trigger(self, defn: TriggerDef) -> None:
        """Store a trigger definition.

        Raises :class:`TriggerAlreadyExists` if a trigger with the same name
        already exists.
        """
        if defn.name in self._triggers:
            raise TriggerAlreadyExists(name=defn.name)
        self._triggers[defn.name] = defn
        self._triggers_by_table.setdefault(defn.table, []).append(defn)

    def drop_trigger(self, name: str, if_exists: bool = False) -> None:
        """Remove a trigger definition by name.

        Raises :class:`TriggerNotFound` when *name* is absent and
        ``if_exists=False``.
        """
        if name not in self._triggers:
            if if_exists:
                return
            raise TriggerNotFound(name=name)
        defn = self._triggers.pop(name)
        table_list = self._triggers_by_table.get(defn.table, [])
        self._triggers_by_table[defn.table] = [t for t in table_list if t.name != name]

    def list_triggers(self, table: str) -> list[TriggerDef]:
        """Return all triggers for *table* in creation order."""
        return list(self._triggers_by_table.get(table, []))

    # --- Indexes ----------------------------------------------------------

    def create_index(self, index: IndexDef) -> None:
        """Store an index definition and validate it against the schema.

        The in-memory backend does not build a sorted data structure for
        the index at creation time — :meth:`scan_index` performs a linear
        scan of the table rows instead.  This is correct (though O(n)) and
        appropriate for a pedagogical reference backend.

        Raises
        ------
        IndexAlreadyExists
            If an index named ``index.name`` already exists.
        TableNotFound
            If ``index.table`` is not a known table.
        ColumnNotFound
            If any column in ``index.columns`` is not a column of
            ``index.table``.
        """
        if index.name in self._indexes:
            raise IndexAlreadyExists(index=index.name)
        t = self._require_table(index.table)
        col_names = {col.name for col in t.columns}
        for col in index.columns:
            if col not in col_names:
                raise ColumnNotFound(table=index.table, column=col)
        self._indexes[index.name] = index
        self._schema_version += 1

    def drop_index(self, name: str, *, if_exists: bool = False) -> None:
        """Remove an index definition.

        Raises :class:`IndexNotFound` when *name* is absent and
        ``if_exists=False``.
        """
        if name not in self._indexes:
            if if_exists:
                return
            raise IndexNotFound(index=name)
        del self._indexes[name]
        self._schema_version += 1

    def list_indexes(self, table: str | None = None) -> list[IndexDef]:
        """Return all stored index definitions, optionally filtered by table.

        Returns indexes in creation order.
        """
        if table is None:
            return list(self._indexes.values())
        return [idx for idx in self._indexes.values() if idx.table == table]

    def scan_index(
        self,
        index_name: str,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[int]:
        """Yield list-indices of matching rows from the indexed table.

        For the in-memory backend the "rowid" exposed by ``scan_index`` is
        the 0-based position of the row in the table's row list — the same
        value used internally by :class:`ListCursor`.  This is consistent
        within the backend but is not comparable to the integer rowids used
        by file-backed backends.

        The scan is O(n): all rows are examined, key values are extracted
        and compared, then matching rows are yielded in ascending key order.
        This is correct for a pedagogical backend; file-backed backends do
        this in O(log n + k) via the B-tree index.

        Raises :class:`IndexNotFound` if *index_name* does not exist.
        """
        idx_def = self._indexes.get(index_name)
        if idx_def is None:
            raise IndexNotFound(index=index_name)

        t = self._require_table(idx_def.table)
        col_names = idx_def.columns

        # Build (sort_key, original_row_idx) pairs for all rows.
        # sort_key is a tuple of _sql_sort_key(v) values — one per index column.
        keyed: list[tuple[tuple[tuple[int, object], ...], int]] = []
        for i, row in enumerate(t.rows):
            key_vals = [row.get(col) for col in col_names]
            sort_key = tuple(_sql_sort_key(v) for v in key_vals)
            keyed.append((sort_key, i))

        # Sort by key — Python's tuple comparison does the right thing since
        # all elements are (int, comparable) pairs.
        keyed.sort(key=lambda kv: kv[0])

        lo_sort = tuple(_sql_sort_key(v) for v in lo) if lo is not None else None
        hi_sort = tuple(_sql_sort_key(v) for v in hi) if hi is not None else None

        for sort_key, row_idx in keyed:
            # Trim to the minimum length for partial-key comparison.
            if lo_sort is not None:
                cmp_lo = sort_key[: len(lo_sort)]
                if cmp_lo < lo_sort or (cmp_lo == lo_sort and not lo_inclusive):
                    continue
            if hi_sort is not None:
                cmp_hi = sort_key[: len(hi_sort)]
                if cmp_hi > hi_sort or (cmp_hi == hi_sort and not hi_inclusive):
                    return
            yield row_idx

    def scan_by_rowids(self, table: str, rowids: list[int]) -> RowIterator:
        """Return a RowIterator over the rows at the given list indices.

        For the in-memory backend every "rowid" is the 0-based position of a row
        in the table's internal list — exactly what :meth:`scan_index` yields.
        Rows are returned in the order the rowids are given; caller should sort
        if ascending order is required.

        Out-of-range indices are silently skipped.
        """
        t = self._require_table(table)
        rows = [t.rows[i] for i in rowids if 0 <= i < len(t.rows)]
        return ListRowIterator(rows)

    # --- Private helpers --------------------------------------------------

    def _require_table(self, table: str) -> _Table:
        t = self._tables.get(table)
        if t is None:
            raise TableNotFound(table=table)
        return t

    def _require_active(self, handle: TransactionHandle) -> None:
        if self._active_handle is None:
            raise Unsupported(operation="no active transaction")
        if int(handle) != self._active_handle:
            raise Unsupported(operation="stale transaction handle")

    def _apply_defaults(self, t: _Table, row: Row) -> Row:
        """Return ``row`` with any missing columns filled in from defaults.

        If a column is absent from the row and it has a DEFAULT, we insert
        the default value. If the column is absent and has no default, we
        leave it absent — NOT NULL / UNIQUE checks downstream will decide
        whether that's an error. (Absent columns produce NULL on read.)

        Column ordering: the returned dict's iteration order matches the
        table's declared column order, NOT the caller-supplied dict's
        order.  This matters because ``SELECT *`` walks the row dict in
        insertion order — if a user INSERT omitted a column (common with
        INTEGER PRIMARY KEY auto-assign), that column would otherwise
        end up at the END of the dict and surface in the wrong position.
        """
        out: Row = {}
        for col in t.columns:
            if col.name in row:
                out[col.name] = row[col.name]
            elif col.has_default():
                # col.default is ColumnDefault = SqlValue | _NoDefault.
                # has_default() ruled out the sentinel, so this cast is safe.
                out[col.name] = col.default  # type: ignore[assignment]
            else:
                # Missing + no default → NULL, so NOT NULL checks can see it.
                out[col.name] = None
        # Preserve any extra keys the caller passed (typically only the
        # hidden ``_ROWID_KEY``; unknown column names are rejected by
        # ``_check_unknown_columns`` immediately after this returns).
        for key, value in row.items():
            if key not in out:
                out[key] = value
        return out

    def _check_unknown_columns(self, table: str, t: _Table, row: Row) -> None:
        """Reject inserts that mention columns not in the schema."""
        known = {col.name for col in t.columns}
        for key in row:
            if key == _ROWID_KEY:
                # The hidden rowid key is stamped by insert() after this
                # check runs — but _apply_defaults may return a dict that
                # already carries _ROWID_KEY if the caller pre-stamped it.
                # Either way, it is always valid to ignore it here.
                continue
            if key not in known:
                raise ColumnNotFound(table=table, column=key)

    def _check_not_null(self, table: str, t: _Table, row: Row) -> None:
        """Enforce NOT NULL (including implicit NOT NULL from PRIMARY KEY)."""
        for col in t.columns:
            if col.effective_not_null() and row.get(col.name) is None:
                raise ConstraintViolation(
                    table=table,
                    column=col.name,
                    message=f"NOT NULL constraint failed: {table}.{col.name}",
                )

    def _check_strict_types(self, table: str, t: _Table, row: Row) -> None:
        """Enforce STRICT-table per-column type rules with SQLite-style coercion.

        SQLite's STRICT mode is *not* a pure isinstance check — it permits
        lossless type promotions and parses TEXT values that look like a
        number when the column wants a number.  Verified by oracle against
        real ``sqlite3`` 3.50+:

        ======  ======================================================
        Column  Accepts (with possible coercion-in-place)
        ======  ======================================================
        INT     int → int
        INTEGER float → int  *only if whole* (1.0 → 1; 1.5 rejected)
                str → int    *only if it parses as int* ('42' → 42)
        REAL    int → float (promotion)
                float → float
                str → float  *only if it parses as numeric*
        TEXT    str → str
                int → str (canonical decimal)
                float → str (canonical decimal)
        BLOB    bytes/bytearray → bytes
                everything else rejected
        ANY     value stored verbatim (the STRICT escape hatch)
        ======  ======================================================

        NULL is always permitted; NOT NULL is a separate check handled by
        :meth:`_check_not_null`.  Mismatches raise :class:`ConstraintViolation`
        with the SQLite-compatible message ``cannot store TYPE value in TYPE
        column table.col``.

        This method MUTATES *row* in place: when a coercion is permitted
        (e.g. INT 1 → REAL 1.0), the stored value is the promoted form.
        Callers already pass a fresh dict (``_apply_defaults`` or
        ``dict(t.rows[idx])``) so the caller's input isn't aliased.
        """
        for col in t.columns:
            val = row.get(col.name)
            if val is None:
                # NULL is exempt — NOT NULL handles required-ness separately.
                continue
            declared = col.type_name.upper()
            if declared == "ANY":
                continue
            coerced, ok = _strict_coerce(val, declared)
            if not ok:
                actual = _strict_type_label(val)
                raise ConstraintViolation(
                    table=table,
                    column=col.name,
                    message=(
                        f"cannot store {actual} value in {declared} column "
                        f"{table}.{col.name}"
                    ),
                )
            if coerced is not val:
                # Apply the promotion to the proposed row so the stored
                # form matches the column's declared type.
                row[col.name] = coerced

    def _check_unique(
        self,
        table: str,
        t: _Table,
        row: Row,
        ignore_index: int | None,
    ) -> None:
        """Enforce UNIQUE (including implicit UNIQUE from PRIMARY KEY).

        NULL never conflicts with anything — SQL semantics. A UNIQUE column
        may contain many NULLs. ``ignore_index`` is the row being updated,
        which must not conflict with itself.

        Error wording matches SQLite: both explicit ``UNIQUE`` and the
        implicit uniqueness of a ``PRIMARY KEY`` column surface as
        ``UNIQUE constraint failed: <table>.<col>``.  Earlier versions of
        mini-sqlite emitted ``PRIMARY KEY constraint failed: …`` for the
        PK case, which sqlite3 never produces.
        """
        for col in t.columns:
            if not col.effective_unique():
                continue
            new_val = row.get(col.name)
            if new_val is None:
                continue
            for i, existing in enumerate(t.rows):
                if i == ignore_index:
                    continue
                if existing.get(col.name) == new_val:
                    raise ConstraintViolation(
                        table=table,
                        column=col.name,
                        message=f"UNIQUE constraint failed: {table}.{col.name}",
                    )
