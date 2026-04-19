"""Parameter binding — unit-level coverage for ``binding.substitute``."""

import pytest

import mini_sqlite
from mini_sqlite.binding import substitute


def test_no_placeholders():
    assert substitute("SELECT 1", ()) == "SELECT 1"


def test_int_float_bool_null():
    out = substitute("VALUES (?, ?, ?, ?, ?)", (1, 2.5, True, False, None))
    assert out == "VALUES (1, 2.5, 1, 0, NULL)"


def test_string_escaping():
    # Backslash and single-quote both need escaping.
    assert substitute("VALUES (?)", ("O'Brien",)) == "VALUES ('O\\'Brien')"
    assert substitute("VALUES (?)", ("a\\b",)) == "VALUES ('a\\\\b')"


def test_placeholders_inside_strings_are_ignored():
    # The ``?`` inside a string literal must not consume a parameter.
    assert substitute("SELECT '?' WHERE x = ?", (1,)) == "SELECT '?' WHERE x = 1"


def test_placeholders_inside_line_comments_are_ignored():
    out = substitute("SELECT 1 -- ?\nWHERE x = ?", (42,))
    assert "42" in out
    assert out.count("?") == 1  # still in the comment


def test_placeholders_inside_block_comments_are_ignored():
    out = substitute("SELECT /* ? */ ?", (99,))
    assert "99" in out
    # Block-comment preserved.
    assert "/* ? */" in out


def test_too_few_params():
    with pytest.raises(mini_sqlite.ProgrammingError):
        substitute("VALUES (?, ?)", (1,))


def test_too_many_params():
    with pytest.raises(mini_sqlite.ProgrammingError):
        substitute("VALUES (?)", (1, 2))


def test_bytes_not_supported():
    with pytest.raises(mini_sqlite.NotSupportedError):
        substitute("VALUES (?)", (b"x",))


def test_unsupported_type():
    with pytest.raises(mini_sqlite.ProgrammingError):
        substitute("VALUES (?)", (object(),))


def test_escaped_quote_inside_string_parsed_correctly():
    # Backslash-quote should not terminate the string for scanning purposes.
    out = substitute("VALUES ('it\\'s') ?", (1,))
    assert out.endswith(" 1")


def test_int_subclass_cannot_inject_via_repr():
    # An ``int`` subclass with a hostile ``__repr__`` must not leak into SQL.
    class Evil(int):
        def __repr__(self) -> str:
            return "1 OR 1=1"

    assert substitute("VALUES (?)", (Evil(7),)) == "VALUES (7)"


def test_str_subclass_cannot_inject_via_replace():
    # A ``str`` subclass overriding ``replace`` must not bypass escaping.
    class Evil(str):
        def replace(self, *a, **kw):  # noqa: ANN002, ANN003, ANN202
            return "'; DROP TABLE t--"

    out = substitute("VALUES (?)", (Evil("ok"),))
    assert out == "VALUES ('ok')"


def test_non_finite_floats_rejected():
    for bad in (float("inf"), float("-inf"), float("nan")):
        with pytest.raises(mini_sqlite.ProgrammingError):
            substitute("VALUES (?)", (bad,))
