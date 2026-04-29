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


def test_bytes_render_as_blob_literal():
    """``bytes`` parameters render as the SQLite ``X'<hex>'`` blob literal."""
    assert substitute("VALUES (?)", (b"\xde\xad\xbe\xef",)) == "VALUES (X'deadbeef')"


def test_empty_bytes_render_as_empty_blob_literal():
    assert substitute("VALUES (?)", (b"",)) == "VALUES (X'')"


def test_bytearray_renders_same_as_bytes():
    assert substitute("VALUES (?)", (bytearray(b"\x00\xff"),)) == "VALUES (X'00ff')"


def test_memoryview_renders_same_as_bytes():
    assert substitute("VALUES (?)", (memoryview(b"\xab\xcd"),)) == "VALUES (X'abcd')"


def test_bytes_subclass_cannot_inject_via_hex():
    """A ``bytes`` subclass overriding ``hex`` must not bypass the literal form.

    The implementation calls ``bytes(value).hex()`` which materialises a
    fresh ``bytes`` object, so a subclass-defined ``.hex`` is bypassed.
    """
    class Evil(bytes):
        def hex(self, *a, **kw):  # noqa: ANN002, ANN003, ANN202
            return "00'; DROP TABLE t--"

    out = substitute("VALUES (?)", (Evil(b"\x01\x02"),))
    assert out == "VALUES (X'0102')"


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


# ---------------------------------------------------------------------------
# Named parameter style (``:name``)
# ---------------------------------------------------------------------------


class TestNamedParameters:
    """``:identifier`` placeholders bound from a Mapping (PEP 249 'named')."""

    def test_single_named_param(self):
        assert substitute("SELECT :x", {"x": 42}) == "SELECT 42"

    def test_multiple_distinct_named_params(self):
        out = substitute(
            "SELECT :a, :b, :c", {"a": 1, "b": 2.5, "c": "hi"}
        )
        assert out == "SELECT 1, 2.5, 'hi'"

    def test_same_named_param_used_twice(self):
        """A named placeholder may appear more than once with the same key."""
        out = substitute(
            "SELECT * FROM t WHERE x = :v OR y = :v", {"v": 7}
        )
        assert out == "SELECT * FROM t WHERE x = 7 OR y = 7"

    def test_extra_dict_keys_are_ignored(self):
        """Per sqlite3, unused dict keys are silently ignored."""
        out = substitute("SELECT :x", {"x": 1, "unused": 999})
        assert out == "SELECT 1"

    def test_missing_key_raises(self):
        with pytest.raises(mini_sqlite.ProgrammingError, match=":missing"):
            substitute("SELECT :missing", {"x": 1})

    def test_named_inside_string_literal_is_ignored(self):
        # ``:x`` inside a literal must not consume the parameter.
        out = substitute(
            "SELECT ':x is not bound' WHERE y = :x", {"x": 5}
        )
        assert out == "SELECT ':x is not bound' WHERE y = 5"

    def test_named_inside_line_comment_is_ignored(self):
        out = substitute(
            "SELECT 1 -- :ignored\nWHERE x = :x", {"x": 9}
        )
        assert out == "SELECT 1 -- :ignored\nWHERE x = 9"

    def test_named_inside_block_comment_is_ignored(self):
        out = substitute(
            "SELECT /* :ignored */ :x", {"x": 9}
        )
        assert out == "SELECT /* :ignored */ 9"

    def test_double_colon_is_not_a_placeholder(self):
        """``a::INT`` (Postgres-style cast) must pass through untouched."""
        out = substitute("SELECT a :: INT FROM t", {})
        assert out == "SELECT a :: INT FROM t"

    def test_underscore_in_identifier(self):
        out = substitute("SELECT :user_id", {"user_id": 42})
        assert out == "SELECT 42"

    def test_digit_after_initial_letter(self):
        out = substitute("SELECT :col1, :col2", {"col1": 1, "col2": 2})
        assert out == "SELECT 1, 2"

    def test_leading_digit_after_colon_is_not_named(self):
        """``:1`` is *numeric* style — not supported here, must not consume."""
        # We expect substitute to leave ``:1`` untouched (then parser will
        # decide how to handle it).  The mapping has no key '1' anyway.
        out = substitute("SELECT :1", {})
        assert out == "SELECT :1"

    def test_qmark_with_mapping_raises(self):
        with pytest.raises(mini_sqlite.ProgrammingError, match="mapping"):
            substitute("SELECT ?", {"x": 1})

    def test_named_with_sequence_raises(self):
        with pytest.raises(mini_sqlite.ProgrammingError, match="not a mapping"):
            substitute("SELECT :x", (1,))

    def test_mixed_paramstyles_raise(self):
        # Order :name then ? with a mapping: :name binds first, then ? hits
        # the "cannot mix" check before the wrong-container check.
        with pytest.raises(mini_sqlite.ProgrammingError, match="mix"):
            substitute("SELECT :x, ?", {"x": 1})

    def test_named_value_types(self):
        """All SQL value types render correctly through named binding."""
        out = substitute(
            "VALUES (:n, :i, :f, :t, :b)",
            {"n": None, "i": 7, "f": 1.5, "t": "ok", "b": True},
        )
        assert out == "VALUES (NULL, 7, 1.5, 'ok', 1)"

    def test_empty_mapping_with_no_placeholders(self):
        assert substitute("SELECT 1", {}) == "SELECT 1"
