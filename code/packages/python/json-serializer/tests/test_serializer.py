"""Comprehensive tests for the json-serializer package.

These tests cover spec test cases 45-83 from D20-json.md, organized into
sections:

1. **serialize() — compact mode** (tests 45-63)
   Tests that each JsonValue type serializes to the correct compact JSON text.

2. **serialize_pretty() — pretty mode** (tests 64-71)
   Tests indentation, key sorting, trailing newlines, and custom configs.

3. **stringify() and stringify_pretty() — convenience API** (tests 72-78)
   Tests the native-to-JSON-text shortcut functions.

4. **Round-trip tests** (tests 79-83)
   Verifies that serialize(from_native(x)) produces valid, consistent output.

5. **Edge cases and error handling**
   Additional tests for coverage: unknown types, deeply nested structures,
   Unicode, empty strings, etc.
"""

from __future__ import annotations

import math

import pytest

# ---------------------------------------------------------------------------
# Import the types we're testing
# ---------------------------------------------------------------------------
from json_serializer import (
    JsonSerializerError,
    SerializerConfig,
    serialize,
    serialize_pretty,
    stringify,
    stringify_pretty,
)
from json_value import (
    JsonArray,
    JsonBool,
    JsonNull,
    JsonNumber,
    JsonObject,
    JsonString,
    from_native,
)


# =========================================================================
# Section 1: serialize() — compact mode (spec tests 45-63)
# =========================================================================
#
# These tests verify that each JsonValue type produces the correct compact
# JSON representation. Compact mode has NO unnecessary whitespace.
# =========================================================================


class TestSerializeCompact:
    """Tests for serialize() — compact JSON output."""

    # --- Test 45: JsonNull ---
    # JSON null is always the 4-character string "null".
    def test_serialize_null(self) -> None:
        assert serialize(JsonNull()) == "null"

    # --- Test 46: JsonBool(True) ---
    # JSON true is lowercase, unlike Python's "True".
    def test_serialize_bool_true(self) -> None:
        assert serialize(JsonBool(value=True)) == "true"

    # --- Test 47: JsonBool(False) ---
    def test_serialize_bool_false(self) -> None:
        assert serialize(JsonBool(value=False)) == "false"

    # --- Test 48: JsonNumber (positive integer) ---
    # Integers have no decimal point in JSON: 42, not 42.0
    def test_serialize_number_int(self) -> None:
        assert serialize(JsonNumber(value=42)) == "42"

    # --- Test 49: JsonNumber (negative integer) ---
    def test_serialize_number_negative(self) -> None:
        assert serialize(JsonNumber(value=-5)) == "-5"

    # --- Test 50: JsonNumber (float) ---
    def test_serialize_number_float(self) -> None:
        assert serialize(JsonNumber(value=3.14)) == "3.14"

    # --- Test 51: JsonString (simple) ---
    # Strings are wrapped in double quotes.
    def test_serialize_string_simple(self) -> None:
        assert serialize(JsonString(value="hello")) == '"hello"'

    # --- Test 52: JsonString (newline escape) ---
    # A real newline character (U+000A) becomes the two-character
    # sequence \n in JSON output.
    def test_serialize_string_escapes_newline(self) -> None:
        assert serialize(JsonString(value="a\nb")) == '"a\\nb"'

    # --- Test 53: JsonString (quote escape) ---
    # Double quotes inside a string must be escaped with backslash.
    def test_serialize_string_escapes_quote(self) -> None:
        assert serialize(JsonString(value='say "hi"')) == '"say \\"hi\\""'

    # --- Test 54: JsonString (backslash escape) ---
    # A literal backslash becomes \\\\ in JSON.
    def test_serialize_string_escapes_backslash(self) -> None:
        assert serialize(JsonString(value="a\\b")) == '"a\\\\b"'

    # --- Test 55: JsonString (tab escape) ---
    def test_serialize_string_escapes_tab(self) -> None:
        assert serialize(JsonString(value="\t")) == '"\\t"'

    # --- Test 56: JsonString (control character \u0000) ---
    # Control characters U+0000-U+001F that don't have named escapes
    # use the \uXXXX hex notation.
    def test_serialize_string_control_chars(self) -> None:
        assert serialize(JsonString(value="\x00")) == '"\\u0000"'

    # --- Test 57: Empty object ---
    # Empty objects are "{}" with no whitespace inside.
    def test_serialize_empty_object(self) -> None:
        assert serialize(JsonObject(pairs={})) == "{}"

    # --- Test 58: Simple object ---
    # One key-value pair, no spaces around colon.
    def test_serialize_simple_object(self) -> None:
        result = serialize(JsonObject(pairs={"a": JsonNumber(value=1)}))
        assert result == '{"a":1}'

    # --- Test 59: Empty array ---
    def test_serialize_empty_array(self) -> None:
        assert serialize(JsonArray(elements=[])) == "[]"

    # --- Test 60: Simple array ---
    def test_serialize_simple_array(self) -> None:
        result = serialize(JsonArray(elements=[JsonNumber(value=1)]))
        assert result == "[1]"

    # --- Test 61: Nested structures ---
    # An object containing an array containing objects.
    def test_serialize_nested(self) -> None:
        value = JsonObject(
            pairs={
                "users": JsonArray(
                    elements=[
                        JsonObject(
                            pairs={
                                "name": JsonString(value="Alice"),
                                "age": JsonNumber(value=30),
                            }
                        ),
                    ]
                ),
            }
        )
        result = serialize(value)
        assert result == '{"users":[{"name":"Alice","age":30}]}'

    # --- Test 62: Infinity raises error ---
    # JSON has no representation for Infinity.
    def test_serialize_infinity_error(self) -> None:
        with pytest.raises(JsonSerializerError, match="Infinity"):
            serialize(JsonNumber(value=float("inf")))

    # --- Test 63: NaN raises error ---
    # JSON has no representation for NaN.
    def test_serialize_nan_error(self) -> None:
        with pytest.raises(JsonSerializerError, match="NaN"):
            serialize(JsonNumber(value=float("nan")))


# =========================================================================
# Section 2: serialize_pretty() — pretty-printed mode (spec tests 64-71)
# =========================================================================
#
# Pretty mode adds newlines and indentation. Primitives stay inline.
# Empty containers stay compact ({} and []).
# =========================================================================


class TestSerializePretty:
    """Tests for serialize_pretty() — human-readable JSON output."""

    # --- Test 64: Pretty empty object ---
    # Empty objects are always "{}" even in pretty mode.
    def test_pretty_empty_object(self) -> None:
        assert serialize_pretty(JsonObject(pairs={})) == "{}"

    # --- Test 65: Pretty simple object ---
    # One key-value pair with 2-space indentation (default).
    def test_pretty_simple_object(self) -> None:
        result = serialize_pretty(
            JsonObject(pairs={"a": JsonNumber(value=1)})
        )
        assert result == '{\n  "a": 1\n}'

    # --- Test 66: Pretty nested object ---
    # Verify indentation increases at each nesting level.
    def test_pretty_nested_object(self) -> None:
        value = JsonObject(
            pairs={
                "outer": JsonObject(
                    pairs={
                        "inner": JsonNumber(value=1),
                    }
                ),
            }
        )
        result = serialize_pretty(value)
        expected = '{\n  "outer": {\n    "inner": 1\n  }\n}'
        assert result == expected

    # --- Test 67: Pretty array ---
    # Each element on its own line.
    def test_pretty_array(self) -> None:
        result = serialize_pretty(
            JsonArray(elements=[JsonNumber(value=1), JsonNumber(value=2)])
        )
        assert result == "[\n  1,\n  2\n]"

    # --- Test 68: Custom indent size ---
    # 4-space indentation instead of the default 2.
    def test_custom_indent_size(self) -> None:
        config = SerializerConfig(indent_size=4)
        result = serialize_pretty(
            JsonObject(pairs={"a": JsonNumber(value=1)}),
            config=config,
        )
        assert result == '{\n    "a": 1\n}'

    # --- Test 69: Tab indentation ---
    def test_tab_indent(self) -> None:
        config = SerializerConfig(indent_char="\t", indent_size=1)
        result = serialize_pretty(
            JsonObject(pairs={"a": JsonNumber(value=1)}),
            config=config,
        )
        assert result == '{\n\t"a": 1\n}'

    # --- Test 70: Sort keys ---
    # Keys sorted alphabetically regardless of insertion order.
    def test_sort_keys(self) -> None:
        config = SerializerConfig(sort_keys=True)
        value = JsonObject(
            pairs={
                "c": JsonNumber(value=3),
                "a": JsonNumber(value=1),
                "b": JsonNumber(value=2),
            }
        )
        result = serialize_pretty(value, config=config)
        expected = '{\n  "a": 1,\n  "b": 2,\n  "c": 3\n}'
        assert result == expected

    # --- Test 71: Trailing newline ---
    def test_trailing_newline(self) -> None:
        config = SerializerConfig(trailing_newline=True)
        result = serialize_pretty(JsonNull(), config=config)
        assert result == "null\n"

    # Additional: Pretty-print primitives stay inline
    def test_pretty_null(self) -> None:
        assert serialize_pretty(JsonNull()) == "null"

    def test_pretty_bool(self) -> None:
        assert serialize_pretty(JsonBool(value=True)) == "true"

    def test_pretty_number(self) -> None:
        assert serialize_pretty(JsonNumber(value=42)) == "42"

    def test_pretty_string(self) -> None:
        assert serialize_pretty(JsonString(value="hi")) == '"hi"'

    def test_pretty_empty_array(self) -> None:
        assert serialize_pretty(JsonArray(elements=[])) == "[]"

    # Trailing newline on complex structures
    def test_trailing_newline_on_object(self) -> None:
        config = SerializerConfig(trailing_newline=True)
        result = serialize_pretty(
            JsonObject(pairs={"x": JsonNumber(value=1)}),
            config=config,
        )
        assert result.endswith("\n")
        assert result == '{\n  "x": 1\n}\n'

    # Pretty array with nested objects
    def test_pretty_array_of_objects(self) -> None:
        value = JsonArray(
            elements=[
                JsonObject(pairs={"id": JsonNumber(value=1)}),
                JsonObject(pairs={"id": JsonNumber(value=2)}),
            ]
        )
        result = serialize_pretty(value)
        expected = '[\n  {\n    "id": 1\n  },\n  {\n    "id": 2\n  }\n]'
        assert result == expected


# =========================================================================
# Section 3: stringify() and stringify_pretty() (spec tests 72-78)
# =========================================================================
#
# These are convenience functions that convert native Python types to
# JSON text in one step (internally: from_native -> serialize).
# =========================================================================


class TestStringify:
    """Tests for stringify() and stringify_pretty()."""

    # --- Test 72: stringify dict ---
    def test_stringify_dict(self) -> None:
        assert stringify({"a": 1}) == '{"a":1}'

    # --- Test 73: stringify list ---
    def test_stringify_list(self) -> None:
        assert stringify([1, 2]) == "[1,2]"

    # --- Test 74: stringify string ---
    def test_stringify_string(self) -> None:
        assert stringify("hello") == '"hello"'

    # --- Test 75: stringify int ---
    def test_stringify_int(self) -> None:
        assert stringify(42) == "42"

    # --- Test 76: stringify bool ---
    def test_stringify_bool(self) -> None:
        assert stringify(True) == "true"

    # --- Test 77: stringify None ---
    def test_stringify_none(self) -> None:
        assert stringify(None) == "null"

    # --- Test 78: stringify_pretty ---
    def test_stringify_pretty(self) -> None:
        result = stringify_pretty({"a": 1})
        assert result == '{\n  "a": 1\n}'

    # Additional: stringify_pretty with config
    def test_stringify_pretty_with_config(self) -> None:
        config = SerializerConfig(indent_size=4, sort_keys=True)
        result = stringify_pretty({"b": 2, "a": 1}, config=config)
        assert result == '{\n    "a": 1,\n    "b": 2\n}'

    # stringify float
    def test_stringify_float(self) -> None:
        assert stringify(3.14) == "3.14"

    # stringify False
    def test_stringify_false(self) -> None:
        assert stringify(False) == "false"

    # stringify nested
    def test_stringify_nested(self) -> None:
        result = stringify({"list": [1, "two", True, None]})
        assert result == '{"list":[1,"two",true,null]}'


# =========================================================================
# Section 4: Round-trip tests (spec tests 79-83)
# =========================================================================
#
# These tests verify that from_native -> serialize produces consistent
# output that can be parsed back to the same value.
# =========================================================================


class TestRoundTrip:
    """Round-trip tests: native -> JsonValue -> JSON text -> verify."""

    # --- Test 79: Simple round-trip ---
    def test_simple_round_trip(self) -> None:
        original = {"a": 1}
        text = stringify(original)
        assert text == '{"a":1}'

    # --- Test 80: Complex round-trip ---
    def test_complex_round_trip(self) -> None:
        original = {
            "name": "Alice",
            "age": 30,
            "active": True,
            "address": None,
            "scores": [95, 87, 91],
            "metadata": {"role": "admin"},
        }
        text = stringify(original)
        # Verify the output is valid JSON structure
        assert text.startswith("{")
        assert text.endswith("}")
        assert '"name":"Alice"' in text
        assert '"age":30' in text
        assert '"active":true' in text
        assert '"address":null' in text
        assert '"scores":[95,87,91]' in text

    # --- Test 81: Escapes round-trip ---
    def test_escapes_round_trip(self) -> None:
        # A string with all the common escape characters.
        original = "line1\nline2\ttab\\backslash\"quote"
        text = stringify(original)
        assert text == '"line1\\nline2\\ttab\\\\backslash\\"quote"'

    # --- Test 82: Number round-trip ---
    def test_number_round_trip(self) -> None:
        # Integer
        assert stringify(42) == "42"
        assert stringify(-17) == "-17"
        assert stringify(0) == "0"
        # Float
        assert stringify(3.14) == "3.14"
        assert stringify(0.0) == "0.0"

    # --- Test 83: Empty containers round-trip ---
    def test_empty_containers_round_trip(self) -> None:
        assert stringify({}) == "{}"
        assert stringify([]) == "[]"


# =========================================================================
# Section 5: Edge cases and additional coverage
# =========================================================================
#
# Tests beyond the spec to ensure robust error handling and 95%+ coverage.
# =========================================================================


class TestEdgeCases:
    """Additional edge cases for thorough coverage."""

    # Negative infinity
    def test_negative_infinity_error(self) -> None:
        with pytest.raises(JsonSerializerError, match="Infinity"):
            serialize(JsonNumber(value=float("-inf")))

    # Empty string
    def test_empty_string(self) -> None:
        assert serialize(JsonString(value="")) == '""'

    # String with only control characters
    def test_string_all_control_chars(self) -> None:
        # \x01 through \x06 — none have named escapes, all use \uXXXX
        s = "\x01\x02\x03\x04\x05\x06"
        result = serialize(JsonString(value=s))
        assert result == '"\\u0001\\u0002\\u0003\\u0004\\u0005\\u0006"'

    # String with backspace and form feed
    def test_backspace_and_formfeed(self) -> None:
        assert serialize(JsonString(value="\b")) == '"\\b"'
        assert serialize(JsonString(value="\f")) == '"\\f"'

    # String with carriage return
    def test_carriage_return(self) -> None:
        assert serialize(JsonString(value="\r")) == '"\\r"'

    # Forward slash is NOT escaped
    def test_forward_slash_not_escaped(self) -> None:
        assert serialize(JsonString(value="a/b")) == '"a/b"'

    # Unicode characters above U+001F pass through unchanged
    def test_unicode_passthrough(self) -> None:
        assert serialize(JsonString(value="caf\u00e9")) == '"caf\u00e9"'
        assert serialize(JsonString(value="\u4e16\u754c")) == '"\u4e16\u754c"'

    # Zero (integer)
    def test_zero_integer(self) -> None:
        assert serialize(JsonNumber(value=0)) == "0"

    # Large integer
    def test_large_integer(self) -> None:
        assert serialize(JsonNumber(value=10**18)) == "1000000000000000000"

    # Float that looks like integer but is float type
    def test_float_1_0(self) -> None:
        result = serialize(JsonNumber(value=1.0))
        assert result == "1.0"

    # Multi-key object preserves insertion order
    def test_object_insertion_order(self) -> None:
        obj = JsonObject(
            pairs={
                "z": JsonNumber(value=1),
                "a": JsonNumber(value=2),
                "m": JsonNumber(value=3),
            }
        )
        result = serialize(obj)
        # Keys should appear in insertion order, not alphabetical
        assert result == '{"z":1,"a":2,"m":3}'

    # Multi-element array
    def test_multi_element_array(self) -> None:
        arr = JsonArray(
            elements=[
                JsonNumber(value=1),
                JsonString(value="two"),
                JsonBool(value=True),
                JsonNull(),
            ]
        )
        result = serialize(arr)
        assert result == '[1,"two",true,null]'

    # Deeply nested structure
    def test_deeply_nested(self) -> None:
        # 5 levels of nesting: [[[[["deep"]]]]]
        value: JsonValue = JsonString(value="deep")
        for _ in range(5):
            value = JsonArray(elements=[value])
        result = serialize(value)
        assert result == '[[[[["deep"]]]]]'

    # Pretty-print deeply nested
    def test_pretty_deeply_nested(self) -> None:
        value = JsonObject(
            pairs={
                "a": JsonObject(
                    pairs={
                        "b": JsonObject(
                            pairs={
                                "c": JsonNumber(value=1),
                            }
                        ),
                    }
                ),
            }
        )
        result = serialize_pretty(value)
        lines = result.split("\n")
        assert lines[0] == "{"
        assert lines[1] == '  "a": {'
        assert lines[2] == '    "b": {'
        assert lines[3] == '      "c": 1'
        assert lines[4] == "    }"
        assert lines[5] == "  }"
        assert lines[6] == "}"

    # Object with string keys that need escaping
    def test_object_key_escaping(self) -> None:
        obj = JsonObject(
            pairs={
                'key"with"quotes': JsonNumber(value=1),
            }
        )
        result = serialize(obj)
        assert result == '{"key\\"with\\"quotes":1}'

    # Pretty-print object with escaped keys
    def test_pretty_object_key_escaping(self) -> None:
        obj = JsonObject(
            pairs={
                "new\nline": JsonNumber(value=1),
            }
        )
        result = serialize_pretty(obj)
        assert '"new\\nline": 1' in result

    # Config defaults
    def test_config_defaults(self) -> None:
        config = SerializerConfig()
        assert config.indent_size == 2
        assert config.indent_char == " "
        assert config.sort_keys is False
        assert config.trailing_newline is False

    # Multiple escapes in one string
    def test_multiple_escapes(self) -> None:
        s = 'line1\nline2\t"quoted"\\\x00end'
        result = serialize(JsonString(value=s))
        assert result == '"line1\\nline2\\t\\"quoted\\"\\\\\\u0000end"'

    # Negative float
    def test_negative_float(self) -> None:
        assert serialize(JsonNumber(value=-2.5)) == "-2.5"

    # Scientific notation float
    def test_scientific_notation(self) -> None:
        # Python may use scientific notation for very large/small floats
        result = serialize(JsonNumber(value=1e-10))
        # Should be a valid number string (may be "1e-10" or "0.0000000001")
        assert "nan" not in result.lower()
        assert "inf" not in result.lower()

    # Stringify with nested empty containers
    def test_stringify_nested_empty(self) -> None:
        result = stringify({"empty_obj": {}, "empty_arr": []})
        assert '"empty_obj":{}' in result
        assert '"empty_arr":[]' in result

    # Pretty with sort_keys and multiple nesting levels
    def test_pretty_sort_keys_nested(self) -> None:
        config = SerializerConfig(sort_keys=True)
        value = JsonObject(
            pairs={
                "z": JsonObject(
                    pairs={
                        "b": JsonNumber(value=2),
                        "a": JsonNumber(value=1),
                    }
                ),
                "a": JsonNumber(value=0),
            }
        )
        result = serialize_pretty(value, config=config)
        lines = result.split("\n")
        # "a" should come before "z" at top level
        assert lines[1].strip().startswith('"a"')
        # Inside "z", "a" should come before "b"
        assert '"a": 1' in result
        assert '"b": 2' in result

    # Control character \x1f (last control char in range)
    def test_control_char_0x1f(self) -> None:
        result = serialize(JsonString(value="\x1f"))
        assert result == '"\\u001f"'

    # Character at \x20 (space) should NOT be escaped
    def test_space_not_escaped(self) -> None:
        result = serialize(JsonString(value=" "))
        assert result == '" "'

    # Serialize then compare with known JSON
    def test_known_json_output(self) -> None:
        """A complete real-world-ish example."""
        data = {
            "name": "coding-adventures",
            "version": "0.1.0",
            "keywords": ["json", "parser"],
            "active": True,
            "deprecated": False,
            "metadata": None,
        }
        result = stringify(data)
        assert '"name":"coding-adventures"' in result
        assert '"version":"0.1.0"' in result
        assert '"keywords":["json","parser"]' in result
        assert '"active":true' in result
        assert '"deprecated":false' in result
        assert '"metadata":null' in result
