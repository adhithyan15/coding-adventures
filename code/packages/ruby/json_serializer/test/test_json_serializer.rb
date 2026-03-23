# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for CodingAdventures::JsonSerializer
# ================================================================
#
# These tests verify the serialization of JsonValue objects and
# native Ruby types into JSON text, covering:
#
#   1. serialize()        -- compact JSON from JsonValue
#   2. serialize_pretty() -- pretty JSON with configurable format
#   3. stringify()        -- compact JSON from native types
#   4. stringify_pretty() -- pretty JSON from native types
#   5. String escaping    -- RFC 8259 compliance
#   6. Error handling     -- Infinity, NaN
#   7. Round-trips        -- parse then serialize
#
# ================================================================

class TestSerializeCompact < Minitest::Test
  JS = CodingAdventures::JsonSerializer
  JV = CodingAdventures::JsonValue

  # ==============================================================
  # Primitive Values (compact)
  # ==============================================================

  # Spec test #45: Serialize JsonNull
  def test_serialize_null
    assert_equal "null", JS.serialize(JV::Null.new)
  end

  # Spec test #46: Serialize JsonBool true
  def test_serialize_true
    assert_equal "true", JS.serialize(JV::Boolean.new(value: true))
  end

  # Spec test #47: Serialize JsonBool false
  def test_serialize_false
    assert_equal "false", JS.serialize(JV::Boolean.new(value: false))
  end

  # Spec test #48: Serialize JsonNumber int
  def test_serialize_integer
    assert_equal "42", JS.serialize(JV::Number.new(value: 42))
  end

  # Spec test #49: Serialize JsonNumber negative
  def test_serialize_negative_integer
    assert_equal "-5", JS.serialize(JV::Number.new(value: -5))
  end

  # Spec test #50: Serialize JsonNumber float
  def test_serialize_float
    assert_equal "3.14", JS.serialize(JV::Number.new(value: 3.14))
  end

  # Zero
  def test_serialize_zero
    assert_equal "0", JS.serialize(JV::Number.new(value: 0))
  end

  def test_serialize_float_zero
    assert_equal "0.0", JS.serialize(JV::Number.new(value: 0.0))
  end

  # ==============================================================
  # String Escaping (compact)
  # ==============================================================

  # Spec test #51: Simple string
  def test_serialize_string_simple
    assert_equal '"hello"', JS.serialize(JV::String.new(value: "hello"))
  end

  # Spec test #52: Escape newline
  def test_serialize_string_newline
    assert_equal '"a\\nb"', JS.serialize(JV::String.new(value: "a\nb"))
  end

  # Spec test #53: Escape quote
  def test_serialize_string_quote
    assert_equal '"say \\"hi\\""', JS.serialize(JV::String.new(value: 'say "hi"'))
  end

  # Spec test #54: Escape backslash
  def test_serialize_string_backslash
    assert_equal '"a\\\\b"', JS.serialize(JV::String.new(value: "a\\b"))
  end

  # Spec test #55: Escape tab
  def test_serialize_string_tab
    assert_equal '"\\t"', JS.serialize(JV::String.new(value: "\t"))
  end

  # Spec test #56: Escape control characters
  def test_serialize_string_null_char
    assert_equal '"\\u0000"', JS.serialize(JV::String.new(value: "\x00"))
  end

  # Additional escape sequences
  def test_serialize_string_backspace
    assert_equal '"\\b"', JS.serialize(JV::String.new(value: "\b"))
  end

  def test_serialize_string_form_feed
    assert_equal '"\\f"', JS.serialize(JV::String.new(value: "\f"))
  end

  def test_serialize_string_carriage_return
    assert_equal '"\\r"', JS.serialize(JV::String.new(value: "\r"))
  end

  # Control characters in the U+0001-U+001F range
  def test_serialize_string_control_char_0x01
    assert_equal '"\\u0001"', JS.serialize(JV::String.new(value: "\x01"))
  end

  def test_serialize_string_control_char_0x1f
    assert_equal '"\\u001f"', JS.serialize(JV::String.new(value: "\x1F"))
  end

  # Empty string
  def test_serialize_empty_string
    assert_equal '""', JS.serialize(JV::String.new(value: ""))
  end

  # Forward slash is NOT escaped (per our convention)
  def test_serialize_string_forward_slash
    assert_equal '"a/b"', JS.serialize(JV::String.new(value: "a/b"))
  end

  # Unicode characters (non-ASCII) pass through unescaped
  def test_serialize_string_unicode
    assert_equal "\"caf\u00E9\"", JS.serialize(JV::String.new(value: "caf\u00E9"))
  end

  # ==============================================================
  # Arrays (compact)
  # ==============================================================

  # Spec test #59: Empty array
  def test_serialize_empty_array
    assert_equal "[]", JS.serialize(JV::Array.new)
  end

  # Spec test #60: Simple array
  def test_serialize_simple_array
    arr = JV::Array.new(elements: [JV::Number.new(value: 1)])
    assert_equal "[1]", JS.serialize(arr)
  end

  # Multiple elements with no spaces after commas
  def test_serialize_multi_element_array
    arr = JV::Array.new(elements: [
      JV::Number.new(value: 1),
      JV::Number.new(value: 2),
      JV::Number.new(value: 3)
    ])
    assert_equal "[1,2,3]", JS.serialize(arr)
  end

  # Mixed-type array
  def test_serialize_mixed_array
    arr = JV::Array.new(elements: [
      JV::Number.new(value: 1),
      JV::String.new(value: "two"),
      JV::Boolean.new(value: true),
      JV::Null.new
    ])
    assert_equal '[1,"two",true,null]', JS.serialize(arr)
  end

  # ==============================================================
  # Objects (compact)
  # ==============================================================

  # Spec test #57: Empty object
  def test_serialize_empty_object
    assert_equal "{}", JS.serialize(JV::Object.new)
  end

  # Spec test #58: Simple object
  def test_serialize_simple_object
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    assert_equal '{"a":1}', JS.serialize(obj)
  end

  # Multiple pairs
  def test_serialize_multi_pair_object
    obj = JV::Object.new(pairs: {
      "a" => JV::Number.new(value: 1),
      "b" => JV::Number.new(value: 2)
    })
    assert_equal '{"a":1,"b":2}', JS.serialize(obj)
  end

  # ==============================================================
  # Nested Structures (compact)
  # ==============================================================

  # Spec test #61: Nested structures
  def test_serialize_nested
    obj = JV::Object.new(pairs: {
      "users" => JV::Array.new(elements: [
        JV::Object.new(pairs: {
          "name" => JV::String.new(value: "Alice"),
          "age" => JV::Number.new(value: 30)
        })
      ])
    })
    assert_equal '{"users":[{"name":"Alice","age":30}]}', JS.serialize(obj)
  end

  # ==============================================================
  # Error Cases
  # ==============================================================

  # Spec test #62: Infinity error
  def test_serialize_infinity_raises
    assert_raises(JS::Error) { JS.serialize(JV::Number.new(value: Float::INFINITY)) }
  end

  def test_serialize_negative_infinity_raises
    assert_raises(JS::Error) { JS.serialize(JV::Number.new(value: -Float::INFINITY)) }
  end

  # Spec test #63: NaN error
  def test_serialize_nan_raises
    assert_raises(JS::Error) { JS.serialize(JV::Number.new(value: Float::NAN)) }
  end

  # Unknown type raises error
  def test_serialize_unknown_type_raises
    assert_raises(JS::Error) { JS.serialize("not a json value") }
  end
end

# ================================================================
# Tests for serialize_pretty
# ================================================================

class TestSerializePretty < Minitest::Test
  JS = CodingAdventures::JsonSerializer
  JV = CodingAdventures::JsonValue

  # Spec test #64: Pretty empty object
  def test_pretty_empty_object
    assert_equal "{}", JS.serialize_pretty(JV::Object.new)
  end

  # Spec test #65: Pretty simple object
  def test_pretty_simple_object
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    expected = "{\n  \"a\": 1\n}"
    assert_equal expected, JS.serialize_pretty(obj)
  end

  # Spec test #66: Pretty nested object (indentation increases)
  def test_pretty_nested_object
    obj = JV::Object.new(pairs: {
      "outer" => JV::Object.new(pairs: {
        "inner" => JV::Number.new(value: 42)
      })
    })
    expected = "{\n  \"outer\": {\n    \"inner\": 42\n  }\n}"
    assert_equal expected, JS.serialize_pretty(obj)
  end

  # Spec test #67: Pretty array
  def test_pretty_array
    arr = JV::Array.new(elements: [
      JV::Number.new(value: 1),
      JV::Number.new(value: 2)
    ])
    expected = "[\n  1,\n  2\n]"
    assert_equal expected, JS.serialize_pretty(arr)
  end

  # Pretty empty array
  def test_pretty_empty_array
    assert_equal "[]", JS.serialize_pretty(JV::Array.new)
  end

  # Spec test #68: Custom indent size
  def test_pretty_indent_size_4
    config = JS::SerializerConfig.new(indent_size: 4)
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    expected = "{\n    \"a\": 1\n}"
    assert_equal expected, JS.serialize_pretty(obj, config: config)
  end

  # Spec test #69: Tab indent
  def test_pretty_tab_indent
    config = JS::SerializerConfig.new(indent_char: "\t", indent_size: 1)
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    expected = "{\n\t\"a\": 1\n}"
    assert_equal expected, JS.serialize_pretty(obj, config: config)
  end

  # Spec test #70: Sort keys
  def test_pretty_sort_keys
    config = JS::SerializerConfig.new(sort_keys: true)
    obj = JV::Object.new(pairs: {
      "c" => JV::Number.new(value: 3),
      "a" => JV::Number.new(value: 1),
      "b" => JV::Number.new(value: 2)
    })
    expected = "{\n  \"a\": 1,\n  \"b\": 2,\n  \"c\": 3\n}"
    assert_equal expected, JS.serialize_pretty(obj, config: config)
  end

  # Spec test #71: Trailing newline
  def test_pretty_trailing_newline
    config = JS::SerializerConfig.new(trailing_newline: true)
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    expected = "{\n  \"a\": 1\n}\n"
    assert_equal expected, JS.serialize_pretty(obj, config: config)
  end

  # Primitives in pretty mode are the same as compact
  def test_pretty_null
    assert_equal "null", JS.serialize_pretty(JV::Null.new)
  end

  def test_pretty_boolean
    assert_equal "true", JS.serialize_pretty(JV::Boolean.new(value: true))
  end

  def test_pretty_number
    assert_equal "42", JS.serialize_pretty(JV::Number.new(value: 42))
  end

  def test_pretty_string
    assert_equal '"hello"', JS.serialize_pretty(JV::String.new(value: "hello"))
  end

  # Complex nested pretty-printing
  def test_pretty_complex_nested
    obj = JV::Object.new(pairs: {
      "name" => JV::String.new(value: "Alice"),
      "scores" => JV::Array.new(elements: [
        JV::Number.new(value: 95),
        JV::Number.new(value: 87)
      ])
    })
    expected = "{\n  \"name\": \"Alice\",\n  \"scores\": [\n    95,\n    87\n  ]\n}"
    assert_equal expected, JS.serialize_pretty(obj)
  end
end

# ================================================================
# Tests for stringify and stringify_pretty
# ================================================================

class TestStringify < Minitest::Test
  JS = CodingAdventures::JsonSerializer

  # Spec test #72: stringify dict
  def test_stringify_hash
    assert_equal '{"a":1}', JS.stringify({ "a" => 1 })
  end

  # Spec test #73: stringify array
  def test_stringify_array
    assert_equal "[1,2]", JS.stringify([1, 2])
  end

  # Spec test #74: stringify string
  def test_stringify_string
    assert_equal '"hello"', JS.stringify("hello")
  end

  # Spec test #75: stringify int
  def test_stringify_integer
    assert_equal "42", JS.stringify(42)
  end

  # Spec test #76: stringify bool
  def test_stringify_true
    assert_equal "true", JS.stringify(true)
  end

  def test_stringify_false
    assert_equal "false", JS.stringify(false)
  end

  # Spec test #77: stringify nil
  def test_stringify_nil
    assert_equal "null", JS.stringify(nil)
  end

  # Spec test #78: stringify_pretty
  def test_stringify_pretty
    expected = "{\n  \"a\": 1\n}"
    assert_equal expected, JS.stringify_pretty({ "a" => 1 })
  end

  # stringify float
  def test_stringify_float
    assert_equal "3.14", JS.stringify(3.14)
  end

  # stringify nested
  def test_stringify_nested
    native = { "users" => [{ "name" => "Alice" }] }
    assert_equal '{"users":[{"name":"Alice"}]}', JS.stringify(native)
  end

  # stringify_pretty with config
  def test_stringify_pretty_with_config
    config = JS::SerializerConfig.new(indent_size: 4, sort_keys: true)
    result = JS.stringify_pretty({ "b" => 2, "a" => 1 }, config: config)
    expected = "{\n    \"a\": 1,\n    \"b\": 2\n}"
    assert_equal expected, result
  end
end

# ================================================================
# SerializerConfig Tests
# ================================================================

class TestSerializerConfig < Minitest::Test
  JS = CodingAdventures::JsonSerializer

  def test_default_config
    config = JS::SerializerConfig.new
    assert_equal 2, config.indent_size
    assert_equal " ", config.indent_char
    assert_equal false, config.sort_keys
    assert_equal false, config.trailing_newline
  end

  def test_custom_config
    config = JS::SerializerConfig.new(
      indent_size: 4,
      indent_char: "\t",
      sort_keys: true,
      trailing_newline: true
    )
    assert_equal 4, config.indent_size
    assert_equal "\t", config.indent_char
    assert_equal true, config.sort_keys
    assert_equal true, config.trailing_newline
  end

  def test_indent_for
    config = JS::SerializerConfig.new(indent_size: 2, indent_char: " ")
    assert_equal "", config.indent_for(0)
    assert_equal "  ", config.indent_for(1)
    assert_equal "    ", config.indent_for(2)
  end

  def test_indent_for_tabs
    config = JS::SerializerConfig.new(indent_size: 1, indent_char: "\t")
    assert_equal "", config.indent_for(0)
    assert_equal "\t", config.indent_for(1)
    assert_equal "\t\t", config.indent_for(2)
  end
end

# ================================================================
# Full Round-trip Tests (parse + serialize)
# ================================================================

class TestRoundTrip < Minitest::Test
  JS = CodingAdventures::JsonSerializer
  JV = CodingAdventures::JsonValue

  # Spec test #79: parse then serialize
  def test_parse_then_serialize
    result = JS.stringify(JV.parse_native('{"a":1}'))
    assert_equal '{"a":1}', result
  end

  # Spec test #80: Complex round-trip
  def test_complex_round_trip
    json = '{"users":[{"name":"Alice","age":30},{"name":"Bob","age":25}]}'
    native = JV.parse_native(json)
    result = JS.stringify(native)
    # Re-parse to verify structural equality (key order may differ)
    assert_equal native, JV.parse_native(result)
  end

  # Spec test #81: Escapes round-trip
  def test_escapes_round_trip
    # String with various escape characters
    json = '"hello\\nworld\\ttab\\\\backslash"'
    value = JV.parse(json)
    serialized = JS.serialize(value)
    re_parsed = JV.parse(serialized)
    assert_equal value, re_parsed
  end

  # Spec test #82: Number round-trip
  def test_number_round_trip_integer
    result = JS.stringify(JV.parse_native("42"))
    assert_equal "42", result
  end

  def test_number_round_trip_float
    result = JS.stringify(JV.parse_native("3.14"))
    assert_equal "3.14", result
  end

  def test_number_round_trip_negative
    result = JS.stringify(JV.parse_native("-17"))
    assert_equal "-17", result
  end

  # Spec test #83: Empty containers round-trip
  def test_empty_object_round_trip
    assert_equal "{}", JS.stringify(JV.parse_native("{}"))
  end

  def test_empty_array_round_trip
    assert_equal "[]", JS.stringify(JV.parse_native("[]"))
  end

  # Boolean round-trip
  def test_boolean_round_trip
    assert_equal "true", JS.stringify(JV.parse_native("true"))
    assert_equal "false", JS.stringify(JV.parse_native("false"))
  end

  # Null round-trip
  def test_null_round_trip
    assert_equal "null", JS.stringify(JV.parse_native("null"))
  end

  # Full pipeline: text -> parse -> serialize_pretty -> parse -> check equality
  def test_pretty_round_trip
    original = { "name" => "Alice", "scores" => [95, 87] }
    pretty = JS.stringify_pretty(original)
    re_parsed = JV.parse_native(pretty)
    assert_equal original, re_parsed
  end
end
