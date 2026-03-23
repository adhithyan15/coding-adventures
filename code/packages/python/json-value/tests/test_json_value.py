"""Tests for the json-value package.

These tests cover all 44 test cases from the D20 spec, organized into
four sections:

1. from_ast() -- converting parser ASTs to JsonValue (tests 1-20)
2. to_native() -- converting JsonValue to native Python (tests 21-28)
3. from_native() -- converting native Python to JsonValue (tests 29-38)
4. parse() and parse_native() convenience functions (tests 39-42)
5. Round-trip tests (tests 43-44)

Each test follows the pattern:
- Parse JSON text with the full pipeline (lexer --> parser --> AST)
- Convert the AST to JsonValue with from_ast()
- Assert the JsonValue has the expected structure and values

The tests use the real json-parser to produce ASTs, so they also serve
as integration tests for the full JSON pipeline.
"""

from __future__ import annotations

import pytest

from json_value import (
    JsonArray,
    JsonBool,
    JsonNull,
    JsonNumber,
    JsonObject,
    JsonString,
    JsonValue,
    JsonValueError,
    from_ast,
    from_native,
    parse,
    parse_native,
    to_native,
)

# We import parse_json to create real ASTs for from_ast tests.
from json_parser import parse_json
# We import ASTNode and Token to construct synthetic ASTs for error-path tests.
from lang_parser import ASTNode
from lexer import Token


# ===========================================================================
# Section 1: from_ast() tests (spec tests 1-20)
# ===========================================================================


class TestFromAst:
    """Tests for converting json-parser ASTs to JsonValue objects.

    These tests use the real json-parser to produce ASTs, then verify
    that from_ast() correctly converts them to the expected JsonValue types.
    """

    # -- Test 1: Empty object --
    def test_empty_object(self) -> None:
        """parse '{}' --> AST --> from_ast --> JsonObject with empty pairs."""
        ast = parse_json("{}")
        result = from_ast(ast)
        assert isinstance(result, JsonObject)
        assert result.pairs == {}

    # -- Test 2: Empty array --
    def test_empty_array(self) -> None:
        """parse '[]' --> AST --> from_ast --> JsonArray with empty elements."""
        ast = parse_json("[]")
        result = from_ast(ast)
        assert isinstance(result, JsonArray)
        assert result.elements == []

    # -- Test 3: String --
    def test_string(self) -> None:
        """parse '"hello"' --> AST --> from_ast --> JsonString("hello")."""
        ast = parse_json('"hello"')
        result = from_ast(ast)
        assert isinstance(result, JsonString)
        assert result.value == "hello"

    # -- Test 4: Integer --
    def test_integer(self) -> None:
        """parse '42' --> AST --> from_ast --> JsonNumber(42) as int."""
        ast = parse_json("42")
        result = from_ast(ast)
        assert isinstance(result, JsonNumber)
        assert result.value == 42
        assert isinstance(result.value, int)

    # -- Test 5: Negative integer --
    def test_negative_integer(self) -> None:
        """parse '-17' --> AST --> from_ast --> JsonNumber(-17) as int."""
        ast = parse_json("-17")
        result = from_ast(ast)
        assert isinstance(result, JsonNumber)
        assert result.value == -17
        assert isinstance(result.value, int)

    # -- Test 6: Float --
    def test_float(self) -> None:
        """parse '3.14' --> AST --> from_ast --> JsonNumber(3.14) as float."""
        ast = parse_json("3.14")
        result = from_ast(ast)
        assert isinstance(result, JsonNumber)
        assert result.value == pytest.approx(3.14)
        assert isinstance(result.value, float)

    # -- Test 7: Exponent --
    def test_exponent(self) -> None:
        """parse '1e10' --> AST --> from_ast --> JsonNumber(float)."""
        ast = parse_json("1e10")
        result = from_ast(ast)
        assert isinstance(result, JsonNumber)
        assert result.value == pytest.approx(1e10)
        assert isinstance(result.value, float)

    # -- Test 8: True --
    def test_true(self) -> None:
        """parse 'true' --> AST --> from_ast --> JsonBool(True)."""
        ast = parse_json("true")
        result = from_ast(ast)
        assert isinstance(result, JsonBool)
        assert result.value is True

    # -- Test 9: False --
    def test_false(self) -> None:
        """parse 'false' --> AST --> from_ast --> JsonBool(False)."""
        ast = parse_json("false")
        result = from_ast(ast)
        assert isinstance(result, JsonBool)
        assert result.value is False

    # -- Test 10: Null --
    def test_null(self) -> None:
        """parse 'null' --> AST --> from_ast --> JsonNull."""
        ast = parse_json("null")
        result = from_ast(ast)
        assert isinstance(result, JsonNull)

    # -- Test 11: Simple object --
    def test_simple_object(self) -> None:
        """parse '{"a": 1}' --> JsonObject with one pair."""
        ast = parse_json('{"a": 1}')
        result = from_ast(ast)
        assert isinstance(result, JsonObject)
        assert len(result.pairs) == 1
        assert "a" in result.pairs
        assert result.pairs["a"] == JsonNumber(1)

    # -- Test 12: Multi-key object --
    def test_multi_key_object(self) -> None:
        """parse '{"a": 1, "b": 2}' --> JsonObject with two pairs."""
        ast = parse_json('{"a": 1, "b": 2}')
        result = from_ast(ast)
        assert isinstance(result, JsonObject)
        assert len(result.pairs) == 2
        assert result.pairs["a"] == JsonNumber(1)
        assert result.pairs["b"] == JsonNumber(2)

    # -- Test 13: Simple array --
    def test_simple_array(self) -> None:
        """parse '[1, 2, 3]' --> JsonArray with three JsonNumber elements."""
        ast = parse_json("[1, 2, 3]")
        result = from_ast(ast)
        assert isinstance(result, JsonArray)
        assert len(result.elements) == 3
        assert result.elements[0] == JsonNumber(1)
        assert result.elements[1] == JsonNumber(2)
        assert result.elements[2] == JsonNumber(3)

    # -- Test 14: Mixed array --
    def test_mixed_array(self) -> None:
        """parse '[1, "two", true, null]' --> mixed types."""
        ast = parse_json('[1, "two", true, null]')
        result = from_ast(ast)
        assert isinstance(result, JsonArray)
        assert len(result.elements) == 4
        assert result.elements[0] == JsonNumber(1)
        assert result.elements[1] == JsonString("two")
        assert result.elements[2] == JsonBool(True)
        assert result.elements[3] == JsonNull()

    # -- Test 15: Nested object --
    def test_nested_object(self) -> None:
        """parse '{"a": {"b": 1}}' --> JsonObject containing JsonObject."""
        ast = parse_json('{"a": {"b": 1}}')
        result = from_ast(ast)
        assert isinstance(result, JsonObject)
        inner = result.pairs["a"]
        assert isinstance(inner, JsonObject)
        assert inner.pairs["b"] == JsonNumber(1)

    # -- Test 16: Nested array --
    def test_nested_array(self) -> None:
        """parse '[[1, 2], [3, 4]]' --> JsonArray containing JsonArrays."""
        ast = parse_json("[[1, 2], [3, 4]]")
        result = from_ast(ast)
        assert isinstance(result, JsonArray)
        assert len(result.elements) == 2
        assert isinstance(result.elements[0], JsonArray)
        assert isinstance(result.elements[1], JsonArray)
        assert result.elements[0].elements == [JsonNumber(1), JsonNumber(2)]
        assert result.elements[1].elements == [JsonNumber(3), JsonNumber(4)]

    # -- Test 17: Complex nested --
    def test_complex_nested(self) -> None:
        """parse '{"users": [{"name": "Alice"}]}' --> deep nesting."""
        ast = parse_json('{"users": [{"name": "Alice"}]}')
        result = from_ast(ast)
        assert isinstance(result, JsonObject)

        users = result.pairs["users"]
        assert isinstance(users, JsonArray)
        assert len(users.elements) == 1

        user = users.elements[0]
        assert isinstance(user, JsonObject)
        assert user.pairs["name"] == JsonString("Alice")

    # -- Test 18: String with escapes --
    def test_string_with_escapes(self) -> None:
        r"""parse '"hello\nworld"' --> JsonString with actual newline."""
        ast = parse_json('"hello\\nworld"')
        result = from_ast(ast)
        assert isinstance(result, JsonString)
        assert result.value == "hello\nworld"

    # -- Test 19: Empty string --
    def test_empty_string(self) -> None:
        """parse '""' --> JsonString("")."""
        ast = parse_json('""')
        result = from_ast(ast)
        assert isinstance(result, JsonString)
        assert result.value == ""

    # -- Test 20: Zero --
    def test_zero(self) -> None:
        """parse '0' --> JsonNumber(0) as int."""
        ast = parse_json("0")
        result = from_ast(ast)
        assert isinstance(result, JsonNumber)
        assert result.value == 0
        assert isinstance(result.value, int)


# ===========================================================================
# Section 2: to_native() tests (spec tests 21-28)
# ===========================================================================


class TestToNative:
    """Tests for converting JsonValue objects to native Python types."""

    # -- Test 21: JsonObject to dict --
    def test_object_to_dict(self) -> None:
        """JsonObject({"a": JsonNumber(1)}) --> {"a": 1}."""
        obj = JsonObject({"a": JsonNumber(1)})
        result = to_native(obj)
        assert result == {"a": 1}
        assert isinstance(result, dict)

    # -- Test 22: JsonArray to list --
    def test_array_to_list(self) -> None:
        """JsonArray([JsonNumber(1), JsonNumber(2)]) --> [1, 2]."""
        arr = JsonArray([JsonNumber(1), JsonNumber(2)])
        result = to_native(arr)
        assert result == [1, 2]
        assert isinstance(result, list)

    # -- Test 23: JsonString to str --
    def test_string_to_str(self) -> None:
        """JsonString("hello") --> "hello"."""
        result = to_native(JsonString("hello"))
        assert result == "hello"
        assert isinstance(result, str)

    # -- Test 24: JsonNumber int to int --
    def test_number_int(self) -> None:
        """JsonNumber(42) --> 42."""
        result = to_native(JsonNumber(42))
        assert result == 42
        assert isinstance(result, int)

    # -- Test 25: JsonNumber float to float --
    def test_number_float(self) -> None:
        """JsonNumber(3.14) --> 3.14."""
        result = to_native(JsonNumber(3.14))
        assert result == pytest.approx(3.14)
        assert isinstance(result, float)

    # -- Test 26: JsonBool to bool --
    def test_bool_to_bool(self) -> None:
        """JsonBool(True) --> True, JsonBool(False) --> False."""
        assert to_native(JsonBool(True)) is True
        assert to_native(JsonBool(False)) is False

    # -- Test 27: JsonNull to None --
    def test_null_to_none(self) -> None:
        """JsonNull() --> None."""
        result = to_native(JsonNull())
        assert result is None

    # -- Test 28: Nested to_native --
    def test_nested_to_native(self) -> None:
        """Deeply nested JsonValue --> deeply nested native types."""
        value = JsonObject({
            "name": JsonString("Alice"),
            "scores": JsonArray([JsonNumber(100), JsonNumber(95)]),
            "active": JsonBool(True),
            "meta": JsonObject({
                "version": JsonNumber(2),
                "tags": JsonArray([JsonString("admin"), JsonNull()]),
            }),
        })
        result = to_native(value)
        expected = {
            "name": "Alice",
            "scores": [100, 95],
            "active": True,
            "meta": {
                "version": 2,
                "tags": ["admin", None],
            },
        }
        assert result == expected

    def test_to_native_unknown_type_raises(self) -> None:
        """Passing a non-JsonValue raises JsonValueError."""
        with pytest.raises(JsonValueError, match="Cannot convert"):
            to_native("not a JsonValue")  # type: ignore[arg-type]


# ===========================================================================
# Section 3: from_native() tests (spec tests 29-38)
# ===========================================================================


class TestFromNative:
    """Tests for converting native Python types to JsonValue objects."""

    # -- Test 29: dict to JsonObject --
    def test_dict_to_object(self) -> None:
        """{"a": 1} --> JsonObject({"a": JsonNumber(1)})."""
        result = from_native({"a": 1})
        assert isinstance(result, JsonObject)
        assert result.pairs == {"a": JsonNumber(1)}

    # -- Test 30: list to JsonArray --
    def test_list_to_array(self) -> None:
        """[1, 2] --> JsonArray([JsonNumber(1), JsonNumber(2)])."""
        result = from_native([1, 2])
        assert isinstance(result, JsonArray)
        assert result.elements == [JsonNumber(1), JsonNumber(2)]

    # -- Test 31: str to JsonString --
    def test_str_to_string(self) -> None:
        """\"hello\" --> JsonString("hello")."""
        result = from_native("hello")
        assert isinstance(result, JsonString)
        assert result.value == "hello"

    # -- Test 32: int to JsonNumber --
    def test_int_to_number(self) -> None:
        """42 --> JsonNumber(42)."""
        result = from_native(42)
        assert isinstance(result, JsonNumber)
        assert result.value == 42
        assert isinstance(result.value, int)

    # -- Test 33: float to JsonNumber --
    def test_float_to_number(self) -> None:
        """3.14 --> JsonNumber(3.14)."""
        result = from_native(3.14)
        assert isinstance(result, JsonNumber)
        assert result.value == pytest.approx(3.14)
        assert isinstance(result.value, float)

    # -- Test 34: bool to JsonBool --
    def test_bool_to_bool(self) -> None:
        """True --> JsonBool(True), False --> JsonBool(False)."""
        assert from_native(True) == JsonBool(True)
        assert from_native(False) == JsonBool(False)

    # -- Test 35: None to JsonNull --
    def test_none_to_null(self) -> None:
        """None --> JsonNull()."""
        result = from_native(None)
        assert isinstance(result, JsonNull)

    # -- Test 36: Nested from_native --
    def test_nested_from_native(self) -> None:
        """Deeply nested native --> deeply nested JsonValue."""
        native = {
            "users": [
                {"name": "Alice", "active": True},
                {"name": "Bob", "active": False},
            ],
            "count": 2,
            "meta": None,
        }
        result = from_native(native)
        assert isinstance(result, JsonObject)
        users = result.pairs["users"]
        assert isinstance(users, JsonArray)
        assert len(users.elements) == 2
        alice = users.elements[0]
        assert isinstance(alice, JsonObject)
        assert alice.pairs["name"] == JsonString("Alice")
        assert alice.pairs["active"] == JsonBool(True)
        assert result.pairs["count"] == JsonNumber(2)
        assert isinstance(result.pairs["meta"], JsonNull)

    # -- Test 37: Non-string key error --
    def test_non_string_key_raises(self) -> None:
        """{1: "val"} --> raise JsonValueError."""
        with pytest.raises(JsonValueError, match="keys must be strings"):
            from_native({1: "val"})  # type: ignore[dict-item]

    # -- Test 38: Non-JSON type error --
    def test_non_json_type_raises(self) -> None:
        """A set is not JSON-compatible --> raise JsonValueError."""
        with pytest.raises(JsonValueError, match="Cannot convert"):
            from_native({1, 2, 3})  # type: ignore[arg-type]

    def test_bool_not_converted_to_int(self) -> None:
        """Booleans must become JsonBool, not JsonNumber.

        In Python, bool is a subclass of int: isinstance(True, int) is True.
        This test verifies we check for bool BEFORE int.
        """
        result = from_native(True)
        assert isinstance(result, JsonBool)
        assert not isinstance(result, JsonNumber)

    def test_function_raises(self) -> None:
        """A function is not JSON-compatible."""
        with pytest.raises(JsonValueError, match="Cannot convert"):
            from_native(lambda x: x)  # type: ignore[arg-type]


# ===========================================================================
# Section 4: parse() and parse_native() tests (spec tests 39-42)
# ===========================================================================


class TestParseConvenience:
    """Tests for the convenience parse() and parse_native() functions."""

    # -- Test 39: parse returns JsonValue --
    def test_parse_returns_json_value(self) -> None:
        """parse('{"a": 1}') returns a JsonObject."""
        result = parse('{"a": 1}')
        assert isinstance(result, JsonObject)
        assert result.pairs["a"] == JsonNumber(1)

    # -- Test 40: parse_native returns native --
    def test_parse_native_returns_native(self) -> None:
        """parse_native('{"a": 1}') returns {"a": 1}."""
        result = parse_native('{"a": 1}')
        assert result == {"a": 1}

    # -- Test 41: parse invalid JSON --
    def test_parse_invalid_json(self) -> None:
        """parse('not json') raises JsonValueError."""
        with pytest.raises(JsonValueError):
            parse("not json")

    # -- Test 42: parse_native invalid JSON --
    def test_parse_native_invalid_json(self) -> None:
        """parse_native('{') raises JsonValueError."""
        with pytest.raises(JsonValueError):
            parse_native("{")

    def test_parse_string(self) -> None:
        """parse('"hello"') returns JsonString."""
        result = parse('"hello"')
        assert result == JsonString("hello")

    def test_parse_number(self) -> None:
        """parse('42') returns JsonNumber."""
        result = parse("42")
        assert result == JsonNumber(42)

    def test_parse_true(self) -> None:
        """parse('true') returns JsonBool(True)."""
        result = parse("true")
        assert result == JsonBool(True)

    def test_parse_null(self) -> None:
        """parse('null') returns JsonNull."""
        result = parse("null")
        assert isinstance(result, JsonNull)

    def test_parse_native_array(self) -> None:
        """parse_native('[1, 2, 3]') returns [1, 2, 3]."""
        result = parse_native("[1, 2, 3]")
        assert result == [1, 2, 3]

    def test_parse_native_nested(self) -> None:
        """parse_native handles nested structures."""
        result = parse_native('{"users": [{"name": "Alice"}]}')
        assert result == {"users": [{"name": "Alice"}]}


# ===========================================================================
# Section 5: Round-trip tests (spec tests 43-44)
# ===========================================================================


class TestRoundTrip:
    """Tests verifying that data survives from_native --> to_native round trips."""

    # -- Test 43: Simple round-trip --
    def test_simple_round_trip(self) -> None:
        """value --> from_native --> to_native --> value (match)."""
        original = {"name": "Alice", "age": 30, "active": True}
        json_val = from_native(original)
        result = to_native(json_val)
        assert result == original

    # -- Test 44: Complex nested round-trip --
    def test_complex_round_trip(self) -> None:
        """Complex nested structure survives from_native --> to_native."""
        original = {
            "users": [
                {
                    "name": "Alice",
                    "age": 30,
                    "scores": [100, 95.5, 88],
                    "active": True,
                    "notes": None,
                },
                {
                    "name": "Bob",
                    "age": 25,
                    "scores": [],
                    "active": False,
                    "notes": "new hire",
                },
            ],
            "metadata": {
                "version": 2,
                "tags": ["admin", "user"],
            },
            "empty_object": {},
            "empty_array": [],
        }
        json_val = from_native(original)
        result = to_native(json_val)
        assert result == original

    def test_round_trip_via_parse(self) -> None:
        """JSON text --> parse --> to_native --> from_native --> to_native."""
        text = '{"a": 1, "b": [2, 3.14, true, false, null], "c": {"d": "hello"}}'
        native1 = parse_native(text)
        json_val = from_native(native1)
        native2 = to_native(json_val)
        assert native1 == native2

    def test_round_trip_preserves_int_float(self) -> None:
        """Integers stay as int, floats stay as float through round-trip."""
        original = {"i": 42, "f": 3.14, "z": 0}
        json_val = from_native(original)
        result = to_native(json_val)
        assert isinstance(result["i"], int)  # type: ignore[index]
        assert isinstance(result["f"], float)  # type: ignore[index]
        assert isinstance(result["z"], int)  # type: ignore[index]

    def test_round_trip_empty_containers(self) -> None:
        """Empty objects and arrays survive round-trip."""
        for original in [{}, []]:
            json_val = from_native(original)
            result = to_native(json_val)
            assert result == original


# ===========================================================================
# Edge case and error tests (for coverage)
# ===========================================================================


class TestEdgeCases:
    """Additional tests for edge cases and error paths."""

    def test_from_ast_with_unexpected_type(self) -> None:
        """Passing a non-ASTNode/non-Token raises JsonValueError."""
        with pytest.raises(JsonValueError, match="Expected ASTNode or Token"):
            from_ast("not a node")  # type: ignore[arg-type]

    def test_json_value_base_class(self) -> None:
        """JsonValue base class can be instantiated (it's a dataclass)."""
        base = JsonValue()
        assert isinstance(base, JsonValue)

    def test_json_object_default(self) -> None:
        """JsonObject() creates an empty object."""
        obj = JsonObject()
        assert obj.pairs == {}

    def test_json_array_default(self) -> None:
        """JsonArray() creates an empty array."""
        arr = JsonArray()
        assert arr.elements == []

    def test_json_string_default(self) -> None:
        """JsonString() creates an empty string."""
        s = JsonString()
        assert s.value == ""

    def test_json_number_default(self) -> None:
        """JsonNumber() creates zero."""
        n = JsonNumber()
        assert n.value == 0

    def test_json_bool_default(self) -> None:
        """JsonBool() defaults to False."""
        b = JsonBool()
        assert b.value is False

    def test_equality(self) -> None:
        """JsonValue subclasses compare by value."""
        assert JsonString("hello") == JsonString("hello")
        assert JsonNumber(42) == JsonNumber(42)
        assert JsonBool(True) == JsonBool(True)
        assert JsonNull() == JsonNull()
        assert JsonString("a") != JsonString("b")

    def test_parse_negative_float(self) -> None:
        """parse('-0.5') --> JsonNumber(-0.5)."""
        result = parse("-0.5")
        assert isinstance(result, JsonNumber)
        assert result.value == pytest.approx(-0.5)
        assert isinstance(result.value, float)

    def test_parse_scientific_notation(self) -> None:
        """parse('2.5E-3') --> JsonNumber(0.0025)."""
        result = parse("2.5E-3")
        assert isinstance(result, JsonNumber)
        assert result.value == pytest.approx(0.0025)

    def test_large_nested_structure(self) -> None:
        """parse a deeply nested JSON structure."""
        text = '{"a": {"b": {"c": {"d": [1, 2, {"e": true}]}}}}'
        result = parse_native(text)
        assert result == {"a": {"b": {"c": {"d": [1, 2, {"e": True}]}}}}

    def test_json_value_error_message(self) -> None:
        """JsonValueError carries a descriptive message."""
        err = JsonValueError("test error")
        assert str(err) == "test error"

    def test_object_with_all_value_types(self) -> None:
        """An object containing every JSON value type."""
        text = (
            '{"str": "hello", "num": 42, "float": 3.14, '
            '"bool_t": true, "bool_f": false, "null": null, '
            '"arr": [1], "obj": {"x": 1}}'
        )
        result = parse(text)
        assert isinstance(result, JsonObject)
        assert result.pairs["str"] == JsonString("hello")
        assert result.pairs["num"] == JsonNumber(42)
        assert result.pairs["float"] == JsonNumber(3.14)
        assert result.pairs["bool_t"] == JsonBool(True)
        assert result.pairs["bool_f"] == JsonBool(False)
        assert isinstance(result.pairs["null"], JsonNull)
        assert isinstance(result.pairs["arr"], JsonArray)
        assert isinstance(result.pairs["obj"], JsonObject)

    def test_duplicate_keys_last_wins(self) -> None:
        """When an object has duplicate keys, the last value wins."""
        text = '{"a": 1, "a": 2}'
        result = parse(text)
        assert isinstance(result, JsonObject)
        assert result.pairs["a"] == JsonNumber(2)

    def test_from_native_nested_list(self) -> None:
        """Nested lists convert correctly."""
        result = from_native([[1, 2], [3, 4]])
        assert isinstance(result, JsonArray)
        assert len(result.elements) == 2
        assert isinstance(result.elements[0], JsonArray)
        assert result.elements[0].elements == [JsonNumber(1), JsonNumber(2)]


# ===========================================================================
# Synthetic AST tests (for error/edge paths not reachable via real parser)
# ===========================================================================


class TestSyntheticAstErrors:
    """Tests using hand-constructed ASTNode/Token objects to exercise
    error paths that the real parser never produces.

    These are needed for coverage of defensive error handling in from_ast().
    """

    def test_unexpected_token_type(self) -> None:
        """Passing a structural token (LBRACE) to from_ast raises error."""
        token = Token("LBRACE", "{", 1, 1)
        with pytest.raises(JsonValueError, match="Unexpected token type"):
            from_ast(token)

    def test_unknown_ast_rule(self) -> None:
        """An ASTNode with an unknown rule_name raises error."""
        node = ASTNode("unknown_rule", [])
        with pytest.raises(JsonValueError, match="Unknown AST rule"):
            from_ast(node)

    def test_value_node_no_meaningful_child(self) -> None:
        """A value node with only structural tokens raises error."""
        node = ASTNode("value", [Token("COMMA", ",", 1, 1)])
        with pytest.raises(JsonValueError, match="no meaningful child"):
            from_ast(node)

    def test_pair_node_no_key(self) -> None:
        """A pair node missing the STRING key raises error."""
        value_node = ASTNode("value", [Token("NUMBER", "1", 1, 1)])
        pair = ASTNode("pair", [Token("COLON", ":", 1, 1), value_node])
        with pytest.raises(JsonValueError, match="no STRING key"):
            from_ast(ASTNode("object", [pair]))

    def test_pair_node_no_value(self) -> None:
        """A pair node missing the value raises error."""
        pair = ASTNode("pair", [Token("STRING", "key", 1, 1)])
        with pytest.raises(JsonValueError, match="no value"):
            from_ast(ASTNode("object", [pair]))

    def test_from_ast_pair_directly(self) -> None:
        """Calling from_ast on a pair node returns the value part."""
        value_node = ASTNode("value", [Token("NUMBER", "42", 1, 1)])
        pair = ASTNode("pair", [
            Token("STRING", "key", 1, 1),
            Token("COLON", ":", 1, 1),
            value_node,
        ])
        result = from_ast(pair)
        assert isinstance(result, JsonNumber)
        assert result.value == 42

    def test_array_with_direct_token_children(self) -> None:
        """Array node with direct value tokens (not wrapped in value nodes)."""
        arr = ASTNode("array", [
            Token("LBRACKET", "[", 1, 1),
            Token("NUMBER", "1", 1, 2),
            Token("COMMA", ",", 1, 3),
            Token("NUMBER", "2", 1, 4),
            Token("RBRACKET", "]", 1, 5),
        ])
        result = from_ast(arr)
        assert isinstance(result, JsonArray)
        assert len(result.elements) == 2
        assert result.elements[0] == JsonNumber(1)
        assert result.elements[1] == JsonNumber(2)

    def test_from_ast_token_directly(self) -> None:
        """Calling from_ast on a Token directly works for value tokens."""
        assert from_ast(Token("STRING", "hello", 1, 1)) == JsonString("hello")
        assert from_ast(Token("NUMBER", "42", 1, 1)) == JsonNumber(42)
        assert from_ast(Token("TRUE", "true", 1, 1)) == JsonBool(True)
        assert from_ast(Token("FALSE", "false", 1, 1)) == JsonBool(False)
        assert isinstance(from_ast(Token("NULL", "null", 1, 1)), JsonNull)
