"""Tests for honouring ``PRAGMA foreign_keys`` at INSERT/UPDATE/DELETE.

Mini-sqlite defaults FK enforcement to ON (documented deviation from
SQLite's OFF default).  ``PRAGMA foreign_keys = OFF`` disables
enforcement on a per-connection basis; ``PRAGMA foreign_keys = ON``
re-enables it.  Toggling the setting in the middle of a session
takes effect for subsequent statements.

These tests pin the toggle behaviour and verify both the child-side
(orphan INSERT) and parent-side (DELETE with referencing children)
short-circuit paths.
"""

from __future__ import annotations

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


class TestDefaultOn:
    """The default FK state is ON (mini-sqlite deviation from SQLite)."""

    def test_default_rejects_orphan(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        with pytest.raises(mini_errors.IntegrityError):
            c.execute("INSERT INTO c VALUES (1, 999)")

    def test_default_pragma_reads_one(self) -> None:
        c = mini_sqlite.connect(":memory:")
        assert c.execute("PRAGMA foreign_keys").fetchone() == (1,)


class TestPragmaOff:
    """``PRAGMA foreign_keys = OFF`` disables enforcement."""

    def test_orphan_accepted_with_fk_off(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("PRAGMA foreign_keys = OFF")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        # No exception — FK enforcement is off.
        c.execute("INSERT INTO c VALUES (1, 999)")
        assert c.execute("SELECT * FROM c").fetchall() == [(1, 999)]

    def test_parent_delete_succeeds_with_fk_off(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("PRAGMA foreign_keys = OFF")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        c.execute("INSERT INTO p VALUES (1)")
        c.execute("INSERT INTO c VALUES (1, 1)")
        # With FK on, this DELETE would be rejected because c.p_id=1
        # references p.id=1.  With FK off, it's permitted.
        c.execute("DELETE FROM p WHERE id = 1")
        assert c.execute("SELECT COUNT(*) FROM p").fetchone() == (0,)

    def test_foreign_key_check_surfaces_violations_after_off_inserts(self) -> None:
        # End-to-end story: turn FK off, insert orphans, turn FK back
        # on, ask foreign_key_check what's wrong.
        c = mini_sqlite.connect(":memory:")
        c.execute("PRAGMA foreign_keys = OFF")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        c.execute("INSERT INTO c VALUES (1, 100)")
        c.execute("INSERT INTO c VALUES (2, 200)")
        c.execute("PRAGMA foreign_keys = ON")
        rows = c.execute("PRAGMA foreign_key_check").fetchall()
        assert len(rows) == 2
        assert {r[1] for r in rows} == {1, 2}  # both child rowids
        assert all(r[0] == "c" and r[2] == "p" for r in rows)


class TestPragmaToggleMidSession:
    """Toggling FK mid-session takes effect immediately."""

    def test_re_enable_blocks_subsequent_orphan(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("PRAGMA foreign_keys = OFF")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        # First insert: OK (FK off)
        c.execute("INSERT INTO c VALUES (1, 999)")
        # Re-enable
        c.execute("PRAGMA foreign_keys = ON")
        # Second insert: rejected (FK on)
        with pytest.raises(mini_errors.IntegrityError):
            c.execute("INSERT INTO c VALUES (2, 888)")


class TestPerConnectionIsolation:
    """PRAGMA state doesn't leak across connections."""

    def test_off_on_one_connection_does_not_affect_another(self) -> None:
        c1 = mini_sqlite.connect(":memory:")
        c2 = mini_sqlite.connect(":memory:")
        c1.execute("PRAGMA foreign_keys = OFF")
        # c2 still defaults to ON
        c2.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c2.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        with pytest.raises(mini_errors.IntegrityError):
            c2.execute("INSERT INTO c VALUES (1, 999)")


class TestNullValueStillPasses:
    """NULL FK values pass regardless of the foreign_keys setting."""

    def test_null_accepted_with_fk_on(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE p (id INTEGER PRIMARY KEY)")
        c.execute(
            "CREATE TABLE c (id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        # NULL FK is always permitted (SQL standard: unknown reference
        # is not an error).
        c.execute("INSERT INTO c VALUES (1, NULL)")
        assert c.execute("SELECT * FROM c").fetchall() == [(1, None)]
