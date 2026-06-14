"""
Row and row-iteration types
===========================

A :data:`Row` is a mapping from column name to SqlValue. We use a plain
``dict`` rather than a dataclass because rows are the hot path — they are
created and thrown away by the millions during a full table scan — and
``dict`` construction is one of the fastest operations in Python.

The :class:`RowIterator` protocol is what ``scan()`` returns. It is a
deliberately minimal interface: ``next()`` for one row, ``close()`` for
resource cleanup. We define our own protocol rather than leaning on
``Iterator[Row]`` for one specific reason: **close semantics**. File- or
network-backed backends need a hook to release file handles, socket
connections, or database cursors. Python's iterator protocol has no such
hook (generators do, via ``.close()``, but only by convention).

A :class:`Cursor` is a RowIterator that additionally supports positioned
updates — it remembers which row was just returned by ``next()`` so that
``update()`` and ``delete()`` can identify it. Implementing row identity is a
backend detail; we only require that the backend's own ``update``/``delete``
methods accept this cursor and do the right thing.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from .values import SqlValue

# A row is a mapping from column name to SqlValue. dict, not dataclass —
# see module docstring.
Row = dict[str, SqlValue]


@runtime_checkable
class RowIterator(Protocol):
    """Lazy iterator over backend rows.

    The VM calls ``next()`` in a loop until it returns ``None``. Implementations
    may materialize all rows up-front (fine for an in-memory backend) or
    stream them one at a time (what a CSV or SQLite backend will do).

    ``close()`` must be safe to call multiple times and must be safe to call
    before iteration is complete — the VM calls it to abort a scan early
    (e.g. when ``LIMIT`` is reached).
    """

    def next(self) -> Row | None: ...
    def close(self) -> None: ...


@runtime_checkable
class Cursor(RowIterator, Protocol):
    """RowIterator that remembers the current row for positioned DML.

    The "current row" is the most recent one returned by ``next()`` — i.e.
    the row the VM is currently examining. Backends implement row identity
    however they like (index into a list, rowid, byte offset); the VM never
    inspects the mechanism.
    """

    def current_row(self) -> Row | None: ...


class ListRowIterator:
    """RowIterator backed by a materialized list.

    Used internally by :class:`InMemoryBackend`. Also useful in tests when
    you want a quick RowIterator over a handful of rows. We yield shallow
    copies so the caller can mutate the returned row without corrupting the
    underlying storage — rows are dicts, and dicts are mutable.
    """

    def __init__(self, rows: list[Row]) -> None:
        self._rows = rows
        self._idx = 0
        self._closed = False

    def next(self) -> Row | None:
        if self._closed or self._idx >= len(self._rows):
            return None
        row = self._rows[self._idx]
        self._idx += 1
        # Shallow copy — protects backend state from caller mutation.
        # Filter out internal implementation keys (those starting with ``\x00``,
        # e.g. the ``"\x00rowid"`` stamp) so that hidden metadata never leaks
        # into query results.  Callers that need rowid access should use the
        # ``rowid()`` method instead of reading the row dict directly.
        return {k: v for k, v in row.items() if not k.startswith("\x00")}

    def rowid(self) -> int | None:
        """Return the stable integer rowid of the last row yielded by :meth:`next`.

        Each row in the backing list carries a hidden ``"\\x00rowid"`` field
        stamped at insert time — a monotonically increasing integer starting
        at 1 (matching real-SQLite convention).  Stable rowids do not change
        when other rows are deleted.

        Returns ``None`` before the first :meth:`next` call (no row has been
        yielded yet), after the iterator is exhausted, or if the backing rows
        were created without rowid stamps (e.g. raw test fixtures).

        Calling convention::

            it = ListRowIterator(rows)   # rows stamped with _ROWID_KEY
            row = it.next()    # yields rows[0] whose rowid stamp is 1
            it.rowid()         # → 1
            row = it.next()    # yields rows[1] whose rowid stamp is 2
            it.rowid()         # → 2
        """
        if self._closed or self._idx == 0:
            return None
        # _idx was already incremented — the last yielded row is at _idx-1.
        return self._rows[self._idx - 1].get("\x00rowid")

    def close(self) -> None:
        self._closed = True


class ListCursor:
    """Cursor backed by a materialized list — tracks index for positioned DML.

    Used by :class:`InMemoryBackend` for ``update`` and ``delete``. The backend
    knows the list that backs this cursor, so it can use the cursor's index
    to mutate the underlying storage directly.
    """

    def __init__(self, rows: list[Row]) -> None:
        self._rows = rows
        self._idx = -1  # No row consumed yet.
        self._current: Row | None = None
        self._closed = False

    def next(self) -> Row | None:
        if self._closed:
            return None
        self._idx += 1
        if self._idx >= len(self._rows):
            self._current = None
            return None
        self._current = self._rows[self._idx]
        # Shallow copy with hidden keys filtered out — mirrors ListRowIterator.next().
        return {k: v for k, v in self._current.items() if not k.startswith("\x00")}

    def current_row(self) -> Row | None:
        if self._current is None:
            return None
        # Filter hidden keys so positioned DML handlers receive clean rows.
        return {k: v for k, v in self._current.items() if not k.startswith("\x00")}

    def rowid(self) -> int | None:
        """Return the stable integer rowid of the current row.

        Reads the hidden ``"\\x00rowid"`` field stamped at insert time —
        matches the convention of :meth:`ListRowIterator.rowid`.  Stable
        rowids do not change when other rows are deleted.

        Returns ``None`` before the first :meth:`next` call, after the cursor
        is exhausted, or if the backing row has no rowid stamp::

            cur = ListCursor(rows)   # rows stamped with _ROWID_KEY
            cur.next()       # advances to rows[0] with stamp 1
            cur.rowid()      # → 1
            cur.next()       # advances to rows[1] with stamp 2
            cur.rowid()      # → 2
        """
        if self._current is None:
            return None
        return self._current.get("\x00rowid")

    def current_index(self) -> int:
        """Index of the current row in the backing list.

        Exposed for InMemoryBackend's use only — not part of the public
        Cursor protocol. Backends that don't use a list-backed cursor will
        not expose this method.
        """
        return self._idx

    def close(self) -> None:
        self._closed = True
