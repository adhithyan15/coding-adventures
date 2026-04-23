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
        return dict(row)  # Shallow copy — protects backend state from caller mutation.

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
        return dict(self._current)  # Shallow copy.

    def current_row(self) -> Row | None:
        if self._current is None:
            return None
        return dict(self._current)  # Shallow copy.

    def current_index(self) -> int:
        """Index of the current row in the backing list.

        Exposed for InMemoryBackend's use only — not part of the public
        Cursor protocol. Backends that don't use a list-backed cursor will
        not expose this method.
        """
        return self._idx

    def close(self) -> None:
        self._closed = True
