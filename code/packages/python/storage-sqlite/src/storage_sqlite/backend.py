"""
SqliteFileBackend — the Backend adapter for real SQLite files (phase 7).

This module wires all the lower-level layers (pager, record, btree, freelist,
schema) together behind the :class:`~sql_backend.Backend` interface so that
``mini_sqlite.connect("foo.db")`` works end-to-end against a real SQLite file.

Architecture
------------

The class hierarchy is::

    sql_backend.Backend  (abstract interface)
        └── SqliteFileBackend
                ├── Pager          (page I/O, rollback journal)
                ├── Freelist       (page reuse)
                ├── Schema         (sqlite_schema catalog)
                └── BTree          (per-table B-trees, opened on demand)

Every DML or DDL operation opens the relevant B-tree fresh via the pager —
no stale state is cached. The pager's dirty-page table acts as the implicit
write buffer: changes are invisible to other connections until
``pager.commit()`` is called.

Row encoding
------------

SQLite stores rows as **records** (see :mod:`storage_sqlite.record`): a
header of serial-type varints followed by the value bytes, in column
declaration order. One special case: a column declared ``INTEGER PRIMARY
KEY`` is a **rowid alias** — its value IS the B-tree row key and is NOT
stored inside the record payload. Reading it back, we inject the rowid
directly into the decoded dict.

Transaction semantics
---------------------

* ``begin_transaction()`` — record an opaque handle; does not flush or lock.
  All writes go to the pager's dirty-page table immediately (they are
  visible to reads within the same connection but not persisted to disk).
* ``commit(handle)`` — call ``pager.commit()``, which fsyncs the journal,
  writes all dirty pages to the main file, fsyncs the main file, and
  removes the journal.
* ``rollback(handle)`` — call ``pager.rollback()``, which discards all
  dirty pages and leaves the file in its pre-transaction state.

Operations that happen *without* an explicit ``begin_transaction`` still
work: writes accumulate in dirty pages and reads see them within the same
session. They are silently lost on process exit unless eventually committed
via an explicit transaction.

Column parsing
--------------

The SQL string stored in ``sqlite_schema`` is the canonical source of truth
for column definitions. We generate it in :func:`_columns_to_sql` and parse
it back in :func:`_sql_to_columns`. The format is::

    CREATE TABLE <name> (<col> <type> [PRIMARY KEY] [NOT NULL] [UNIQUE]
                                      [DEFAULT <literal>], ...)

The parser handles the subset of SQLite column syntax used by this backend
and by real ``sqlite3`` for the same subset of table definitions (lowercase
keywords, implicit NOT NULL from PRIMARY KEY, etc.).

v1 limitations
--------------

* No index support. All scans are full table scans. The B-tree rows are
  in rowid order (insertion order for non-IPK tables).
* AUTOINCREMENT is not supported. ``INTEGER PRIMARY KEY`` is a plain rowid
  alias without the monotonicity guarantee of AUTOINCREMENT.
* The uniqueness check on insert is O(n) — a full scan per UNIQUE column.
* No ALTER TABLE. Tables must be recreated to change their schema.
* Single connection only. No advisory locking between processes.
"""

from __future__ import annotations

import contextlib
import copy
import os
import re
import struct
from collections.abc import Iterator

from sql_backend import (
    Backend,
    ColumnAlreadyExists,
    ColumnDef,
    ColumnNotFound,
    ConstraintViolation,
    IndexAlreadyExists,
    IndexDef,
    IndexNotFound,
    ListRowIterator,
    Row,
    RowIterator,
    SqlValue,
    TableAlreadyExists,
    TableNotFound,
    TransactionHandle,
    TriggerAlreadyExists,
    TriggerNotFound,
    Unsupported,
)
from sql_backend.schema import NO_DEFAULT, TriggerDef

from storage_sqlite import record as _record
from storage_sqlite.btree import BTree, DuplicateRowidError
from storage_sqlite.freelist import Freelist
from storage_sqlite.index_tree import IndexTree
from storage_sqlite.pager import Pager
from storage_sqlite.schema import Schema, SchemaError, initialize_new_database

# ---------------------------------------------------------------------------
# SQL column-definition helpers
# ---------------------------------------------------------------------------


def _format_literal(value: object) -> str:
    """Format a Python value as a SQL literal string.

    Used when generating DEFAULT clauses inside ``CREATE TABLE`` SQL.
    Handles the five SQL value types: NULL, integer, float, text, blob.
    """
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        # Python bool is a subclass of int — emit 1/0, not True/False.
        return "1" if value else "0"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    if isinstance(value, bytes):
        return f"X'{value.hex()}'"
    return repr(value)  # fallback — should not happen with SqlValue types


def _columns_to_sql(table: str, columns: list[ColumnDef]) -> str:
    """Serialise a list of :class:`ColumnDef` objects as a ``CREATE TABLE``
    statement.

    The generated SQL is parseable by both this module's :func:`_sql_to_columns`
    and by the real ``sqlite3`` CLI.

    Examples::

        _columns_to_sql("users", [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT", not_null=True),
        ])
        # → "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
    """
    parts: list[str] = []
    for col in columns:
        tokens: list[str] = [col.name, col.type_name]
        if col.primary_key:
            tokens.append("PRIMARY KEY")
        # NOT NULL: emit only when not implied by PRIMARY KEY to match sqlite3's
        # output style; PRIMARY KEY already implies NOT NULL in SQLite.
        if col.not_null and not col.primary_key:
            tokens.append("NOT NULL")
        if col.unique and not col.primary_key:
            tokens.append("UNIQUE")
        if col.has_default():
            tokens.append(f"DEFAULT {_format_literal(col.default)}")
        parts.append(" ".join(tokens))
    return f"CREATE TABLE {table} ({', '.join(parts)})"


# ---------------------------------------------------------------------------
# SQL → ColumnDef parser
# ---------------------------------------------------------------------------

# A minimal tokeniser that handles identifiers/keywords, numbers, single-
# quoted strings, and X'…' blob literals. Comments are stripped before this
# runs.
_SQL_TOKEN = re.compile(
    r"X'[0-9A-Fa-f]*'"            # blob literal X'DEADBEEF'
    r"|'(?:[^']|'')*'"            # single-quoted string ('' = escaped quote)
    r"|-?\d+(?:\.\d+)?(?:[Ee][+-]?\d+)?"  # number (int or float)
    r"|\w+"                       # identifier or keyword
    r"|[(),;]",                   # punctuation
    re.IGNORECASE,
)


def _tokenize(sql: str) -> list[str]:
    """Return a flat list of tokens from *sql*, stripping SQL comments.

    Only the token types needed for CREATE TABLE column definitions are
    recognised: identifiers, keywords, numbers, single-quoted strings,
    X'' blob literals, and punctuation. Whitespace is discarded.
    """
    # Strip line comments first so they don't confuse the regex.
    stripped = re.sub(r"--[^\n]*", "", sql)
    return _SQL_TOKEN.findall(stripped)


def _parse_literal(tok: str) -> object:
    """Convert a raw SQL literal token to a Python :data:`~sql_backend.SqlValue`.

    Token forms handled:

    * ``NULL``  → ``None``
    * Integer   → ``int``
    * Float     → ``float``
    * ``'…'``   → ``str`` (undoes ``''`` escaping)
    * ``X'…'``  → ``bytes``
    """
    if tok.upper() == "NULL":
        return None
    if tok.upper().startswith("X'"):
        hex_part = tok[2:-1]
        return bytes.fromhex(hex_part)
    if tok.startswith("'"):
        # Strip surrounding single quotes and unescape doubled single quotes.
        inner = tok[1:-1].replace("''", "'")
        return inner
    # Try integer first, then float.
    try:
        return int(tok)
    except ValueError:
        return float(tok)


def _split_column_defs(body: str) -> list[str]:
    """Split a comma-separated list of column definitions.

    Respects parenthesis depth so that expressions like ``DEFAULT (1+2)``
    are not split at the inner comma (though we don't generate such forms;
    this makes the parser robust against real ``sqlite3`` output).
    """
    parts: list[str] = []
    depth = 0
    buf: list[str] = []
    for ch in body:
        if ch == "(":
            depth += 1
            buf.append(ch)
        elif ch == ")":
            depth -= 1
            buf.append(ch)
        elif ch == "," and depth == 0:
            parts.append("".join(buf))
            buf = []
        else:
            buf.append(ch)
    if buf:
        parts.append("".join(buf))
    return parts


def _parse_one_column(col_sql: str) -> ColumnDef | None:
    """Parse a single column definition string into a :class:`ColumnDef`.

    Returns ``None`` for table-level constraints (e.g. ``PRIMARY KEY(id)``
    on a separate line) which start with a constraint keyword rather than a
    column name.

    Recognises::

        <name> <type> [PRIMARY KEY] [NOT NULL] [UNIQUE] [DEFAULT <literal>]

    Keywords are matched case-insensitively. Unknown tokens are skipped so
    the parser stays permissive when reading SQL produced by the real
    ``sqlite3`` CLI.
    """
    tokens = _tokenize(col_sql)
    if len(tokens) < 2:
        return None

    # Table-level constraints start with a keyword, not a column name.
    first_upper = tokens[0].upper()
    if first_upper in ("PRIMARY", "UNIQUE", "CHECK", "FOREIGN", "CONSTRAINT"):
        return None

    name = tokens[0]
    type_name = tokens[1]

    not_null = False
    primary_key = False
    unique = False
    default: object = NO_DEFAULT

    i = 2
    while i < len(tokens):
        tok_upper = tokens[i].upper()
        if tok_upper == "PRIMARY" and i + 1 < len(tokens) and tokens[i + 1].upper() == "KEY":
            primary_key = True
            i += 2
        elif tok_upper == "NOT" and i + 1 < len(tokens) and tokens[i + 1].upper() == "NULL":
            not_null = True
            i += 2
        elif tok_upper == "UNIQUE":
            unique = True
            i += 1
        elif tok_upper == "DEFAULT" and i + 1 < len(tokens):
            default = _parse_literal(tokens[i + 1])
            i += 2
        else:
            i += 1  # skip unknown token (e.g. AUTO_INCREMENT, REFERENCES, …)

    return ColumnDef(
        name=name,
        type_name=type_name,
        not_null=not_null,
        primary_key=primary_key,
        unique=unique,
        default=default,
    )


def _sql_to_columns(sql: str) -> list[ColumnDef]:
    """Extract :class:`ColumnDef` objects from a ``CREATE TABLE`` SQL string.

    Locates the opening and closing parentheses that delimit the column list,
    splits by comma, and delegates each column definition to
    :func:`_parse_one_column`. Table-level constraints (``PRIMARY KEY(…)``
    etc.) are silently skipped.

    Raises :class:`ValueError` if the SQL string contains no parentheses.

    Example::

        _sql_to_columns("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT NOT NULL)")
        # → [ColumnDef(name='id', ..., primary_key=True),
        #    ColumnDef(name='x', ..., not_null=True)]
    """
    start = sql.index("(")
    end = sql.rindex(")")
    body = sql[start + 1 : end]
    col_parts = _split_column_defs(body)
    columns: list[ColumnDef] = []
    for part in col_parts:
        col = _parse_one_column(part.strip())
        if col is not None:
            columns.append(col)
    return columns


# ---------------------------------------------------------------------------
# Index SQL helpers
# ---------------------------------------------------------------------------


def _quote_identifier(name: str) -> str:
    """Double-quote a SQL identifier, escaping any embedded double-quotes.

    SQLite (and the SQL standard) allows arbitrary characters in identifiers
    when they are wrapped in double-quotes, with embedded double-quotes
    represented as two consecutive double-quotes.  This prevents DDL
    injection when user-supplied names (table names, column names, index
    names) are interpolated into SQL strings.

    Examples::

        _quote_identifier("users")        # → '"users"'
        _quote_identifier('a"b')          # → '"a""b"'
        _quote_identifier("idx; DROP TABLE users; --")
        # → '"idx; DROP TABLE users; --"'  (safe — treated as a literal name)
    """
    return '"' + name.replace('"', '""') + '"'


def _parse_index_columns(sql: str) -> list[str]:
    """Extract column names from a ``CREATE INDEX`` statement.

    Locates the parenthesised column list and splits on commas.  Strips
    surrounding double-quote characters from each name so that names
    written by :func:`_columns_to_index_sql` (which quotes all identifiers)
    are returned as bare strings.

    Examples::

        _parse_index_columns(
            'CREATE INDEX "idx" ON "orders" ("user_id")'
        )
        # → ["user_id"]

        _parse_index_columns(
            'CREATE INDEX "idx" ON "orders" ("last_name", "first_name")'
        )
        # → ["last_name", "first_name"]

    Returns an empty list if the SQL contains no parentheses (defensive
    fallback for schema rows written by external tools with non-standard
    syntax).
    """
    try:
        start = sql.index("(") + 1
        end = sql.rindex(")")
    except ValueError:
        return []
    cols = []
    for tok in sql[start:end].split(","):
        tok = tok.strip()
        if not tok:
            continue
        # Strip surrounding double-quotes (SQLite identifier quoting).
        if tok.startswith('"') and tok.endswith('"') and len(tok) >= 2:
            tok = tok[1:-1].replace('""', '"')
        cols.append(tok)
    return cols


def _columns_to_index_sql(name: str, table: str, columns: list[str]) -> str:
    """Serialise an index definition as a quoted ``CREATE INDEX`` statement.

    All identifiers are double-quoted via :func:`_quote_identifier` to
    prevent DDL injection when user-supplied names contain SQL metacharacters.
    The output is stored verbatim in ``sqlite_schema`` and is parseable by
    both :func:`_parse_index_columns` and by the real ``sqlite3`` CLI::

        _columns_to_index_sql("idx_orders_user_id", "orders", ["user_id"])
        # → 'CREATE INDEX "idx_orders_user_id" ON "orders" ("user_id")'
    """
    col_list = ", ".join(_quote_identifier(c) for c in columns)
    return f"CREATE INDEX {_quote_identifier(name)} ON {_quote_identifier(table)} ({col_list})"


# ---------------------------------------------------------------------------
# Trigger SQL helpers
# ---------------------------------------------------------------------------


def _trigger_to_sql(defn: TriggerDef) -> str:
    """Serialise a :class:`~sql_backend.schema.TriggerDef` to a ``CREATE
    TRIGGER`` statement for storage in ``sqlite_schema``.

    The body is wrapped in ``BEGIN … END`` to form a syntactically complete
    SQL statement that is round-trippable via :func:`_sql_to_trigger_def`.

    Example::

        _trigger_to_sql(TriggerDef(
            name="trg_audit", table="orders",
            timing="AFTER", event="INSERT",
            body="INSERT INTO audit VALUES (NEW.id);",
        ))
        # → 'CREATE TRIGGER "trg_audit" AFTER INSERT ON "orders"\\nBEGIN\\n...\\nEND'
    """
    return (
        f"CREATE TRIGGER {_quote_identifier(defn.name)} "
        f"{defn.timing} {defn.event} ON {_quote_identifier(defn.table)}\n"
        f"BEGIN\n{defn.body}\nEND"
    )


_TIMING_EVENT_RE = re.compile(
    r"\b(BEFORE|AFTER)\s+(INSERT|UPDATE|DELETE)\b",
    re.IGNORECASE,
)
_BODY_RE = re.compile(r"\bBEGIN\b(.*)\bEND\b", re.IGNORECASE | re.DOTALL)


def _sql_to_trigger_def(name: str, tbl_name: str, sql: str) -> TriggerDef:
    """Parse a ``CREATE TRIGGER`` SQL string back to a :class:`~sql_backend.schema.TriggerDef`.

    Extracts ``timing`` (BEFORE/AFTER), ``event`` (INSERT/UPDATE/DELETE), and
    ``body`` (the text between ``BEGIN`` and the final ``END``).

    Raises :class:`ValueError` if the SQL cannot be parsed.
    """
    m = _TIMING_EVENT_RE.search(sql)
    if m is None:
        raise ValueError(f"cannot parse trigger timing/event from: {sql!r}")
    timing = m.group(1).upper()
    event = m.group(2).upper()

    body_m = _BODY_RE.search(sql)
    body = body_m.group(1).strip() if body_m else ""

    return TriggerDef(
        name=name,
        table=tbl_name,
        timing=timing,  # type: ignore[arg-type]
        event=event,  # type: ignore[arg-type]
        body=body,
    )


# ---------------------------------------------------------------------------
# Row encode / decode helpers
# ---------------------------------------------------------------------------


def _is_ipk(col: ColumnDef) -> bool:
    """Return True if *col* is an INTEGER PRIMARY KEY (the SQLite rowid alias).

    In SQLite a column declared ``INTEGER PRIMARY KEY`` (the type must be
    exactly "INTEGER" or "INT", case-insensitive) is an alias for the rowid.
    Its value is stored as the B-tree cell key, **not** in the record payload.
    """
    return col.primary_key and col.type_name.upper() in ("INTEGER", "INT")


def _encode_row(rowid: int, row: Row, columns: list[ColumnDef]) -> bytes:
    """Encode *row* as a SQLite record payload.

    Columns are encoded in declaration order. For an INTEGER PRIMARY KEY
    column (a rowid alias), we write a NULL value into the payload — this is
    exactly what the real ``sqlite3`` library does.  The B-tree cell key
    carries the actual integer; the NULL in the payload is a well-known
    convention that lets both the official library and this backend decode
    the row consistently.  Absent non-IPK columns are treated as SQL NULL.

    Args:
        rowid: The B-tree row key for this row.
        row:   Mapping of column name → value.
        columns: Column definitions in declaration order.

    Returns:
        Raw record bytes suitable for passing to :meth:`~storage_sqlite.btree.BTree.insert`.
    """
    values: list[object] = []
    for col in columns:
        if _is_ipk(col):
            # Store NULL as a placeholder, exactly as the real sqlite3 does.
            # The true value is the B-tree rowid (cell key), not this slot.
            values.append(None)
        else:
            values.append(row.get(col.name))  # absent → None (SQL NULL)
    return _record.encode(values)


def _decode_row(rowid: int, payload: bytes, columns: list[ColumnDef]) -> Row:
    """Decode a raw record payload back into a :data:`~sql_backend.Row` dict.

    The inverse of :func:`_encode_row`. For INTEGER PRIMARY KEY columns the
    payload contains a NULL slot (a placeholder written there both by this
    backend and by the real ``sqlite3`` library).  We consume that NULL slot
    but discard it, injecting *rowid* as the actual column value instead.
    This gives us byte-compatibility with files produced by both backends.

    Args:
        rowid:   The B-tree cell key for this row.
        payload: Raw record bytes as returned by a B-tree scan.
        columns: Column definitions in declaration order (same order as encoding).

    Returns:
        A dict mapping column name → decoded SqlValue.
    """
    decoded_values, _ = _record.decode(payload)
    result: Row = {}
    payload_idx = 0
    for col in columns:
        if _is_ipk(col):
            result[col.name] = rowid  # inject rowid; consume (and discard) the NULL slot
            payload_idx += 1
        else:
            if payload_idx < len(decoded_values):
                result[col.name] = decoded_values[payload_idx]
            else:
                # Column was added via ALTER TABLE ADD COLUMN after this row was
                # written. Return the declared default, or NULL if there is none.
                result[col.name] = col.default if col.has_default() else None  # type: ignore[assignment]
            payload_idx += 1
    return result


# ---------------------------------------------------------------------------
# Rowid helpers
# ---------------------------------------------------------------------------


def _find_max_rowid(tree: BTree) -> int:
    """Return the maximum rowid currently stored in *tree*, or 0 if empty.

    Performs a full scan (O(n)). For v1 this is acceptable; a future
    optimisation can walk to the rightmost leaf in O(log n).
    """
    max_rid = 0
    for rowid, _ in tree.scan():
        if rowid > max_rid:
            max_rid = rowid
    return max_rid


def _choose_rowid(row: Row, columns: list[ColumnDef], tree: BTree) -> int:
    """Determine the rowid to use when inserting *row*.

    Rules (mirroring SQLite's behaviour):

    1. If the table has an INTEGER PRIMARY KEY column AND the row supplies a
       non-NULL value for it, use that value as the rowid.
    2. Otherwise (no IPK, or IPK column is absent/NULL in this row), assign
       ``max_existing_rowid + 1``.
    """
    for col in columns:
        if _is_ipk(col):
            val = row.get(col.name)
            if val is not None and isinstance(val, int):
                return val
            # IPK absent or NULL → fall through to auto-assign.
            break

    return _find_max_rowid(tree) + 1


# ---------------------------------------------------------------------------
# Constraint checkers
# ---------------------------------------------------------------------------


def _apply_defaults(row: Row, columns: list[ColumnDef]) -> Row:
    """Return a copy of *row* with missing columns filled from their defaults.

    Columns absent from *row* that have a DEFAULT clause get the default value.
    Columns absent with no default are set to ``None`` (SQL NULL), so that
    subsequent NOT NULL checks can fire correctly.
    """
    out: Row = dict(row)
    for col in columns:
        if col.name not in out:
            if col.has_default():
                # col.default is ColumnDefault = SqlValue | _NoDefault.
                # has_default() ruled out the sentinel so this cast is safe.
                out[col.name] = col.default  # type: ignore[assignment]
            else:
                out[col.name] = None  # absent + no default → NULL
    return out


def _check_not_null(table: str, row: Row, columns: list[ColumnDef]) -> None:
    """Raise :class:`~sql_backend.ConstraintViolation` if a NOT NULL column
    contains NULL in *row*.

    PRIMARY KEY implies NOT NULL (via :meth:`~sql_backend.ColumnDef.effective_not_null`).
    """
    for col in columns:
        if col.effective_not_null() and row.get(col.name) is None:
            raise ConstraintViolation(
                table=table,
                column=col.name,
                message=f"NOT NULL constraint failed: {table}.{col.name}",
            )


def _check_unique(
    table: str,
    row: Row,
    columns: list[ColumnDef],
    tree: BTree,
    tree_columns: list[ColumnDef],
    ignore_rowid: int | None = None,
) -> None:
    """Raise :class:`~sql_backend.ConstraintViolation` if a UNIQUE column
    already contains the value being inserted/updated.

    NULL values never conflict (SQL semantics). *ignore_rowid* is the rowid
    of the row being updated so it is not compared against itself.

    This is O(n) per unique column. Acceptable for v1; future versions can
    add in-memory uniqueness indexes.
    """
    unique_cols = [c for c in columns if c.effective_unique()]
    if not unique_cols:
        return

    new_vals = {col.name: row.get(col.name) for col in unique_cols}
    # Optimisation: if none of the unique-column values are non-NULL we can
    # skip the scan entirely.
    if all(v is None for v in new_vals.values()):
        return

    for existing_rowid, payload in tree.scan():
        if existing_rowid == ignore_rowid:
            continue
        existing_row = _decode_row(existing_rowid, payload, tree_columns)
        for col in unique_cols:
            new_val = new_vals[col.name]
            if new_val is None:
                continue  # NULL never conflicts
            if existing_row.get(col.name) == new_val:
                label = "PRIMARY KEY" if col.primary_key else "UNIQUE"
                raise ConstraintViolation(
                    table=table,
                    column=col.name,
                    message=f"{label} constraint failed: {table}.{col.name}",
                )


# ---------------------------------------------------------------------------
# BTreeCursor
# ---------------------------------------------------------------------------


class _BTreeCursor:
    """A :class:`~sql_backend.Cursor` backed by a B-tree scan.

    Satisfies both :class:`~sql_backend.RowIterator` (for use as a plain
    scan return value) and :class:`~sql_backend.Cursor` (for positioned
    ``UPDATE``/``DELETE``).

    The cursor reads B-tree cells lazily through a Python generator. Each
    ``next()`` call advances the generator and decodes one record.

    Row identity is the B-tree rowid (:attr:`_current_rowid`). The backend
    uses this rowid to locate and mutate the correct cell.
    """

    __slots__ = (
        "_columns",
        "_current_row",
        "_current_rowid",
        "_gen",
        "_closed",
    )

    def __init__(self, tree: BTree, columns: list[ColumnDef]) -> None:
        self._columns = columns
        # tree.scan() yields (rowid, payload) pairs in ascending rowid order.
        self._gen = tree.scan()
        self._current_rowid: int | None = None
        self._current_row: Row | None = None
        self._closed = False

    def next(self) -> Row | None:
        """Advance to the next row; return it as a dict, or ``None`` at end."""
        if self._closed:
            return None
        try:
            rowid, payload = next(self._gen)
        except StopIteration:
            self._current_rowid = None
            self._current_row = None
            return None
        self._current_rowid = rowid
        self._current_row = _decode_row(rowid, payload, self._columns)
        return dict(self._current_row)

    def current_row(self) -> Row | None:
        """Return the most recent row returned by :meth:`next`, or ``None``."""
        if self._current_row is None:
            return None
        return dict(self._current_row)

    def close(self) -> None:
        """Release the cursor. Safe to call multiple times."""
        self._closed = True
        self._current_rowid = None
        self._current_row = None


# ---------------------------------------------------------------------------
# SqliteFileBackend
# ---------------------------------------------------------------------------


class SqliteFileBackend(Backend):
    """A :class:`~sql_backend.Backend` that reads and writes real SQLite files.

    This is the file-backed sibling of :class:`~sql_backend.InMemoryBackend`.
    It implements the same interface but persists data to a SQLite-compatible
    ``.db`` file. Files produced by this backend are readable by the
    ``sqlite3`` command-line tool, and files produced by ``sqlite3`` are
    readable by this backend (for the v1 subset of features).

    Parameters
    ----------
    path:
        Filesystem path to the ``.db`` file. The file is created if it does
        not exist. Pass ``":memory:"`` to get an :class:`~sql_backend.InMemoryBackend`
        — that routing is done by the mini-sqlite facade; this class does not
        handle ``":memory:"`` itself.

    Examples
    --------
    ::

        with SqliteFileBackend("app.db") as backend:
            backend.create_table("users", [
                ColumnDef(name="id", type_name="INTEGER", primary_key=True),
                ColumnDef(name="name", type_name="TEXT", not_null=True),
            ], if_not_exists=False)
            backend.insert("users", {"id": 1, "name": "Alice"})
            h = backend.begin_transaction()
            backend.commit(h)  # flush to disk
    """

    def __init__(self, path: str) -> None:
        abs_path = os.path.abspath(path)
        if os.path.exists(abs_path):
            self._pager = Pager.open(abs_path)
        else:
            self._pager = Pager.create(abs_path)
            # Write the initial page-1 (database header + empty sqlite_schema
            # leaf) and commit immediately so the file is valid on disk before
            # any DML starts.
            initialize_new_database(self._pager)
            self._pager.commit()

        self._freelist = Freelist(self._pager)
        self._schema = Schema(self._pager, freelist=self._freelist)

        # Transaction tracking.
        self._next_handle: int = 1
        self._active_handle: int | None = None

        # Savepoint stack: list of (name, dirty_snapshot, size_pages_snapshot).
        self._savepoint_stack: list[tuple[str, dict[int, bytes], int]] = []

    # ── Context manager ──────────────────────────────────────────────────────

    def __enter__(self) -> SqliteFileBackend:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        self.close()

    def close(self) -> None:
        """Close the backend, rolling back any uncommitted transaction."""
        if self._active_handle is not None:
            self._pager.rollback()
            self._active_handle = None
        self._pager.close()

    # ── Schema helpers ───────────────────────────────────────────────────────

    def _require_table(self, table: str) -> tuple[int, list[ColumnDef]]:
        """Return ``(rootpage, columns)`` for *table*, raising
        :class:`~sql_backend.TableNotFound` if the table does not exist.
        """
        result = self._schema.find_table(table)
        if result is None:
            raise TableNotFound(table=table)
        _, rootpage, sql = result
        try:
            columns = _sql_to_columns(sql)
        except (ValueError, IndexError) as exc:
            raise TableNotFound(table=table) from exc
        return rootpage, columns

    def _open_tree(self, rootpage: int) -> BTree:
        """Open a B-tree for *rootpage* with the freelist attached."""
        return BTree.open(self._pager, rootpage, freelist=self._freelist)

    # ── Backend interface: Schema ─────────────────────────────────────────────

    def tables(self) -> list[str]:
        """Return the names of all user tables in insertion order."""
        return self._schema.list_tables()

    def columns(self, table: str) -> list[ColumnDef]:
        """Return the columns of *table* in declaration order.

        Raises :class:`~sql_backend.TableNotFound` if *table* is unknown.
        """
        _, cols = self._require_table(table)
        return cols

    # ── Backend interface: Read ───────────────────────────────────────────────

    def scan(self, table: str) -> RowIterator:
        """Return a row iterator over all rows in *table*.

        Raises :class:`~sql_backend.TableNotFound` if *table* is unknown.
        Rows are yielded in ascending rowid order (i.e. insertion order for
        tables without an explicit rowid reuse).
        """
        rootpage, columns = self._require_table(table)
        tree = self._open_tree(rootpage)
        return _BTreeCursor(tree, columns)

    def _open_cursor(self, table: str) -> _BTreeCursor:
        """Return a :class:`_BTreeCursor` for *table*.

        Not part of the public Backend interface, but expected by the
        conformance test suite which needs a cursor to test positioned
        UPDATE and DELETE.
        """
        rootpage, columns = self._require_table(table)
        tree = self._open_tree(rootpage)
        return _BTreeCursor(tree, columns)

    # ── Backend interface: Write ──────────────────────────────────────────────

    def insert(self, table: str, row: Row) -> None:
        """Insert *row* into *table*.

        Applies column defaults for absent columns, enforces NOT NULL and
        UNIQUE / PRIMARY KEY constraints, then encodes and inserts the row
        into the table's B-tree.

        Raises :class:`~sql_backend.TableNotFound`, :class:`~sql_backend.ColumnNotFound`,
        or :class:`~sql_backend.ConstraintViolation`.
        """
        rootpage, columns = self._require_table(table)

        # Fill in defaults and map absent columns to NULL.
        full_row = _apply_defaults(row, columns)

        # Reject columns not in the schema.
        known = {col.name for col in columns}
        for key in full_row:
            if key not in known:
                raise ColumnNotFound(table=table, column=key)

        # Constraint checks.
        _check_not_null(table, full_row, columns)

        tree = self._open_tree(rootpage)

        # Determine the rowid and check primary-key uniqueness.
        rowid = _choose_rowid(full_row, columns, tree)

        # For non-IPK UNIQUE columns, scan for duplicates before inserting.
        non_pk_unique = [c for c in columns if c.effective_unique() and not _is_ipk(c)]
        if non_pk_unique:
            _check_unique(table, full_row, non_pk_unique, tree, columns)

        payload = _encode_row(rowid, full_row, columns)
        try:
            tree.insert(rowid, payload)
        except DuplicateRowidError:
            # The IPK column value already exists → PRIMARY KEY violation.
            pk_cols = [c for c in columns if _is_ipk(c)]
            if pk_cols:
                raise ConstraintViolation(
                    table=table,
                    column=pk_cols[0].name,
                    message=f"PRIMARY KEY constraint failed: {table}.{pk_cols[0].name}",
                ) from None
            raise  # Should not happen for non-IPK tables; propagate as-is.

    def update(
        self,
        table: str,
        cursor: object,
        assignments: dict[str, object],
    ) -> None:
        """Apply *assignments* to the row the cursor is currently on.

        Validates column names, enforces NOT NULL on the new values, then
        re-encodes the row and calls :meth:`BTree.update` at the cursor's
        current rowid.

        Raises :class:`~sql_backend.TableNotFound`, :class:`~sql_backend.ColumnNotFound`,
        :class:`~sql_backend.ConstraintViolation`, or :class:`~sql_backend.Unsupported`
        if the cursor is not a native :class:`_BTreeCursor`.
        """
        if not isinstance(cursor, _BTreeCursor):
            raise Unsupported(operation="update with non-native cursor")
        if cursor._current_rowid is None:  # noqa: SLF001
            raise Unsupported(operation="update without current row")

        rootpage, columns = self._require_table(table)

        # Validate column names before applying any change.
        col_names = {c.name for c in columns}
        for col_name in assignments:
            if col_name not in col_names:
                raise ColumnNotFound(table=table, column=col_name)

        # Build the proposed new row.
        current_row = cursor._current_row or {}  # noqa: SLF001
        proposed: Row = {**current_row, **assignments}  # type: ignore[arg-type]

        # Constraint checks on proposed row.
        _check_not_null(table, proposed, columns)

        tree = self._open_tree(rootpage)
        rowid = cursor._current_rowid  # noqa: SLF001
        new_payload = _encode_row(rowid, proposed, columns)
        tree.update(rowid, new_payload)

        # Update cursor's cached row so further reads are consistent.
        cursor._current_row = proposed  # noqa: SLF001

    def delete(self, table: str, cursor: object) -> None:
        """Delete the row the cursor is currently on.

        After deletion the cursor's current row is cleared (``current_row()``
        returns ``None``). The cursor's generator is still live so ``next()``
        will advance to the row that USED TO follow the deleted one.

        Raises :class:`~sql_backend.Unsupported` if the cursor is not a native
        :class:`_BTreeCursor`.
        """
        if not isinstance(cursor, _BTreeCursor):
            raise Unsupported(operation="delete with non-native cursor")
        if cursor._current_rowid is None:  # noqa: SLF001
            raise Unsupported(operation="delete without current row")

        rootpage, _ = self._require_table(table)
        tree = self._open_tree(rootpage)
        rowid = cursor._current_rowid  # noqa: SLF001
        tree.delete(rowid)

        # Clear cursor position — the row no longer exists.
        cursor._current_rowid = None  # noqa: SLF001
        cursor._current_row = None  # noqa: SLF001

    # ── Backend interface: DDL ────────────────────────────────────────────────

    def create_table(
        self,
        table: str,
        columns: list[ColumnDef],
        if_not_exists: bool,
    ) -> None:
        """Create a new table.

        Generates the ``CREATE TABLE`` SQL, stores it in ``sqlite_schema``,
        and allocates a fresh root page for the new B-tree.

        If *if_not_exists* is ``True`` and the table already exists, this is a
        no-op. Otherwise raises :class:`~sql_backend.TableAlreadyExists`.
        """
        if self._schema.find_table(table) is not None:
            if if_not_exists:
                return
            raise TableAlreadyExists(table=table)

        sql = _columns_to_sql(table, columns)
        self._schema.create_table(table, sql)

    def drop_table(self, table: str, if_exists: bool) -> None:
        """Drop *table*, freeing all its pages.

        Raises :class:`~sql_backend.TableNotFound` if the table does not
        exist and *if_exists* is ``False``.
        """
        try:
            self._schema.drop_table(table)
        except SchemaError:
            if if_exists:
                return
            raise TableNotFound(table=table) from None

    def add_column(self, table: str, column: ColumnDef) -> None:
        """Add a new column to an existing table (ALTER TABLE … ADD COLUMN).

        Rewrites the ``CREATE TABLE`` SQL stored in ``sqlite_schema`` to
        include the new column.  Existing rows are NOT modified on disk —
        :func:`_decode_row` returns the column default (or NULL) for columns
        that are absent from old records, mirroring SQLite's own behaviour for
        ALTER TABLE ADD COLUMN without a data-file rewrite.

        Raises :class:`~sql_backend.TableNotFound` if *table* does not exist.
        Raises :class:`~sql_backend.ColumnAlreadyExists` if a column with
        that name already exists in the table.
        """
        _, columns = self._require_table(table)
        if any(c.name == column.name for c in columns):
            raise ColumnAlreadyExists(table=table, column=column.name)
        new_sql = _columns_to_sql(table, columns + [column])
        self._schema.update_table_sql(table, new_sql)

    # ── Backend interface: Transactions ───────────────────────────────────────

    def begin_transaction(self) -> TransactionHandle:
        """Begin a transaction and return an opaque handle.

        The pager already accumulates all writes in its dirty-page table, so
        no additional action is needed at the storage level. We just record
        the handle so :meth:`commit` and :meth:`rollback` can validate it.

        Raises :class:`~sql_backend.Unsupported` if a transaction is already
        open (nested transactions are not supported).
        """
        if self._active_handle is not None:
            raise Unsupported(operation="nested transactions")
        handle = TransactionHandle(self._next_handle)
        self._next_handle += 1
        self._active_handle = int(handle)
        return handle

    def commit(self, handle: TransactionHandle) -> None:
        """Commit the transaction identified by *handle*.

        Before flushing, updates the three header fields that the real
        SQLite uses to validate the on-disk database size:

        * **file_change_counter** (offset 24): incremented by 1 on every
          commit so other readers can detect schema/data changes.
        * **database_size_pages** (offset 28): set to the actual number of
          pages currently allocated (``pager.size_pages``).  If this field
          disagrees with the file size, the real sqlite3 reports
          ``malformed database schema`` because page references stored in
          ``sqlite_schema`` appear to be out of range.
        * **version_valid_for** (offset 92): set to the new
          ``file_change_counter`` value so that sqlite3 knows the
          ``database_size_pages`` field is fresh.

        Then flushes all dirty pages to disk via ``pager.commit()``.
        """
        self._require_active(handle)
        self._update_commit_header()
        self._pager.commit()
        self._active_handle = None

    def _update_commit_header(self) -> None:
        """Stamp the page-1 database header with current commit metadata.

        Called from :meth:`commit` before ``pager.commit()``.  Only the
        three fields at offsets 24, 28, and 92 are modified; all other
        bytes (B-tree data at offset 100+, schema cookie at offset 40,
        etc.) are preserved.
        """
        buf = bytearray(self._pager.read(1))
        (counter,) = struct.unpack_from(">I", buf, 24)  # file_change_counter
        new_counter = (counter + 1) & 0xFFFFFFFF
        struct.pack_into(">I", buf, 24, new_counter)               # file_change_counter
        struct.pack_into(">I", buf, 28, self._pager.size_pages)    # database_size_pages
        struct.pack_into(">I", buf, 92, new_counter)               # version_valid_for
        self._pager.write(1, bytes(buf))

    def rollback(self, handle: TransactionHandle) -> None:
        """Roll back the transaction identified by *handle*.

        Discards all dirty pages, restoring the file to its state at the
        time of the last ``commit()``.
        """
        self._require_active(handle)
        self._pager.rollback()
        self._active_handle = None
        self._savepoint_stack.clear()
        # Reattach the schema object so it reads fresh pages after rollback.
        self._schema = Schema(self._pager, freelist=self._freelist)

    def current_transaction(self) -> TransactionHandle | None:
        """Return the active transaction handle, or ``None`` if none is open.

        Allows multi-statement transactions that span separate
        :func:`~sql_vm.vm.execute` calls to retrieve the handle issued by
        an earlier ``BeginTransaction`` instruction without storing it
        externally.
        """
        if self._active_handle is None:
            return None
        return TransactionHandle(self._active_handle)

    # ── Backend interface: Savepoints ─────────────────────────────────────────

    def create_savepoint(self, name: str) -> None:
        """Push a snapshot of the pager's dirty-page table.

        Savepoints in the file backend are implemented by deep-copying the
        pager's in-memory dirty-page dict and recording the current logical
        page count.  Rolling back to a savepoint restores both, effectively
        undoing all page writes that happened after the savepoint was created.

        No disk I/O occurs here — the snapshot lives entirely in memory until
        the outer transaction is committed or rolled back.

        Multiple savepoints with the same name are allowed; they stack
        independently, as in SQLite.
        """
        dirty_snap = copy.deepcopy(self._pager._dirty)  # noqa: SLF001
        size_snap = self._pager._size_pages  # noqa: SLF001
        self._savepoint_stack.append((name, dirty_snap, size_snap))

    def release_savepoint(self, name: str) -> None:
        """Release (destroy) the named savepoint and all savepoints after it.

        The current data state is unchanged — this is a "partial commit" up
        to the release point.  Raises :class:`~sql_backend.Unsupported` if no
        savepoint with *name* exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"RELEASE {name!r}: no such savepoint")
        del self._savepoint_stack[idx:]

    def rollback_to_savepoint(self, name: str) -> None:
        """Restore the pager dirty-page state to when *name* was created.

        Replaces the pager's dirty-page dict and logical size with the
        snapshot taken at savepoint creation, then re-attaches the schema
        object so it reads the restored pages.  Savepoints created after
        *name* are destroyed; *name* itself is kept alive so the caller may
        roll back to it again or release it later.

        Raises :class:`~sql_backend.Unsupported` if no savepoint with *name*
        exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"ROLLBACK TO {name!r}: no such savepoint")
        _sp_name, dirty_snap, size_snap = self._savepoint_stack[idx]
        self._pager._dirty = copy.deepcopy(dirty_snap)  # noqa: SLF001
        self._pager._size_pages = size_snap  # noqa: SLF001
        self._schema = Schema(self._pager, freelist=self._freelist)
        del self._savepoint_stack[idx + 1:]

    def _find_savepoint(self, name: str) -> int | None:
        """Return the index of the *last* savepoint named *name*, or ``None``."""
        for i in range(len(self._savepoint_stack) - 1, -1, -1):
            if self._savepoint_stack[i][0] == name:
                return i
        return None

    # ── Backend interface: Triggers ───────────────────────────────────────────

    def create_trigger(self, defn: TriggerDef) -> None:
        """Store a trigger definition in ``sqlite_schema``.

        Serialises *defn* to a ``CREATE TRIGGER`` SQL string and inserts a
        ``type = 'trigger'`` row into ``sqlite_schema`` with ``rootpage = 0``
        (the standard convention for trigger rows).

        Raises :class:`~sql_backend.TriggerAlreadyExists` if a trigger with
        ``defn.name`` already exists.
        """
        if self._schema.find_trigger(defn.name) is not None:
            raise TriggerAlreadyExists(name=defn.name)
        sql = _trigger_to_sql(defn)
        self._schema.create_trigger(defn.name, defn.table, sql)

    def drop_trigger(self, name: str, if_exists: bool = False) -> None:
        """Remove a trigger definition by name.

        Deletes the ``sqlite_schema`` row and bumps the schema cookie.
        When ``if_exists=True`` and the trigger does not exist, this is a
        silent no-op.  Otherwise raises :class:`~sql_backend.TriggerNotFound`.
        """
        try:
            self._schema.drop_trigger(name)
        except SchemaError:
            if if_exists:
                return
            raise TriggerNotFound(name=name) from None

    def list_triggers(self, table: str) -> list[TriggerDef]:
        """Return all triggers for *table* in creation order.

        Reads ``sqlite_schema`` for ``type = 'trigger'`` rows whose
        ``tbl_name`` matches *table*, parses the ``CREATE TRIGGER`` SQL to
        recover the timing, event, and body, and returns a list of
        :class:`~sql_backend.schema.TriggerDef` objects.

        Returns an empty list when no triggers exist for *table*.
        """
        rows = self._schema.list_triggers(table)
        result: list[TriggerDef] = []
        for name, tbl_name, sql in rows:
            if sql is None:
                continue
            with contextlib.suppress(ValueError):
                result.append(_sql_to_trigger_def(name, tbl_name, sql))
        return result

    # ── Backend interface: Indexes ────────────────────────────────────────────

    def create_index(self, index: IndexDef) -> None:
        """Create a B-tree index and backfill it from the existing table rows.

        Steps:

        1. Reject with :class:`~sql_backend.IndexAlreadyExists` if an index
           with ``index.name`` already exists.
        2. Reject with :class:`~sql_backend.TableNotFound` if ``index.table``
           does not exist.
        3. Reject with :class:`~sql_backend.ColumnNotFound` if any column in
           ``index.columns`` is not in ``index.table``.
        4. Generate the ``CREATE INDEX`` SQL and call
           :meth:`~storage_sqlite.schema.Schema.create_index` to allocate an
           empty index B-tree root page and insert the ``sqlite_schema`` row.
        5. Backfill: scan every existing row in the table and insert its key
           columns + rowid into the new index tree.

        The backfill is committed immediately (auto-commit) so that the index
        is durably written before the method returns.

        Raises
        ------
        IndexAlreadyExists
            When ``index.name`` is already in ``sqlite_schema``.
        TableNotFound
            When ``index.table`` is not a known table.
        ColumnNotFound
            When any element of ``index.columns`` is not a column of
            ``index.table``.
        """
        # 1. Duplicate-name check.
        if self._schema.find_index(index.name) is not None:
            raise IndexAlreadyExists(index=index.name)

        # 2. Table existence + column schema.
        rootpage, columns = self._require_table(index.table)
        col_names = {c.name for c in columns}

        # 3. Column validation.
        for col in index.columns:
            if col not in col_names:
                raise ColumnNotFound(table=index.table, column=col)

        # 4. Allocate index storage and write the schema row.
        sql = _columns_to_index_sql(index.name, index.table, index.columns)
        idx_rootpage = self._schema.create_index(index.name, index.table, sql)

        # 5. Backfill existing rows.
        table_tree = self._open_tree(rootpage)
        idx_tree = IndexTree.open(self._pager, idx_rootpage, freelist=self._freelist)
        for rowid, payload in table_tree.scan():
            row = _decode_row(rowid, payload, columns)
            key_vals: list[SqlValue] = [row.get(col) for col in index.columns]  # type: ignore[misc]
            idx_tree.insert(key_vals, rowid)

        # Commit the backfill so it is durable.
        self._update_commit_header()
        self._pager.commit()

    def drop_index(self, name: str, *, if_exists: bool = False) -> None:
        """Drop an index by name.

        Frees all pages used by the index B-tree and removes the
        ``sqlite_schema`` row.  Bumps the schema cookie.

        Parameters
        ----------
        name:
            The index name to drop.
        if_exists:
            When ``True``, silently succeed if the index does not exist.
            When ``False`` (default), raise :class:`~sql_backend.IndexNotFound`.
        """
        try:
            self._schema.drop_index(name)
        except SchemaError:
            if if_exists:
                return
            raise IndexNotFound(index=name) from None

        # Commit the structural change.
        self._update_commit_header()
        self._pager.commit()

    def list_indexes(self, table: str | None = None) -> list[IndexDef]:
        """Return all indexes, optionally filtered to one table.

        Reads ``sqlite_schema`` for ``type = 'index'`` rows, parses the
        ``CREATE INDEX`` SQL to recover column names, and returns a list
        of :class:`~sql_backend.IndexDef` descriptors in creation order.

        The ``auto`` flag is set when the index name starts with the
        ``auto_`` prefix (the convention used by the IndexAdvisor).

        Parameters
        ----------
        table:
            When provided, only indexes on this table are returned.
        """
        rows = self._schema.list_indexes(table)
        result: list[IndexDef] = []
        for name, tbl_name, _, sql in rows:
            columns = _parse_index_columns(sql) if sql else []
            auto = name.startswith("auto_")
            result.append(
                IndexDef(name=name, table=tbl_name, columns=columns, unique=False, auto=auto)
            )
        return result

    def scan_index(
        self,
        index_name: str,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[int]:
        """Yield rowids from the named index within the given key range.

        Opens the index B-tree and delegates to
        :meth:`~storage_sqlite.index_tree.IndexTree.range_scan`.  Yields
        the rowid of each matching index entry in ascending key order.

        Parameters
        ----------
        index_name:
            Name of the index to scan.
        lo:
            Lower bound key values (``None`` = unbounded).
        hi:
            Upper bound key values (``None`` = unbounded).
        lo_inclusive:
            Include entries whose key equals *lo* (default ``True``).
        hi_inclusive:
            Include entries whose key equals *hi* (default ``True``).

        Yields
        ------
        int
            Table rowids in ascending key order.

        Raises
        ------
        IndexNotFound
            When *index_name* is not in ``sqlite_schema``.
        """
        result = self._schema.find_index(index_name)
        if result is None:
            raise IndexNotFound(index=index_name)
        _, idx_rootpage, _ = result
        idx_tree = IndexTree.open(self._pager, idx_rootpage, freelist=self._freelist)
        for _, rowid in idx_tree.range_scan(
            lo, hi, lo_inclusive=lo_inclusive, hi_inclusive=hi_inclusive
        ):
            yield rowid

    def scan_by_rowids(self, table: str, rowids: list[int]) -> RowIterator:
        """Fetch rows by their B-tree integer rowids.

        Each rowid is a signed integer key in the table's B-tree.
        :meth:`scan_index` yields these rowids; this method does a point lookup
        per rowid using :meth:`~storage_sqlite.btree.BTree.find`.

        Rows whose rowid is not found (deleted between index scan and row fetch)
        are silently skipped — this mirrors SQLite's own behaviour for stale
        index entries.
        """
        rootpage, columns = self._require_table(table)
        tree = self._open_tree(rootpage)
        rows = []
        for rowid in rowids:
            payload = tree.find(rowid)
            if payload is not None:
                rows.append(_decode_row(rowid, payload, columns))
        return ListRowIterator(rows)

    # ── Internal helpers ─────────────────────────────────────────────────────

    def _require_active(self, handle: TransactionHandle) -> None:
        if self._active_handle is None:
            raise Unsupported(operation="no active transaction")
        if int(handle) != self._active_handle:
            raise Unsupported(operation="stale transaction handle")
