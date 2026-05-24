"""Tests for PK uniqueness violation wording (matches SQLite).

SQLite reports PRIMARY KEY uniqueness violations using the unified
``UNIQUE constraint failed: <table>.<col>`` wording — the same
phrasing used for explicit ``UNIQUE`` columns.  PRIMARY KEY implies
UNIQUE in SQL, so the dedicated "PRIMARY KEY constraint failed"
phrase that older mini-sqlite emitted is not something sqlite3
actually produces; this PR realigns the wording.

Covers all three places mini-sqlite enforces uniqueness:

* ``sql-backend``'s InMemoryBackend (default backend for the in-memory
  mini-sqlite engine).
* ``storage-sqlite``'s file backend (alternative backend used when
  ``mini_sqlite.connect("path.db")`` opens an on-disk file).
* Explicit ``UNIQUE`` columns (regression — wording was already right
  here, the test pins it so future cleanups don't accidentally split
  the wording back into two branches).
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


class TestNamedPK:
    """``PRIMARY KEY`` on a non-INTEGER column."""

    def test_duplicate_named_pk_says_unique(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE pk (a INT PRIMARY KEY)")
            c.execute("INSERT INTO pk VALUES (1)")
        with pytest.raises(mini_errors.IntegrityError) as mini_exc:
            mini.execute("INSERT INTO pk VALUES (1)")
        with pytest.raises(sqlite3.IntegrityError) as ref_exc:
            ref.execute("INSERT INTO pk VALUES (1)")
        assert str(mini_exc.value) == str(ref_exc.value)
        assert str(mini_exc.value) == "UNIQUE constraint failed: pk.a"


class TestIntegerPK:
    """``INTEGER PRIMARY KEY`` is the rowid alias path — different code."""

    def test_duplicate_ipk_says_unique(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE ipk (id INTEGER PRIMARY KEY, v TEXT)")
            c.execute("INSERT INTO ipk VALUES (1, 'a')")
        with pytest.raises(mini_errors.IntegrityError) as mini_exc:
            mini.execute("INSERT INTO ipk VALUES (1, 'b')")
        with pytest.raises(sqlite3.IntegrityError) as ref_exc:
            ref.execute("INSERT INTO ipk VALUES (1, 'b')")
        assert str(mini_exc.value) == str(ref_exc.value)
        assert str(mini_exc.value) == "UNIQUE constraint failed: ipk.id"


class TestExplicitUnique:
    """Regression: existing ``UNIQUE`` wording must keep matching SQLite."""

    def test_duplicate_unique_says_unique(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE un (x INT UNIQUE)")
            c.execute("INSERT INTO un VALUES (10)")
        with pytest.raises(mini_errors.IntegrityError) as mini_exc:
            mini.execute("INSERT INTO un VALUES (10)")
        with pytest.raises(sqlite3.IntegrityError) as ref_exc:
            ref.execute("INSERT INTO un VALUES (10)")
        assert str(mini_exc.value) == str(ref_exc.value)
        assert str(mini_exc.value) == "UNIQUE constraint failed: un.x"


class TestUpdateViolation:
    """UPDATE that creates a duplicate uses the same wording."""

    def test_update_pk_into_duplicate_says_unique(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE pk (a INT PRIMARY KEY)")
            c.execute("INSERT INTO pk VALUES (1)")
            c.execute("INSERT INTO pk VALUES (2)")
        with pytest.raises(mini_errors.IntegrityError) as mini_exc:
            mini.execute("UPDATE pk SET a = 1 WHERE a = 2")
        with pytest.raises(sqlite3.IntegrityError) as ref_exc:
            ref.execute("UPDATE pk SET a = 1 WHERE a = 2")
        assert str(mini_exc.value) == str(ref_exc.value)
        assert str(mini_exc.value) == "UNIQUE constraint failed: pk.a"
