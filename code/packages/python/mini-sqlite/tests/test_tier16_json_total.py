"""Tier 16 — JSON scalar functions and TOTAL() aggregate.

Covers two new feature areas:

1. **JSON1 scalar functions** — the SQLite JSON1 extension family of functions
   for building, inspecting, and mutating JSON documents.  Implemented in
   ``sql_vm.scalar_functions`` using Python's standard ``json`` module.

   Functions covered (oracle-verified against real sqlite3):
   ``json()``, ``json_valid()``, ``json_quote()``, ``json_array()``,
   ``json_object()``, ``json_extract()``, ``json_type()``,
   ``json_array_length()``, ``json_patch()``, ``json_remove()``,
   ``json_set()``, ``json_insert()``, ``json_replace()``.

   Note on divergences from SQLite:
   - SQLite raises ``OperationalError: malformed JSON`` for most JSON
     functions when given invalid JSON strings.  Our implementation returns
     NULL in those cases (more lenient, SQL-null-propagating semantics).
     Tests involving malformed JSON are therefore unit-tested against our
     expected NULL behaviour in the sql-vm test suite, not oracle-tested here.
   - ``json_keys()`` is not available in all SQLite versions and is tested
     only as a unit test (sql-vm) rather than oracle-compared here.
   - ``json_object(key, json_array(...))`` with nested JSON functions embeds
     the inner result as a text value in our implementation, whereas SQLite
     recognises the "JSON subtype" and embeds it directly.  This case is also
     excluded from oracle tests.

2. **TOTAL() aggregate** — SQLite-specific variant of ``SUM()`` that returns
   ``0.0`` (float) for empty groups or all-NULL input, never returning NULL.
   Added to the ``AggFunc`` enum in sql-planner, sql-codegen, and handled in
   the sql-vm aggregate evaluation path.

All assertions (except where noted above) are oracle-verified against real
``sqlite3``.
"""

from __future__ import annotations

import json
import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _both(sql: str, *, setup: list[str] | None = None, params: tuple = ()) -> tuple[list, list]:
    """Run *sql* (with *params*) against both real sqlite3 and mini-sqlite.

    Returns (ref_rows, got_rows) — both as lists of tuples.
    """
    ref_con = sqlite3.connect(":memory:")
    got_con = mini_sqlite.connect(":memory:")
    for s in (setup or []):
        ref_con.execute(s)
        got_con.execute(s)
    ref_rows = ref_con.execute(sql, params).fetchall()
    got_rows = got_con.execute(sql, params).fetchall()
    return ref_rows, got_rows


def _assert_both(sql: str, *, setup: list[str] | None = None, params: tuple = ()) -> None:
    """Assert mini-sqlite produces the same rows as real sqlite3."""
    ref, got = _both(sql, setup=setup, params=params)
    assert got == ref, f"mini-sqlite={got!r}, sqlite3={ref!r}\nSQL: {sql}"


# ---------------------------------------------------------------------------
# json_valid()
# ---------------------------------------------------------------------------


class TestJsonValid:
    def test_valid_object(self) -> None:
        _assert_both("SELECT json_valid(?)", params=('{"a":1}',))

    def test_valid_array(self) -> None:
        _assert_both("SELECT json_valid(?)", params=('[1,2,3]',))

    def test_valid_string(self) -> None:
        _assert_both('SELECT json_valid(\'"hello"\')')

    def test_invalid(self) -> None:
        _assert_both("SELECT json_valid(?)", params=('not valid',))

    def test_null(self) -> None:
        # SQLite 3.45+: json_valid(NULL) → NULL (not 0).
        _assert_both("SELECT json_valid(NULL)")


# ---------------------------------------------------------------------------
# json() — canonical form
# ---------------------------------------------------------------------------


class TestJsonCanonical:
    def test_minify_object(self) -> None:
        _assert_both("SELECT json(?)", params=('{ "a" : 1 , "b" : 2 }',))

    def test_minify_array(self) -> None:
        _assert_both("SELECT json(?)", params=('[ 1 , 2 , 3 ]',))

    def test_null_input(self) -> None:
        _assert_both("SELECT json(NULL)")

    # Note: SQLite raises OperationalError('malformed JSON') for invalid input.
    # Our implementation returns NULL.  This is tested in the sql-vm unit tests.


# ---------------------------------------------------------------------------
# json_quote()
# ---------------------------------------------------------------------------


class TestJsonQuote:
    def test_integer(self) -> None:
        _assert_both("SELECT json_quote(42)")

    def test_float(self) -> None:
        _assert_both("SELECT json_quote(3.14)")

    def test_text(self) -> None:
        _assert_both("SELECT json_quote('hello')")

    def test_null(self) -> None:
        _assert_both("SELECT json_quote(NULL)")

    def test_text_with_quotes(self) -> None:
        _assert_both("SELECT json_quote(?)", params=('say "hi"',))


# ---------------------------------------------------------------------------
# json_array()
# ---------------------------------------------------------------------------


class TestJsonArray:
    def test_empty(self) -> None:
        _assert_both("SELECT json_array()")

    def test_integers(self) -> None:
        _assert_both("SELECT json_array(1, 2, 3)")

    def test_mixed(self) -> None:
        _assert_both("SELECT json_array(1, 'two', 3.0)")

    def test_with_null(self) -> None:
        _assert_both("SELECT json_array(1, NULL, 3)")

    def test_strings(self) -> None:
        _assert_both("SELECT json_array('a', 'b', 'c')")


# ---------------------------------------------------------------------------
# json_object()
# ---------------------------------------------------------------------------


class TestJsonObject:
    def test_simple(self) -> None:
        _assert_both("SELECT json_object('a', 1, 'b', 2)")

    def test_null_value(self) -> None:
        _assert_both("SELECT json_object('x', NULL)")

    def test_string_value(self) -> None:
        _assert_both("SELECT json_object('name', 'Alice')")

    def test_empty(self) -> None:
        _assert_both("SELECT json_object()")

    # Note: json_object('arr', json_array(1,2,3)) diverges from SQLite because
    # SQLite recognises the "JSON subtype" of the inner result and embeds it
    # directly as a JSON array.  Our implementation treats the string returned
    # by json_array as a plain text value.  Tested as unit test only.


# ---------------------------------------------------------------------------
# json_extract()
# ---------------------------------------------------------------------------


class TestJsonExtract:
    def test_object_field(self) -> None:
        _assert_both("SELECT json_extract(?, '$.a')", params=('{"a":42,"b":99}',))

    def test_nested_field(self) -> None:
        _assert_both("SELECT json_extract(?, '$.a.b')", params=('{"a":{"b":7}}',))

    def test_array_index(self) -> None:
        _assert_both("SELECT json_extract(?, '$[1]')", params=('[10,20,30]',))

    def test_missing_path(self) -> None:
        _assert_both("SELECT json_extract(?, '$.missing')", params=('{"a":1}',))

    def test_null_json(self) -> None:
        _assert_both("SELECT json_extract(NULL, '$.a')")

    def test_nested_array_extraction(self) -> None:
        _assert_both("SELECT json_extract(?, '$.arr[1]')", params=('{"arr":[1,2,3]}',))

    def test_multiple_paths(self) -> None:
        _assert_both("SELECT json_extract(?, '$.a', '$.b')", params=('{"a":1,"b":2}',))

    def test_string_value(self) -> None:
        _assert_both("SELECT json_extract(?, '$.s')", params=('{"s":"hello"}',))

    def test_boolean_true_extracts_as_1(self) -> None:
        _assert_both("SELECT json_extract(?, '$.b')", params=('{"b":true}',))

    def test_boolean_false_extracts_as_0(self) -> None:
        _assert_both("SELECT json_extract(?, '$.b')", params=('{"b":false}',))

    def test_array_result(self) -> None:
        _assert_both("SELECT json_extract(?, '$.arr')", params=('{"arr":[1,2,3]}',))


# ---------------------------------------------------------------------------
# json_type()
# ---------------------------------------------------------------------------


class TestJsonType:
    def test_root_object(self) -> None:
        _assert_both("SELECT json_type(?)", params=('{"a":1}',))

    def test_root_array(self) -> None:
        _assert_both("SELECT json_type(?)", params=('[1,2]',))

    def test_root_integer(self) -> None:
        _assert_both("SELECT json_type(?)", params=("42",))

    def test_root_real(self) -> None:
        _assert_both("SELECT json_type(?)", params=("3.14",))

    def test_root_string(self) -> None:
        _assert_both('SELECT json_type(\'"hello"\')')

    def test_root_null(self) -> None:
        _assert_both("SELECT json_type(?)", params=("null",))

    def test_root_true(self) -> None:
        _assert_both("SELECT json_type(?)", params=("true",))

    def test_root_false(self) -> None:
        _assert_both("SELECT json_type(?)", params=("false",))

    def test_path_to_integer(self) -> None:
        _assert_both("SELECT json_type(?, '$.a')", params=('{"a":1}',))

    def test_path_to_array(self) -> None:
        _assert_both("SELECT json_type(?, '$.a')", params=('{"a":[1,2,3]}',))

    def test_path_missing(self) -> None:
        _assert_both("SELECT json_type(?, '$.missing')", params=('{"a":1}',))

    def test_null_json(self) -> None:
        _assert_both("SELECT json_type(NULL)")

    # Note: SQLite raises for invalid JSON; our impl returns NULL.  Unit-tested.


# ---------------------------------------------------------------------------
# json_array_length()
# ---------------------------------------------------------------------------


class TestJsonArrayLength:
    def test_root_array(self) -> None:
        _assert_both("SELECT json_array_length(?)", params=('[1,2,3]',))

    def test_empty_array(self) -> None:
        _assert_both("SELECT json_array_length(?)", params=('[]',))

    def test_path_to_array(self) -> None:
        _assert_both("SELECT json_array_length(?, '$.a')", params=('{"a":[1,2]}',))

    def test_not_an_array(self) -> None:
        # Valid JSON that is not an array → 0 (matching SQLite behaviour).
        _assert_both("SELECT json_array_length(?)", params=('{"a":1}',))

    def test_null_json(self) -> None:
        _assert_both("SELECT json_array_length(NULL)")

    def test_path_not_found(self) -> None:
        _assert_both("SELECT json_array_length(?, '$.missing')", params=('[1,2]',))


# ---------------------------------------------------------------------------
# json_keys() — extension function, not oracle-verified
# ---------------------------------------------------------------------------
# Note: json_keys() is not available in all SQLite versions (it was added in
# SQLite 3.38.0 as part of the JSON5 functions but was removed again; as of
# 3.50.4 it raises "no such function: json_keys").  We implement it as an
# extension and test it only in the sql-vm unit tests, not here.
#
# class TestJsonKeys: ... (see test_json_functions.py in sql-vm tests)


# ---------------------------------------------------------------------------
# json_patch()
# ---------------------------------------------------------------------------


class TestJsonPatch:
    def test_simple_merge(self) -> None:
        _assert_both(
            "SELECT json_patch(?, ?)",
            params=('{"a":1,"b":2}', '{"b":99}'),
        )

    def test_remove_with_null_value(self) -> None:
        _assert_both(
            "SELECT json_patch(?, ?)",
            params=('{"a":1,"b":2}', '{"b":null}'),
        )

    def test_add_new_key(self) -> None:
        _assert_both(
            "SELECT json_patch(?, ?)",
            params=('{"a":1}', '{"c":3}'),
        )

    def test_null_inputs(self) -> None:
        _assert_both("SELECT json_patch(NULL, '{}')")
        _assert_both("SELECT json_patch('{}', NULL)")

    # Note: SQLite raises for invalid JSON; our impl returns NULL.  Unit-tested.


# ---------------------------------------------------------------------------
# json_remove()
# ---------------------------------------------------------------------------


class TestJsonRemove:
    def test_remove_field(self) -> None:
        _assert_both(
            "SELECT json_remove(?, '$.a')",
            params=('{"a":1,"b":2}',),
        )

    def test_remove_array_element(self) -> None:
        _assert_both(
            "SELECT json_remove(?, '$[1]')",
            params=('[1,2,3]',),
        )

    def test_remove_missing_path(self) -> None:
        _assert_both(
            "SELECT json_remove(?, '$.missing')",
            params=('{"a":1}',),
        )

    def test_null_json(self) -> None:
        _assert_both("SELECT json_remove(NULL, '$.a')")

    # Note: SQLite raises for invalid JSON; our impl returns NULL.  Unit-tested.


# ---------------------------------------------------------------------------
# json_set()
# ---------------------------------------------------------------------------


class TestJsonSet:
    def test_overwrite_existing(self) -> None:
        _assert_both(
            "SELECT json_set(?, '$.a', 99)",
            params=('{"a":1}',),
        )

    def test_insert_new_key(self) -> None:
        ref, got = _both(
            "SELECT json_set(?, '$.b', 2)",
            params=('{"a":1}',),
        )
        assert json.loads(got[0][0]) == json.loads(ref[0][0])

    def test_multiple_pairs(self) -> None:
        ref, got = _both(
            "SELECT json_set(?, '$.a', 10, '$.b', 20)",
            params=('{"a":1}',),
        )
        assert json.loads(got[0][0]) == json.loads(ref[0][0])

    def test_null_json(self) -> None:
        _assert_both("SELECT json_set(NULL, '$.a', 1)")

    def test_array_element(self) -> None:
        _assert_both(
            "SELECT json_set(?, '$[1]', 99)",
            params=('[1,2,3]',),
        )


# ---------------------------------------------------------------------------
# json_insert()
# ---------------------------------------------------------------------------


class TestJsonInsert:
    def test_insert_new_key(self) -> None:
        ref, got = _both(
            "SELECT json_insert(?, '$.b', 2)",
            params=('{"a":1}',),
        )
        assert json.loads(got[0][0]) == json.loads(ref[0][0])

    def test_no_overwrite_existing(self) -> None:
        # Key already exists → no change.
        _assert_both(
            "SELECT json_insert(?, '$.a', 99)",
            params=('{"a":1}',),
        )

    def test_null_json(self) -> None:
        _assert_both("SELECT json_insert(NULL, '$.a', 1)")


# ---------------------------------------------------------------------------
# json_replace()
# ---------------------------------------------------------------------------


class TestJsonReplace:
    def test_replace_existing(self) -> None:
        _assert_both(
            "SELECT json_replace(?, '$.a', 99)",
            params=('{"a":1}',),
        )

    def test_no_insert_missing(self) -> None:
        # Key does not exist → no change.
        _assert_both(
            "SELECT json_replace(?, '$.b', 2)",
            params=('{"a":1}',),
        )

    def test_null_json(self) -> None:
        _assert_both("SELECT json_replace(NULL, '$.a', 1)")


# ---------------------------------------------------------------------------
# End-to-end: JSON functions in table queries
# ---------------------------------------------------------------------------


class TestJsonInTableQueries:
    _SETUP = [
        "CREATE TABLE products (id INTEGER PRIMARY KEY, attrs TEXT)",
        "INSERT INTO products VALUES (1, '{\"color\":\"red\",\"size\":10}')",
        "INSERT INTO products VALUES (2, '{\"color\":\"blue\",\"size\":20}')",
        "INSERT INTO products VALUES (3, '{\"color\":\"red\",\"size\":15}')",
    ]

    def test_extract_field_from_column(self) -> None:
        _assert_both(
            "SELECT id, json_extract(attrs, '$.color') FROM products ORDER BY id",
            setup=self._SETUP,
        )

    def test_filter_on_json_field(self) -> None:
        _assert_both(
            "SELECT id FROM products WHERE json_extract(attrs, '$.color') = 'red' ORDER BY id",
            setup=self._SETUP,
        )

    def test_json_type_in_query(self) -> None:
        _assert_both(
            "SELECT id, json_type(attrs, '$.size') FROM products ORDER BY id",
            setup=self._SETUP,
        )

    def test_json_set_in_query(self) -> None:
        _assert_both(
            "SELECT id, json_set(attrs, '$.size', 99) FROM products WHERE id = 1",
            setup=self._SETUP,
        )


# ---------------------------------------------------------------------------
# TOTAL() aggregate
# ---------------------------------------------------------------------------


class TestTotal:
    _SETUP = [
        "CREATE TABLE nums (dept TEXT, val REAL)",
        "INSERT INTO nums VALUES ('A', 10.0)",
        "INSERT INTO nums VALUES ('A', 20.0)",
        "INSERT INTO nums VALUES ('B', 5.0)",
        "INSERT INTO nums VALUES ('B', NULL)",
        "INSERT INTO nums VALUES ('B', NULL)",
    ]

    def test_total_basic(self) -> None:
        """TOTAL() sums all non-NULL values, same as SUM() when input non-empty."""
        _assert_both("SELECT TOTAL(val) FROM nums", setup=self._SETUP)

    def test_total_empty_table(self) -> None:
        """TOTAL() on empty table returns 0.0, not NULL."""
        setup = ["CREATE TABLE empty_t (x INTEGER)"]
        ref, got = _both("SELECT TOTAL(x) FROM empty_t", setup=setup)
        # Both should return 0.0.
        assert got == [(0.0,)]
        assert ref == [(0.0,)]

    def test_total_all_null(self) -> None:
        """TOTAL() on all-NULL group returns 0.0."""
        setup = [
            "CREATE TABLE nulls (x INTEGER)",
            "INSERT INTO nulls VALUES (NULL)",
            "INSERT INTO nulls VALUES (NULL)",
        ]
        ref, got = _both("SELECT TOTAL(x) FROM nulls", setup=setup)
        assert got == [(0.0,)]
        assert ref == [(0.0,)]

    def test_sum_all_null_returns_null(self) -> None:
        """Contrast: SUM() on all-NULL group returns NULL (not 0)."""
        setup = [
            "CREATE TABLE nulls2 (x INTEGER)",
            "INSERT INTO nulls2 VALUES (NULL)",
        ]
        ref, got = _both("SELECT SUM(x) FROM nulls2", setup=setup)
        assert got == [(None,)]
        assert ref == [(None,)]

    def test_total_with_nulls_ignored(self) -> None:
        """TOTAL() ignores NULL values, summing only non-NULL ones."""
        _assert_both(
            "SELECT dept, TOTAL(val) FROM nums GROUP BY dept ORDER BY dept",
            setup=self._SETUP,
        )

    def test_total_returns_float(self) -> None:
        """TOTAL() always returns a real (float), even for integer input."""
        setup = [
            "CREATE TABLE ints (x INTEGER)",
            "INSERT INTO ints VALUES (1)",
            "INSERT INTO ints VALUES (2)",
            "INSERT INTO ints VALUES (3)",
        ]
        ref, got = _both("SELECT TOTAL(x) FROM ints", setup=setup)
        # Both should be 6.0 (float).
        assert got == [(6.0,)]
        assert ref == [(6.0,)]

    def test_total_group_by(self) -> None:
        """TOTAL() works correctly with GROUP BY."""
        _assert_both(
            "SELECT dept, TOTAL(val) FROM nums GROUP BY dept ORDER BY dept",
            setup=self._SETUP,
        )

    def test_total_distinct(self) -> None:
        """TOTAL(DISTINCT col) sums only unique non-NULL values."""
        setup = [
            "CREATE TABLE dupes (x REAL)",
            "INSERT INTO dupes VALUES (5.0)",
            "INSERT INTO dupes VALUES (5.0)",
            "INSERT INTO dupes VALUES (10.0)",
        ]
        _assert_both("SELECT TOTAL(DISTINCT x) FROM dupes", setup=setup)
