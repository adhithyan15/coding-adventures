"""Tests for RowIterator and ListCursor."""

from __future__ import annotations

from sql_backend.row import ListCursor, ListRowIterator


class TestListRowIterator:
    def test_iterates_in_order(self) -> None:
        rows = [{"id": 1}, {"id": 2}, {"id": 3}]
        it = ListRowIterator(rows)
        assert it.next() == {"id": 1}
        assert it.next() == {"id": 2}
        assert it.next() == {"id": 3}
        assert it.next() is None
        it.close()

    def test_close_after_exhaustion_is_safe(self) -> None:
        it = ListRowIterator([])
        assert it.next() is None
        it.close()
        # Close is idempotent.
        it.close()
        assert it.next() is None

    def test_close_before_exhaustion_stops_iteration(self) -> None:
        it = ListRowIterator([{"id": 1}, {"id": 2}])
        it.close()
        assert it.next() is None

    def test_returns_copy_not_reference(self) -> None:
        # Mutating the returned row must not corrupt the underlying storage.
        rows = [{"id": 1, "name": "alice"}]
        it = ListRowIterator(rows)
        got = it.next()
        assert got is not None
        got["name"] = "MUTATED"
        # Re-read — underlying row should be untouched.
        it2 = ListRowIterator(rows)
        again = it2.next()
        assert again == {"id": 1, "name": "alice"}


class TestListCursor:
    def test_current_row_before_next_is_none(self) -> None:
        cur = ListCursor([{"id": 1}])
        assert cur.current_row() is None

    def test_current_row_tracks_next(self) -> None:
        cur = ListCursor([{"id": 1}, {"id": 2}])
        cur.next()
        assert cur.current_row() == {"id": 1}
        cur.next()
        assert cur.current_row() == {"id": 2}

    def test_exhaustion(self) -> None:
        cur = ListCursor([{"id": 1}])
        cur.next()
        assert cur.next() is None
        assert cur.current_row() is None

    def test_current_index(self) -> None:
        cur = ListCursor([{"id": 1}, {"id": 2}])
        cur.next()
        assert cur.current_index() == 0
        cur.next()
        assert cur.current_index() == 1

    def test_close_stops_iteration(self) -> None:
        cur = ListCursor([{"id": 1}])
        cur.close()
        assert cur.next() is None
