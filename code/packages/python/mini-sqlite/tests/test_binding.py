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
