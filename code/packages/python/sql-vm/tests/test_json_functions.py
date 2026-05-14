"""
Tests for JSON scalar functions in sql_vm.scalar_functions.

Coverage targets
----------------
- json()              — canonical (minified) JSON
- json_valid()        — 1 for valid JSON, 0 otherwise
- json_quote()        — SQL value → JSON text
- json_array()        — build a JSON array
- json_object()       — build a JSON object
- json_extract()      — extract value(s) at path(s)
- json_type()         — type name at path
- json_array_length() — length of a JSON array
- json_keys()         — object keys as JSON array
- json_patch()        — RFC 7396 merge patch
- json_remove()       — remove paths
- json_set()          — insert or replace
- json_insert()       — insert only (no overwrite)
- json_replace()      — replace only (no insert)
- json_group_array()  — scalar alias for json_array

All assertions are double-checked against Python's own json module semantics
and SQLite's documented JSON1 extension behaviour.
"""

from __future__ import annotations

import pytest

from sql_vm.errors import WrongNumberOfArguments
from sql_vm.scalar_functions import call


def fn(name: str, *args: object) -> object:
    """Thin wrapper for readability."""
    return call(name, list(args))  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# json() — canonical minification
# ---------------------------------------------------------------------------


class TestJson:
    def test_minifies_object(self) -> None:
        # Whitespace is stripped; key order is preserved.
        assert fn("json", '{ "a" : 1 , "b" : 2 }') == '{"a":1,"b":2}'

    def test_minifies_array(self) -> None:
        assert fn("json", "[ 1 , 2 , 3 ]") == "[1,2,3]"

    def test_null_input(self) -> None:
        assert fn("json", None) is None

    def test_invalid_json(self) -> None:
        assert fn("json", "not-valid") is None

    def test_string_scalar(self) -> None:
        assert fn("json", '"hello"') == '"hello"'

    def test_number_scalar(self) -> None:
        assert fn("json", "42") == "42"


# ---------------------------------------------------------------------------
# json_valid()
# ---------------------------------------------------------------------------


class TestJsonValid:
    def test_valid_object(self) -> None:
        assert fn("json_valid", '{"a":1}') == 1

    def test_valid_array(self) -> None:
        assert fn("json_valid", "[1,2,3]") == 1

    def test_valid_string(self) -> None:
        assert fn("json_valid", '"hello"') == 1

    def test_valid_null(self) -> None:
        assert fn("json_valid", "null") == 1

    def test_invalid(self) -> None:
        assert fn("json_valid", "invalid") == 0

    def test_null_input(self) -> None:
        # SQLite 3.45+: json_valid(NULL) → NULL.
        assert fn("json_valid", None) is None

    def test_empty_string(self) -> None:
        assert fn("json_valid", "") == 0


# ---------------------------------------------------------------------------
# json_quote()
# ---------------------------------------------------------------------------


class TestJsonQuote:
    def test_null(self) -> None:
        assert fn("json_quote", None) == "null"

    def test_integer(self) -> None:
        assert fn("json_quote", 42) == "42"

    def test_float(self) -> None:
        assert fn("json_quote", 3.14) == "3.14"

    def test_string(self) -> None:
        assert fn("json_quote", "hello") == '"hello"'

    def test_string_with_quotes(self) -> None:
        # Internal double-quotes are escaped with backslash.
        result = fn("json_quote", 'say "hi"')
        assert result == '"say \\"hi\\""'

    def test_bool_true(self) -> None:
        # Python True maps to JSON true (not 1).
        result = fn("json_quote", True)
        assert result == "true"

    def test_bool_false(self) -> None:
        result = fn("json_quote", False)
        assert result == "false"


# ---------------------------------------------------------------------------
# json_array()
# ---------------------------------------------------------------------------


class TestJsonArray:
    def test_empty(self) -> None:
        assert fn("json_array") == "[]"

    def test_integers(self) -> None:
        assert fn("json_array", 1, 2, 3) == "[1,2,3]"

    def test_mixed_types(self) -> None:
        assert fn("json_array", 1, "two", 3.0) == '[1,"two",3.0]'

    def test_with_null(self) -> None:
        # NULL arguments become JSON null.
        assert fn("json_array", 1, None, 3) == "[1,null,3]"

    def test_nested_array(self) -> None:
        # Passing an already-encoded JSON array string is treated as a string.
        # json_array always treats its inputs as SQL values.
        result = fn("json_array", "a", "b")
        assert result == '["a","b"]'


# ---------------------------------------------------------------------------
# json_object()
# ---------------------------------------------------------------------------


class TestJsonObject:
    def test_simple(self) -> None:
        assert fn("json_object", "a", 1, "b", 2) == '{"a":1,"b":2}'

    def test_null_value(self) -> None:
        assert fn("json_object", "x", None) == '{"x":null}'

    def test_string_value(self) -> None:
        assert fn("json_object", "name", "Alice") == '{"name":"Alice"}'

    def test_float_value(self) -> None:
        result = fn("json_object", "pi", 3.14)
        assert result == '{"pi":3.14}'

    def test_odd_args_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("json_object", "key")   # missing value

    def test_no_args(self) -> None:
        # Zero args → empty object.
        assert fn("json_object") == "{}"


# ---------------------------------------------------------------------------
# json_extract()
# ---------------------------------------------------------------------------


class TestJsonExtract:
    def test_object_field(self) -> None:
        assert fn("json_extract", '{"a":1,"b":2}', "$.a") == 1

    def test_nested_field(self) -> None:
        assert fn("json_extract", '{"a":{"b":42}}', "$.a.b") == 42

    def test_array_index(self) -> None:
        assert fn("json_extract", "[10,20,30]", "$[1]") == 20

    def test_nested_array(self) -> None:
        doc = '{"arr":[1,2,3]}'
        assert fn("json_extract", doc, "$.arr[1]") == 2

    def test_missing_path(self) -> None:
        # Path not found → NULL.
        assert fn("json_extract", '{"a":1}', "$.missing") is None

    def test_null_json(self) -> None:
        assert fn("json_extract", None, "$.a") is None

    def test_invalid_json(self) -> None:
        assert fn("json_extract", "invalid", "$.a") is None

    def test_json_null_value(self) -> None:
        # The JSON document contains null at the path.
        assert fn("json_extract", '{"a":null}', "$.a") is None

    def test_array_result(self) -> None:
        # Extracting a JSON array returns it as a JSON text string.
        result = fn("json_extract", '{"arr":[1,2,3]}', "$.arr")
        assert result == "[1,2,3]"

    def test_object_result(self) -> None:
        # Extracting a JSON object returns it as a JSON text string.
        result = fn("json_extract", '{"obj":{"x":1}}', "$.obj")
        assert result == '{"x":1}'

    def test_multiple_paths(self) -> None:
        # Two or more paths → returns a JSON array of results.
        result = fn("json_extract", '{"a":1,"b":2}', "$.a", "$.b")
        assert result == "[1,2]"

    def test_multiple_paths_with_missing(self) -> None:
        # Missing paths appear as null in the output array.
        result = fn("json_extract", '{"a":1}', "$.a", "$.missing")
        assert result == "[1,null]"

    def test_string_value(self) -> None:
        # JSON string is returned as a SQL string (without surrounding quotes).
        assert fn("json_extract", '{"s":"hello"}', "$.s") == "hello"

    def test_boolean_true(self) -> None:
        # JSON true → SQL integer 1 (SQLite convention).
        assert fn("json_extract", '{"b":true}', "$.b") == 1

    def test_boolean_false(self) -> None:
        # JSON false → SQL integer 0.
        assert fn("json_extract", '{"b":false}', "$.b") == 0

    def test_negative_array_index(self) -> None:
        # Negative array index counts from end.
        assert fn("json_extract", "[10,20,30]", "$[-1]") == 30

    def test_root_path(self) -> None:
        # $ alone returns the full document (as JSON text for complex types).
        result = fn("json_extract", "[1,2,3]", "$")
        assert result == "[1,2,3]"

    def test_non_string_path_in_multi_path(self) -> None:
        # Non-string path argument → treated as missing → null in array.
        result = fn("json_extract", '{"a":1}', "$.a", 42)  # 42 is not a string path
        import json as _j
        parsed = _j.loads(result)  # type: ignore[arg-type]
        assert parsed == [1, None]

    def test_non_string_json(self) -> None:
        # Non-string JSON argument → NULL.
        assert fn("json_extract", 42, "$.a") is None


# ---------------------------------------------------------------------------
# json_type()
# ---------------------------------------------------------------------------


class TestJsonType:
    def test_root_object(self) -> None:
        assert fn("json_type", '{"a":1}') == "object"

    def test_root_array(self) -> None:
        assert fn("json_type", "[1,2]") == "array"

    def test_root_string(self) -> None:
        assert fn("json_type", '"hello"') == "text"

    def test_root_integer(self) -> None:
        assert fn("json_type", "42") == "integer"

    def test_root_real(self) -> None:
        assert fn("json_type", "3.14") == "real"

    def test_root_null(self) -> None:
        assert fn("json_type", "null") == "null"

    def test_root_true(self) -> None:
        assert fn("json_type", "true") == "true"

    def test_root_false(self) -> None:
        assert fn("json_type", "false") == "false"

    def test_path_field(self) -> None:
        assert fn("json_type", '{"a":1}', "$.a") == "integer"

    def test_path_array_field(self) -> None:
        assert fn("json_type", '{"a":[1,2]}', "$.a") == "array"

    def test_path_missing(self) -> None:
        # Path not found → NULL (not an error).
        assert fn("json_type", '{"a":1}', "$.missing") is None

    def test_null_json(self) -> None:
        assert fn("json_type", None) is None

    def test_invalid_json(self) -> None:
        assert fn("json_type", "invalid") is None

    def test_non_string_json(self) -> None:
        # Integer input → NULL (not a string).
        assert fn("json_type", 42) is None


# ---------------------------------------------------------------------------
# json_array_length()
# ---------------------------------------------------------------------------


class TestJsonArrayLength:
    def test_root_array(self) -> None:
        assert fn("json_array_length", "[1,2,3]") == 3

    def test_empty_array(self) -> None:
        assert fn("json_array_length", "[]") == 0

    def test_path_to_array(self) -> None:
        assert fn("json_array_length", '{"a":[1,2]}', "$.a") == 2

    def test_not_an_array(self) -> None:
        # Valid JSON that is not an array → 0 (matching SQLite: "0 if X is some
        # kind of JSON value other than an array").
        assert fn("json_array_length", '{"a":1}') == 0

    def test_null_json(self) -> None:
        assert fn("json_array_length", None) is None

    def test_invalid_json(self) -> None:
        assert fn("json_array_length", "bad") is None

    def test_path_not_found(self) -> None:
        assert fn("json_array_length", "[1,2]", "$.missing") is None

    def test_non_string_json(self) -> None:
        # Non-string input → NULL.
        assert fn("json_array_length", 42) is None


# ---------------------------------------------------------------------------
# json_keys()
# ---------------------------------------------------------------------------


class TestJsonKeys:
    def test_simple_object(self) -> None:
        result = fn("json_keys", '{"a":1,"b":2}')
        import json
        keys = json.loads(result)  # type: ignore[arg-type]
        assert set(keys) == {"a", "b"}

    def test_empty_object(self) -> None:
        assert fn("json_keys", "{}") == "[]"

    def test_path_to_object(self) -> None:
        result = fn("json_keys", '{"x":{"y":3,"z":4}}', "$.x")
        import json
        keys = json.loads(result)  # type: ignore[arg-type]
        assert set(keys) == {"y", "z"}

    def test_not_an_object(self) -> None:
        # Arrays are not objects → NULL.
        assert fn("json_keys", "[1,2]") is None

    def test_null_json(self) -> None:
        assert fn("json_keys", None) is None

    def test_path_not_found(self) -> None:
        assert fn("json_keys", '{"a":1}', "$.missing") is None

    def test_invalid_json(self) -> None:
        # Invalid JSON → NULL.
        assert fn("json_keys", "bad-json") is None

    def test_non_string_json(self) -> None:
        # Non-string input → NULL.
        assert fn("json_keys", 42) is None


# ---------------------------------------------------------------------------
# json_patch()
# ---------------------------------------------------------------------------


class TestJsonPatch:
    def test_simple_merge(self) -> None:
        result = fn("json_patch", '{"a":1,"b":2}', '{"b":99}')
        import json
        assert json.loads(result) == {"a": 1, "b": 99}  # type: ignore[arg-type]

    def test_remove_with_null(self) -> None:
        # In merge-patch, a patch key with value null removes the key.
        result = fn("json_patch", '{"a":1,"b":2}', '{"b":null}')
        import json
        assert json.loads(result) == {"a": 1}  # type: ignore[arg-type]

    def test_add_key(self) -> None:
        result = fn("json_patch", '{"a":1}', '{"c":3}')
        import json
        assert json.loads(result) == {"a": 1, "c": 3}  # type: ignore[arg-type]

    def test_null_input(self) -> None:
        assert fn("json_patch", None, '{"a":1}') is None
        assert fn("json_patch", '{"a":1}', None) is None

    def test_invalid_json(self) -> None:
        assert fn("json_patch", "invalid", '{"a":1}') is None

    def test_non_string_inputs(self) -> None:
        # Non-string inputs → NULL.
        assert fn("json_patch", 42, '{"a":1}') is None
        assert fn("json_patch", '{"a":1}', 42) is None

    def test_array_patch(self) -> None:
        # Patch is not an object → replace target entirely.
        result = fn("json_patch", "[1,2,3]", "[4,5]")
        assert result == "[4,5]"


# ---------------------------------------------------------------------------
# json_remove()
# ---------------------------------------------------------------------------


class TestJsonRemove:
    def test_remove_field(self) -> None:
        result = fn("json_remove", '{"a":1,"b":2}', "$.a")
        import json
        assert json.loads(result) == {"b": 2}  # type: ignore[arg-type]

    def test_remove_array_element(self) -> None:
        result = fn("json_remove", "[1,2,3]", "$[1]")
        assert result == "[1,3]"

    def test_remove_missing_path(self) -> None:
        # Missing path is silently ignored.
        result = fn("json_remove", '{"a":1}', "$.missing")
        assert result == '{"a":1}'

    def test_remove_multiple(self) -> None:
        result = fn("json_remove", '{"a":1,"b":2,"c":3}', "$.a", "$.c")
        import json
        assert json.loads(result) == {"b": 2}  # type: ignore[arg-type]

    def test_null_json(self) -> None:
        assert fn("json_remove", None, "$.a") is None

    def test_invalid_json(self) -> None:
        assert fn("json_remove", "bad", "$.a") is None

    def test_too_few_args_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("json_remove", '{"a":1}')   # missing path

    def test_non_string_json(self) -> None:
        # Non-string JSON input → NULL.
        assert fn("json_remove", 42, "$.a") is None


# ---------------------------------------------------------------------------
# json_set()
# ---------------------------------------------------------------------------


class TestJsonSet:
    def test_overwrite_existing(self) -> None:
        result = fn("json_set", '{"a":1}', "$.a", 99)
        assert result == '{"a":99}'

    def test_insert_new_key(self) -> None:
        result = fn("json_set", '{"a":1}', "$.b", 2)
        import json
        assert json.loads(result) == {"a": 1, "b": 2}  # type: ignore[arg-type]

    def test_multiple_pairs(self) -> None:
        result = fn("json_set", '{"a":1}', "$.a", 10, "$.b", 20)
        import json
        assert json.loads(result) == {"a": 10, "b": 20}  # type: ignore[arg-type]

    def test_null_json(self) -> None:
        assert fn("json_set", None, "$.a", 1) is None

    def test_invalid_json(self) -> None:
        assert fn("json_set", "bad", "$.a", 1) is None

    def test_array_index(self) -> None:
        result = fn("json_set", "[1,2,3]", "$[1]", 99)
        assert result == "[1,99,3]"

    def test_wrong_arg_count_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("json_set", '{"a":1}', "$.a")   # missing value

    def test_non_string_json(self) -> None:
        # Non-string JSON input → NULL.
        assert fn("json_set", 42, "$.a", 1) is None


# ---------------------------------------------------------------------------
# json_insert()
# ---------------------------------------------------------------------------


class TestJsonInsert:
    def test_insert_new_key(self) -> None:
        result = fn("json_insert", '{"a":1}', "$.b", 2)
        import json
        assert json.loads(result) == {"a": 1, "b": 2}  # type: ignore[arg-type]

    def test_no_overwrite_existing(self) -> None:
        # Key already exists → unchanged.
        result = fn("json_insert", '{"a":1}', "$.a", 99)
        assert result == '{"a":1}'

    def test_null_json(self) -> None:
        assert fn("json_insert", None, "$.a", 1) is None

    def test_wrong_arg_count_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("json_insert", '{"a":1}', "$.b")  # missing value

    def test_non_string_json(self) -> None:
        # Non-string JSON input → NULL.
        assert fn("json_insert", 42, "$.a", 1) is None

    def test_invalid_json(self) -> None:
        assert fn("json_insert", "bad-json", "$.a", 1) is None


# ---------------------------------------------------------------------------
# json_replace()
# ---------------------------------------------------------------------------


class TestJsonReplace:
    def test_replace_existing(self) -> None:
        result = fn("json_replace", '{"a":1}', "$.a", 99)
        assert result == '{"a":99}'

    def test_no_insert_missing(self) -> None:
        # Path does not exist → no change.
        result = fn("json_replace", '{"a":1}', "$.b", 2)
        assert result == '{"a":1}'

    def test_null_json(self) -> None:
        assert fn("json_replace", None, "$.a", 1) is None

    def test_wrong_arg_count_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("json_replace", '{"a":1}', "$.a")  # missing value

    def test_non_string_json(self) -> None:
        # Non-string JSON input → NULL.
        assert fn("json_replace", 42, "$.a", 1) is None

    def test_invalid_json(self) -> None:
        assert fn("json_replace", "bad-json", "$.a", 1) is None


# ---------------------------------------------------------------------------
# json_group_array() — scalar alias
# ---------------------------------------------------------------------------


class TestJsonGroupArray:
    def test_alias_for_json_array(self) -> None:
        # json_group_array acts as a scalar alias for json_array.
        assert fn("json_group_array", 1, 2, 3) == "[1,2,3]"

    def test_empty(self) -> None:
        assert fn("json_group_array") == "[]"

    def test_with_null(self) -> None:
        assert fn("json_group_array", 1, None, 3) == "[1,null,3]"
