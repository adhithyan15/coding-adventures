"""PEP 249 module-level smoke tests."""

import mini_sqlite


def test_apilevel():
    assert mini_sqlite.apilevel == "2.0"


def test_threadsafety():
    assert mini_sqlite.threadsafety == 1


def test_paramstyle():
    assert mini_sqlite.paramstyle == "qmark"


def test_exception_hierarchy():
    # PEP 249 requires exactly this tree.
    assert issubclass(mini_sqlite.Warning, Exception)
    assert issubclass(mini_sqlite.Error, Exception)
    assert issubclass(mini_sqlite.InterfaceError, mini_sqlite.Error)
    assert issubclass(mini_sqlite.DatabaseError, mini_sqlite.Error)
    assert issubclass(mini_sqlite.DataError, mini_sqlite.DatabaseError)
    assert issubclass(mini_sqlite.OperationalError, mini_sqlite.DatabaseError)
    assert issubclass(mini_sqlite.IntegrityError, mini_sqlite.DatabaseError)
    assert issubclass(mini_sqlite.InternalError, mini_sqlite.DatabaseError)
    assert issubclass(mini_sqlite.ProgrammingError, mini_sqlite.DatabaseError)
    assert issubclass(mini_sqlite.NotSupportedError, mini_sqlite.DatabaseError)


def test_connect_memory_returns_connection():
    conn = mini_sqlite.connect(":memory:")
    assert isinstance(conn, mini_sqlite.Connection)
    conn.close()


def test_connect_file_path_creates_file(tmp_path):
    """A non-:memory: path creates a real database file."""
    import os
    db_path = str(tmp_path / "test.db")
    conn = mini_sqlite.connect(db_path)
    conn.close()
    assert os.path.exists(db_path)
